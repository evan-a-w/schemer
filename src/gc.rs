use crate::gc_obj::*;
use crate::runtime::*;
use crate::types::*;
use std::cell::RefCell;
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::ptr::{self, NonNull};
use std::rc::Rc;

pub struct Gc {
    pub ptrs: HashMap<Id, GcObj>,
    pub roots: HashSet<Id>,
    pub max_id: usize,
}

impl Gc {
    pub fn new() -> Gc {
        Gc {
            ptrs: HashMap::new(),
            roots: HashSet::new(),
            max_id: 0,
        }
    }

    pub fn collect_garbage(&mut self) {
        for root in self.roots.iter() {
            self.ptrs.get(root).unwrap().trace(self);
        }

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

    pub fn take_id(&mut self, id: Id) -> Option<Ponga> {
        let obj = self.ptrs.get_mut(&id)?;
        let res = unsafe { *Box::from_raw(obj.data.as_ptr()) };
        self.ptrs.remove(&id);
        Some(res)
    }

    pub fn get_new_id(&mut self) -> Id {
        let id = self.max_id;
        self.max_id += 1;
        id
    }

    pub fn add_obj(&mut self, data: Ponga) -> Id {
        let obj = GcObj::new(self, data);
        let id = obj.id;
        self.ptrs.insert(obj.id, obj);
        id
    }

    pub fn add_obj_with_id(&mut self, data: Ponga, id: Id) {
        let obj = GcObj {
            data: NonNull::new(Box::into_raw(Box::new(data))).unwrap(),
            flags: UnsafeCell::new(Flags {
                marker: MarkerFlag::Unseen,
                taken: TakenFlag::NotTaken,
                to_free: false,
            }),
            id,
        };

        self.ptrs.insert(obj.id, obj);
    }

    pub fn ponga_into_gc_ref(&mut self, data: Ponga) -> Ponga {
        Ponga::Ref(self.add_obj(data))
    }
}

impl GcObj {
    pub fn new(state: &mut Gc, data: Ponga) -> GcObj {
        GcObj {
            data: NonNull::new(Box::into_raw(Box::new(data))).unwrap(),
            flags: UnsafeCell::new(Flags {
                marker: MarkerFlag::Unseen,
                taken: TakenFlag::NotTaken,
                to_free: false,
            }),
            id: state.get_new_id(),
        }
    }
}

pub trait Trace {
    fn trace(&self, gc: &Gc);
}

// Marks as seen and calls trace on children
impl Trace for GcObj {
    fn trace(&self, gc: &Gc) {
        // Probably don't need this variant
        self.mark_children_not_seen();

        self.borrow().unwrap().trace(gc);
        self.mark_seen();
    }
}

// Just calls the true trace on everything that could be a Ref
impl Trace for Ponga {
    fn trace(&self, gc: &Gc) {
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
            Ponga::CFunc(_, id, state) => {
                gc.ptrs.get(id).unwrap().trace(gc);
                for val in state.values() {
                    val.trace(gc);
                }
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

impl Drop for Gc {
    fn drop(&mut self) {
        for obj in self.ptrs.values_mut() {
            obj.free();
        }
    }
}
