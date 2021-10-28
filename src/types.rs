use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;
use crate::parser::Program;
use std::cmp::PartialEq;

#[derive(PartialEq, Debug)]
pub enum Object {
    Symbol(String),
    Num(Number),
    Atom(Atom),
    List(List),
    Env(Dict),
    Func(Function),
    Thonk((GarbObject, Vec<GarbObject>)),
    Array(Vec<GarbObject>),
    Error(String),
    Bool(bool),
}

pub enum Type {
    Symbol,
    Num,
    Atom,
    List,
    Env,
    Func,
    Thonk,
    Array,
    Error,
    Bool,
}

pub enum Function {
    Base(fn(&mut Dict, &mut Dict, Vec<GarbObject>) -> GarbObject, u16),
    Sequence(RuntimeFunc),
}

impl PartialEq for Function {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Function::Sequence(rt1) => {
                if let Function::Sequence(rt2) = other {
                    rt1 == rt2
                } else {
                    false
                }
            }
            Function::Base(_, id1) => {
                if let Function::Base(_, id2) = other {
                    id1 == id2
                } else {
                    false
                }
            }
        }
    }
}

impl fmt::Debug for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Function::Sequence(rt) => {
                write!(f, "{:?}", rt)
            }
            Function::Base(_, id) => {
                write!(f, "Base func with id {}", id)
            }
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct RuntimeFunc {
    args: usize,
    operations: Vec<GarbObject>,
}

pub type GarbObject = Rc<RefCell<Object>>;

pub type Dict = HashMap<String, GarbObject>;

#[derive(PartialEq, Debug)]
pub enum Number {
    Int(i64),
    Float(f64),
}

#[derive(PartialEq, Debug)]
pub enum Atom {
    Str(String),
    Num(Number),
}

#[derive(PartialEq, Debug)]
pub enum List {
    Node(ListNode),
    Null,
}

#[derive(PartialEq, Debug)]
pub struct ListNode {
    pub next: GarbObject,
    pub val: GarbObject,
}

impl List {
    pub fn cons(self, other: GarbObject) -> Self {
        List::Node(ListNode {
            next: Object::List(self).to_garbobject(),
            val: other,
        })
    }

    pub fn head(&self) -> Option<GarbObject> {
        match self {
            List::Null => None,
            List::Node(x) => Some(x.val.clone()),
        }
    }

    pub fn tail(&self) -> Option<GarbObject> {
        match self {
            List::Null => None,
            List::Node(x) => Some(x.next.clone()),
        }
    }

    pub fn next_null(&self) -> bool {
        match self {
            List::Node(n) => match &*n.next.borrow() {
                Object::List(n) => match n {
                    List::Null => true,
                    _ => false,
                },
                _ => false,
            },
            List::Null => false,
        }
    }

    pub fn print_rec(&self, f: &mut fmt::Formatter<'_>, space: bool) {
        match self {
            List::Null => (),
            List::Node(n) => {
                if space {
                    write!(f, " ");
                }
                write!(f, "{}", n.val.borrow()).unwrap_or(());
                n.next.borrow().print_rec_list(f, true);
            }
        }
    }
}

impl Object {
    pub fn is_func(&self) -> bool {
        match self {
            Object::Func(_) => true,
            _ => false,
        }
    }

    pub fn type_of(&self) -> Type {
        match self {
            Object::Symbol(_) => Type::Symbol,
            Object::Num(_) => Type::Num,
            Object::Atom(_) => Type::Atom,
            Object::List(_) => Type::List,
            Object::Env(_) => Type::Env,
            Object::Func(_) => Type::Func,
            Object::Thonk(_) => Type::Thonk,
            Object::Array(_) => Type::Array,
            Object::Error(_) => Type::Error,
            Object::Bool(_) => Type::Bool,
        }
    }

    pub fn head_list(&self) -> Option<GarbObject> {
        match self {
            Object::List(v) => v.head(),
            _ => None,
        }
    }

    pub fn tail_list(&self) -> Option<GarbObject> {
        match self {
            Object::List(v) => v.tail(),
            _ => None,
        }
    }

    pub fn to_garbobject(self) -> GarbObject {
        Rc::new(RefCell::new(self))
    }

    pub fn print_rec_list(&self, f: &mut fmt::Formatter<'_>, space: bool) {
        match self {
            Object::List(n) => n.print_rec(f, space),
            _ => (),
        }
    }
}

impl fmt::Display for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Object::Symbol(s) => write!(f, "{}", s),
            Object::Num(n) => match n {
                Number::Int(i) => write!(f, "{}", i),
                Number::Float(d) => write!(f, "{}", d),
            },
            Object::Atom(a) => match a {
                Atom::Num(n) => match n {
                    Number::Int(i) => write!(f, "{}", i),
                    Number::Float(d) => write!(f, "{}", d),
                },
                Atom::Str(s) => write!(f, "\"{}\"", s),
            },
            Object::List(l) => write!(f, "'({})", l),
            Object::Env(_) => write!(f, "Environment"),
            Object::Func(_) => write!(f, "Function object"),
            Object::Thonk(_) => write!(f, "Thonkerinni"),
            Object::Array(a) => {
                write!(f, "'#(").unwrap();
                let mut between = false;
                for i in a.iter() {
                    if between {
                        write!(f, " ").unwrap();
                    }
                    write!(f, "{}", i.borrow()).unwrap();
                    between = true;
                }
                write!(f, ")").unwrap();
                Ok(())
            }
            Object::Error(s) => write!(f, "Error '{}'", s),
            Object::Bool(b) => write!(f, "{}", if *b { "#t" } else { "#f" }),
        }
    }
}

impl fmt::Display for List {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.print_rec(f, false);
        Ok(())
    }
}
