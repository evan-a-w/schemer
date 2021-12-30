use crate::types::*;
use std::cell::UnsafeCell;
use std::ops::Deref;
use std::ops::DerefMut;
use std::ptr::{self, NonNull};

#[derive(Clone, Copy, Debug)]
pub enum MarkerFlag {
    Unseen,
    ChildrenNotSeen,
    Seen,
}

#[derive(Clone, Copy, Debug)]
pub enum TakenFlag {
    NotTaken,
    Shared(usize),
    Unique,
}

#[derive(Clone, Copy, Debug)]
pub struct Flags {
    pub marker: MarkerFlag,
    pub taken: TakenFlag,
    pub to_free: bool,
}

#[derive(Debug)]
pub struct GcObj {
    pub data: NonNull<Ponga>,
    pub id: Id,
    pub flags: UnsafeCell<Flags>,
}

impl Clone for GcObj {
    fn clone(&self) -> Self {
        GcObj {
            data: self.data,
            id: self.id,
            flags: UnsafeCell::new(self.get_flags()),
        }
    }
}

pub struct GcRef<'a> {
    inner: &'a Ponga,
    gc_obj: &'a GcObj,
}

pub struct GcRefMut<'a> {
    inner: &'a mut Ponga,
    gc_obj: &'a mut GcObj,
}

impl<'a> GcRefMut<'a> {
    pub fn inner(&mut self) -> &mut Ponga {
        self.inner
    }
}

impl<'a> GcRef<'a> {
    pub fn inner(&self) -> &Ponga {
        self.inner
    }
}

impl<'a> Drop for GcRef<'a> {
    fn drop(&mut self) {
        self.gc_obj.remove_shared();
    }
}

impl<'a> Drop for GcRefMut<'a> {
    fn drop(&mut self) {
        self.gc_obj.flags.get_mut().taken = TakenFlag::NotTaken;
    }
}

impl Deref for GcRef<'_> {
    type Target = Ponga;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl Deref for GcRefMut<'_> {
    type Target = Ponga;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl DerefMut for GcRefMut<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
    }
}

impl GcObj {
    fn add_shared(&self) {
        unsafe {
            let flags = &mut *self.flags.get();
            match flags.taken {
                TakenFlag::NotTaken => flags.taken = TakenFlag::Shared(1),
                TakenFlag::Shared(n) => flags.taken = TakenFlag::Shared(n + 1),
                _ => panic!("Trying to add shared to unique"),
            }
        }
    }

    fn remove_shared(&self) {
        unsafe {
            let flags = &mut *self.flags.get();
            match flags.taken {
                TakenFlag::Shared(n) => {
                    if n == 1 {
                        flags.taken = TakenFlag::NotTaken;
                    } else {
                        flags.taken = TakenFlag::Shared(n - 1);
                    }
                }
                _ => panic!("Trying to dec shared when it is not shared"),
            }
        }
    }

    pub fn mark_seen(&self) {
        unsafe {
            let flags = &mut *self.flags.get();
            flags.marker = MarkerFlag::Seen;
        }
    }

    pub fn mark_unseen(&self) {
        unsafe {
            let flags = &mut *self.flags.get();
            flags.marker = MarkerFlag::Unseen;
        }
    }

    pub fn mark_children_not_seen(&self) {
        unsafe {
            let flags = &mut *self.flags.get();
            flags.marker = MarkerFlag::ChildrenNotSeen;
        }
    }

    pub fn borrow<'a>(&'a self) -> Option<GcRef<'a>> {
        unsafe {
            let flags = &mut *self.flags.get();
            match flags.taken {
                TakenFlag::Unique => None,
                _ => {
                    self.add_shared();
                    Some(GcRef {
                        inner: &*self.data.as_ptr(),
                        gc_obj: self,
                    })
                }
            }
        }
    }

    pub fn to_number_ponga(&self) -> RunRes<Number> {
        self.borrow().unwrap().to_number()
    }

    pub fn borrow_mut<'a>(&'a mut self) -> Option<GcRefMut<'a>> {
        unsafe {
            let flags = &mut *self.flags.get();
            match flags.taken {
                TakenFlag::NotTaken => {
                    flags.taken = TakenFlag::Unique;
                    Some(GcRefMut {
                        inner: &mut *self.data.as_ptr(),
                        gc_obj: self,
                    })
                }
                _ => None,
            }
        }
    }

    pub fn get_flags(&self) -> Flags {
        unsafe { *self.flags.get() }
    }

    pub fn get_marker(&self) -> MarkerFlag {
        unsafe {
            let flags = &*self.flags.get();
            flags.marker
        }
    }

    pub fn get_taken(&self) -> TakenFlag {
        unsafe {
            let flags = &*self.flags.get();
            flags.taken
        }
    }

    pub fn free(&mut self) {
        unsafe {
            Box::from_raw(self.data.as_ptr());
        }
    }
}

impl Drop for GcObj {
    fn drop(&mut self) {
        if self.get_flags().to_free {
            unsafe {
                *Box::from_raw(self.data.as_ptr());
            }
        }
    }
}
