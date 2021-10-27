use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

pub enum Object {
    Symbol(String),
    Num(Number),
    Atom(Atom),
    List(List),
    Exp(Atom, List),
    Env(Dict),
    Func(Func),
    Thonk((GarbObject, Vec<GarbObject>)),
    Array(Vec<GarbObject>),
}

pub struct Func {
    args: usize,
    operations: Vec<GarbObject>,
}

pub type GarbObject = Rc<RefCell<Object>>;

pub type Dict = HashMap<String, GarbObject>;

pub enum Number {
    Int(i64),
    Float(f64),
}

pub enum Atom {
    Str(String),
    Num(Number),
}

pub enum List {
    Node(ListNode),
    Null,
}

pub struct ListNode {
    pub next: Box<List>,
    pub val: GarbObject,
}

impl List {
    pub fn cons(self, other: GarbObject) -> Self {
        List::Node(ListNode {
            next: Box::new(self),
            val: other,
        })
    }
}

impl Object {
    pub fn is_func(&self) -> bool {
        match self {
            Func(_) => true,
            _       => false,
        }
    }
}
