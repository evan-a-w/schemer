use crate::types::*;
use std::cell::UnsafeCell;
use std::ops::Deref;
use std::ops::DerefMut;
use crate::gc::Trace;
use std::ptr::NonNull;

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
pub struct GcObj<T: Trace<T>> {
    pub data: UnsafeCell<NonNull<T>>,
    pub id: Id,
    pub flags: UnsafeCell<Flags>,
}

impl<T: Trace<T>> Clone for GcObj<T> {
    fn clone(&self) -> Self {
        GcObj {
            data: unsafe {
                UnsafeCell::new(NonNull::new_unchecked(
                    self.data.get().as_ref().unwrap().as_ptr()
                ))
            },
            id: self.id,
            flags: UnsafeCell::new(self.get_flags()),
        }
    }
}

pub struct GcRef<'a, T: Trace<T>> {
    gc_obj: &'a GcObj<T>,
}

pub struct GcRefMut<'a, T: Trace<T>> {
    gc_obj: &'a GcObj<T>,
}

impl<'a, T: Trace<T>> GcRefMut<'a, T> {
    pub fn inner(&mut self) -> &mut T {
        unsafe { self.gc_obj.data.get().as_mut().unwrap().as_mut() }
    }
}

impl<'a, T: Trace<T>> GcRef<'a, T> {
    pub fn inner(&self) -> &T {
        unsafe { self.gc_obj.data.get().as_ref().unwrap().as_ref() }
    }
}

impl<'a, T: Trace<T>> Drop for GcRef<'a, T> {
    fn drop(&mut self) {
        self.gc_obj.remove_shared();
    }
}

impl<'a, T: Trace<T>> Drop for GcRefMut<'a, T> {
    fn drop(&mut self) {
        unsafe {
            self.gc_obj.flags.get().as_mut().unwrap().taken = TakenFlag::NotTaken;
        }
    }
}

impl<T: Trace<T>> Deref for GcRef<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.gc_obj.data.get().as_ref().unwrap().as_ref() }
    }
}

impl<T: Trace<T>> Deref for GcObj<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.data.get().as_ref().unwrap().as_ref() }
    }
}

impl<T: Trace<T>> DerefMut for GcObj<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.data.get().as_mut().unwrap().as_mut() }
    }
}

impl<T: Trace<T>> Deref for GcRefMut<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.gc_obj.data.get().as_ref().unwrap().as_ref() }
    }
}

impl<T: Trace<T>> DerefMut for GcRefMut<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.gc_obj.data.get().as_mut().unwrap().as_mut() }
    }
}

impl<T: Trace<T>> GcObj<T> {
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

    pub fn borrow<'a>(&'a self) -> Option<GcRef<'a, T>> {
        unsafe {
            let flags = &mut *self.flags.get();
            match flags.taken {
                TakenFlag::Unique => None,
                _ => {
                    self.add_shared();
                    Some(GcRef {
                        gc_obj: self,
                    })
                }
            }
        }
    }

    pub fn borrow_mut<'a>(&'a self) -> Option<GcRefMut<'a, T>> {
        unsafe {
            let flags = &mut *self.flags.get();
            match flags.taken {
                TakenFlag::NotTaken => {
                    flags.taken = TakenFlag::Unique;
                    Some(GcRefMut {
                        gc_obj: &*(self as *const GcObj<T>),
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

    pub fn free(&mut self) {
        unsafe {
            let _ = *Box::from_raw(self.data.get().as_ref().unwrap().as_ptr());
        }
    }
}

impl<T: Trace<T>> Drop for GcObj<T> {
    fn drop(&mut self) {
        if self.get_flags().to_free {
            unsafe {
                let _ = *Box::from_raw(self.data.get().as_ref().unwrap().as_ptr());
            }
        }
    }
}
