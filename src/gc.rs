use crate::gc_obj::*;
use crate::types::*;
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::time::{Duration, Instant};
use std::ptr::NonNull;

pub struct Gc<T: Trace<T>> {
    pub ptrs: HashMap<Id, GcObj<T>>,
    pub max_id: usize,
    pub last_gc: Instant,
    pub gc_duration: Duration,
}

impl<T: Trace<T>> Gc<T> {
    pub fn new() -> Gc<T> {
        Gc {
            ptrs: HashMap::new(),
            max_id: 0,
            last_gc: Instant::now(),
            gc_duration: Duration::from_secs(5),
        }
    }

    pub fn try_collect_garbage(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_gc) > self.gc_duration {
            self.last_gc = now;
            self.collect_garbage();
        }
    }

    pub fn collect_garbage(&mut self) {
        let mut to_delete = vec![];
        for obj in self.ptrs.values() {
            match obj.get_marker() {
                MarkerFlag::Unseen => to_delete.push(obj.id),
                _ => obj.mark_unseen(),
            }
        }

        for id in to_delete {
            let gco = self.ptrs.get_mut(&id).unwrap();
            gco.free();
            self.ptrs.remove(&id);
        }
    }

    pub fn take(&mut self, id: Id) -> Option<T> {
        self.try_collect_garbage();
        let obj = self.ptrs.get_mut(&id)?;
        let res = unsafe { *Box::from_raw(obj.data.get().as_ref().unwrap().as_ptr()) };
        self.ptrs.remove(&id);
        Some(res)
    }

    pub fn get_new_id(&mut self) -> Id {
        let id = self.max_id;
        self.max_id += 1;
        id
    }

    pub fn add(&mut self, data: T) -> Id {
        self.try_collect_garbage();
        let obj = GcObj::new(self, data);
        let id = obj.id;
        self.ptrs.insert(obj.id, obj);
        id
    }

    pub fn add_id(&mut self, data: T, id: Id) {
        self.try_collect_garbage();
        let obj = GcObj {
            data: UnsafeCell::new(
                NonNull::new(Box::into_raw(Box::new(data))).unwrap()
            ),
            flags: UnsafeCell::new(Flags {
                marker: MarkerFlag::Unseen,
                taken: TakenFlag::NotTaken,
                to_free: false,
            }),
            id,
        };

        self.ptrs.insert(obj.id, obj);
    }

    pub fn get(&self, id: Id) -> RunRes<&GcObj<T>> {
        self.ptrs
            .get(&id)
            .ok_or(RuntimeErr::ReferenceError(format!(
                "Reference {} not found",
                id
            )))
    }

    pub fn get_mut(&mut self, id: Id) -> RunRes<&mut GcObj<T>> {
        self.ptrs
            .get_mut(&id)
            .ok_or(RuntimeErr::ReferenceError(format!(
                "Reference {} not found",
                id
            )))
    }
}

impl Gc<Ponga> {
    pub fn ponga_into_gc_ref(&mut self, data: Ponga) -> Ponga {
        Ponga::Ref(self.add(data))
    }
}

impl<T: Trace<T>> GcObj<T> {
    pub fn new(state: &mut Gc<T>, data: T) -> GcObj<T> {
        GcObj {
            data: UnsafeCell::new(
                NonNull::new(Box::into_raw(Box::new(data))).unwrap()
            ),
            flags: UnsafeCell::new(Flags {
                marker: MarkerFlag::Unseen,
                taken: TakenFlag::NotTaken,
                to_free: false,
            }),
            id: state.get_new_id(),
        }
    }
}

pub trait Trace<T: Trace<T>> {
    fn trace(&self, gc: &Gc<T>);
}

// Marks as seen and calls trace on children
impl<T: Trace<T>> Trace<T> for GcObj<T> {
    fn trace(&self, gc: &Gc<T>) {
        // Probably don't need this variant
        self.mark_children_not_seen();

        self.borrow().unwrap().trace(gc);
        self.mark_seen();
    }
}

// Just calls the true trace on everything that could be a Ref
impl Trace<Ponga> for Ponga {
    fn trace(&self, gc: &Gc<Ponga>) {
        match self {
            Ponga::Ref(id) => gc.ptrs.get(id).unwrap().trace(gc),
            Ponga::Array(arr) => {
                for i in arr.iter() {
                    i.trace(gc);
                }
            }
            Ponga::List(l) => {
                for i in l.iter() {
                    i.trace(gc);
                }
            }
            Ponga::Sexpr(arr) => {
                for i in arr.iter() {
                    i.trace(gc);
                }
            }
            Ponga::Object(obj) => {
                for i in obj.values() {
                    i.trace(gc);
                }
            }
            Ponga::CFunc(_, id, stateid) => {
                gc.ptrs.get(id).unwrap().trace(gc);
                gc.ptrs.get(stateid).unwrap().trace(gc);
            }
            Ponga::MFunc(_, id) => {
                gc.ptrs.get(id).unwrap().trace(gc);
            }
            Ponga::Number(_)
            | Ponga::String(_)
            | Ponga::True
            | Ponga::False
            | Ponga::Char(_)
            | Ponga::Symbol(_)
            | Ponga::Identifier(_)
            | Ponga::HFunc(_)
            | Ponga::Null => (),
        }
    }
}

impl<T: Trace<T>> Drop for Gc<T> {
    fn drop(&mut self) {
        for obj in self.ptrs.values_mut() {
            obj.free();
        }
    }
}
