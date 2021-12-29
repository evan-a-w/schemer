use std::collections::HashMap;
use std::collections::LinkedList;
use crate::ratio::*;
use crate::runtime::Runtime;

pub type Id = usize;

pub type FuncId = usize;

#[derive(Debug, Clone, PartialEq)]
pub enum Number {
    Int(isize),
    Float(f64),
    Rational(Ratio<isize>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Ponga {
    Null,
    Number(Number),
    String(String),
    Char(char),
    Symbol(String),
    Identifier(String),
    List(LinkedList<Ponga>),
    Object(HashMap<String, Ponga>),
    Array(Vec<Ponga>),
    Sexpr(Vec<Ponga>),
    CFunc(Vec<String>, Id),
    HFunc(FuncId),
    True,
    False,
    Ref(Id),
}

impl Ponga {
    pub fn is_func(&self) -> bool {
        match self {
            Ponga::CFunc(_, _) => true,
            Ponga::HFunc(_) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeErr {
    TypeError(String),
    ReferenceError(String),
    Other(String),
}

impl std::fmt::Display for RuntimeErr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use RuntimeErr::*;
        match self {
            TypeError(s) => write!(f, "TypeError: {}", s),
            ReferenceError(s) => write!(f, "ReferenceError: {}", s),
            Other(s) => write!(f, "Error: {}", s),
        }
    }
}

impl std::error::Error for RuntimeErr {}

pub type RunRes<T> = Result<T, RuntimeErr>;
