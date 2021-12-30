use crate::types::*;
use crate::runtime::*;
use crate::gc::*;

pub struct TakeObj<'a> {
    pub obj: Ponga,
    pub id: Id,
    pub gc_ref: &'a mut Gc,
}

impl TakeObj<'_> {
    pub fn new<'a>(gc_ref: &'a mut Gc, id: Id) -> TakeObj<'a> {
        TakeObj {
            obj: gc_ref.take_id(id).unwrap(),
            id,
            gc_ref,
        }
    }
}

impl Drop for TakeObj<'_> {
    fn drop(&mut self) {
        self.gc_ref.add_obj_with_id(std::mem::replace(&mut self.obj, Ponga::Null), self.id);
    }
}
