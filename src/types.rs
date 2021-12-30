use crate::ratio::*;
use crate::runtime::Runtime;
use std::collections::HashMap;
use std::collections::LinkedList;

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

    pub fn is_list(&self) -> bool {
        match self {
            Ponga::List(_) => true,
            _ => false,
        }
    }

    pub fn is_vector(&self) -> bool {
        match self {
            Ponga::Array(_) => true,
            _ => false,
        }
    }

    pub fn is_object(&self) -> bool {
        match self {
            Ponga::Object(_) => true,
            _ => false,
        }
    }

    pub fn is_sexpr(&self) -> bool {
        match self {
            Ponga::Sexpr(_) => true,
            _ => false,
        }
    }

    pub fn is_null(&self) -> bool {
        match self {
            Ponga::Null => true,
            _ => false,
        }
    }

    pub fn is_number(&self) -> bool {
        match self {
            Ponga::Number(_) => true,
            _ => false,
        }
    }
    
    pub fn is_string(&self) -> bool {
        match self {
            Ponga::String(_) => true,
            _ => false,
        }
    }

    pub fn is_char(&self) -> bool {
        match self {
            Ponga::Char(_) => true,
            _ => false,
        }
    }

    pub fn is_symbol(&self) -> bool {
        match self {
            Ponga::Symbol(_) => true,
            _ => false,
        }
    }

    pub fn is_identifier(&self) -> bool {
        match self {
            Ponga::Identifier(_) => true,
            _ => false,
        }
    }

    pub fn is_true(&self) -> bool {
        match self {
            Ponga::True => true,
            _ => false,
        }
    }

    pub fn is_false(&self) -> bool {
        match self {
            Ponga::False => true,
            _ => false,
        }
    }

    pub fn is_ref(&self) -> bool {
        match self {
            Ponga::Ref(_) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeErr {
    TypeError(String),
    ReferenceError(String),
    ParseError(String),
    StdIo(String),
    Other(String),
}

impl std::fmt::Display for RuntimeErr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use RuntimeErr::*;
        match self {
            TypeError(s) => write!(f, "TypeError: {}", s),
            ReferenceError(s) => write!(f, "ReferenceError: {}", s),
            ParseError(s) => write!(f, "ParseError: {}", s),
            StdIo(s) => write!(f, "IO error: {}", s),
            Other(s) => write!(f, "Error: {}", s),
        }
    }
}

impl std::error::Error for RuntimeErr {}

pub type RunRes<T> = Result<T, RuntimeErr>;

use nom::error::{self, ErrorKind};

impl<E> std::convert::From<nom::Err<E>> for RuntimeErr {
    fn from(e: nom::Err<E>) -> Self {
        RuntimeErr::ParseError("Failed to parse".to_string())
    }
}

impl std::convert::From<std::io::Error> for RuntimeErr {
    fn from(e: std::io::Error) -> Self {
        RuntimeErr::StdIo(format!("{:?}", e))
    }
}
