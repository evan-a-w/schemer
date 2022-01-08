use crate::number::*;
use crate::runtime::Runtime;
use crate::env::Env;
use std::collections::HashMap;
use std::collections::LinkedList;
use gc_rs::{Gc, Trace};

pub type Id = usize;

pub type FuncId = usize;

pub enum Types {
    Number,
    String,
    Boolean,
    Function,
    Vector,
    Object,
    Null,
    Char,
    Symbol,
    Any,
    Iterable,
}

#[derive(Debug, PartialEq, Clone, Trace)]
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
    CFunc(Vec<String>, Gc<Ponga>, Gc<Env>), // args, sexpr_id, state_id
    MFunc(Vec<String>, Gc<Ponga>),
    HFunc(FuncId),
    True,
    False,
    Ref(Gc<Ponga>),
}

impl Ponga {
    pub fn is_func(&self) -> bool {
        match self {
            Ponga::CFunc(_, _, _)
            | Ponga::HFunc(_)
            | Ponga::MFunc(_, _) => true,
            _ => false,
        }
    }

    pub fn is_macro(&self) -> bool {
        match self {
            Ponga::MFunc(_, _) => true,
            _ => false,
        }
    }

    pub fn is_copy(&self) -> bool {
        match self {
            Ponga::Null
            | Ponga::Number(_)
            | Ponga::String(_)
            | Ponga::Char(_)
            | Ponga::Symbol(_)
            | Ponga::Ref(_)
            | Ponga::CFunc(_, _, _)
            | Ponga::MFunc(_, _)
            | Ponga::HFunc(_)
            | Ponga::Sexpr(_)
            | Ponga::True
            | Ponga::False => true,
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

    pub fn to_bool(&self) -> RunRes<bool> {
        match self {
            Ponga::True => Ok(true),
            Ponga::False => Ok(false),
            _ => Err(RuntimeErr::TypeError(format!("to bool: {:?} is not a boolean", self))),
        }
    }

    pub fn to_number(&self) -> RunRes<Number> {
        match self {
            Ponga::Number(n) => Ok(*n),
            _ => Err(RuntimeErr::TypeError(format!("to number: {:?} is not a number", self))),
        }
    }

    pub fn get_list(&mut self) -> RunRes<&mut LinkedList<Ponga>> {
        match self {
            Ponga::List(list) => Ok(list),
            _ => Err(RuntimeErr::TypeError(format!("get list: {:?} is not a list", self))),
        }
    }

    pub fn get_list_ref(&self) -> RunRes<&LinkedList<Ponga>> {
        match self {
            Ponga::List(list) => Ok(list),
            _ => Err(RuntimeErr::TypeError(format!("get list: {:?} is not a list", self))),
        }
    }

    pub fn get_number(&self) -> RunRes<Number> {
        match self {
            Ponga::Number(n) => Ok(*n),
            _ => Err(RuntimeErr::TypeError(format!("get number: {:?} is not a number", self))),
        }
    }

    pub fn get_array(self) -> RunRes<Vec<Ponga>> {
        match self {
            Ponga::Array(array) => Ok(array),
            Ponga::Sexpr(array) => Ok(array),
            _ => Err(RuntimeErr::TypeError(format!("get array: {:?} is not an array", self))),
        }
    }

    pub fn extract_name(self) -> RunRes<String> {
        match self {
            Ponga::Identifier(s) => Ok(s),
            Ponga::Symbol(s) => Ok(s),
            _ => Err(RuntimeErr::TypeError(format!("Expected an identifier, got {:?}", self))),
        }
    }
    
    pub fn extract_map(self) -> Option<HashMap<String, Ponga>> {
        match self {
            Ponga::Object(obj) => Some(obj),
            _ => None,
        }
    }

    pub fn extract_map_ref(&self) -> Option<&HashMap<String, Ponga>> {
        match self {
            Ponga::Object(obj) => Some(obj),
            _ => None,
        }
    }

    pub fn get_symbol_string(self) -> RunRes<String> {
        match self {
            Ponga::Symbol(s) => Ok(s),
            _ => Err(RuntimeErr::TypeError(format!(
                "Expected symbol, received {:?}", self)
            )),
        }
    }

    pub fn extract_map_ref_mut(&mut self) -> Option<&mut HashMap<String, Ponga>> {
        match self {
            Ponga::Object(obj) => Some(obj),
            _ => None,
        }
    }
    
    pub fn char_to_char(&self) -> RunRes<char> {
        match self {
            Ponga::Char(c) => Ok(*c),
            _ => Err(RuntimeErr::TypeError(format!("Expected char, received {:?}", self))),
        }
    }

    pub fn flip_code_vals(self, runtime: &Runtime) -> Ponga {
        match self {
            Ponga::List(l) => Ponga::Sexpr(l.into_iter()
                                            .map(|v| v.flip_code_vals(runtime)
                                          ).collect()),
            Ponga::Sexpr(p) => Ponga::List(p.into_iter()
                                            .map(|v| v.flip_code_vals(runtime)
                                          ).collect()),
            Ponga::Symbol(s) => Ponga::Identifier(s),
            Ponga::Identifier(s) => Ponga::Symbol(s),
            Ponga::Ref(id) => {
                let cloned = runtime.get_id_obj_ref(id).unwrap().as_ref().clone();
                cloned.flip_code_vals(runtime)
            }
            _ => self,
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

impl<E> std::convert::From<nom::Err<E>> for RuntimeErr {
    fn from(_: nom::Err<E>) -> Self {
        RuntimeErr::ParseError("Failed to parse".to_string())
    }
}

impl std::convert::From<std::io::Error> for RuntimeErr {
    fn from(e: std::io::Error) -> Self {
        RuntimeErr::StdIo(format!("{:?}", e))
    }
}

impl std::fmt::Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match self {
            Number::Int(i) => write!(f, "{}", i),
            Number::Float(fl) => write!(f, "{}", fl),
            Number::Rational(r) => write!(f, "{}", r),
        }
    }
}

impl std::fmt::Display for Ponga {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match self {
            Ponga::Number(n) => write!(f, "{}", n),
            Ponga::String(s) => write!(f, "\"{}\"", s),
            Ponga::False => write!(f, "#f"),
            Ponga::True => write!(f, "#t"),
            Ponga::Char(c) => write!(f, "#\\{}", c),
            Ponga::Null => write!(f, "'()"),
            Ponga::Symbol(s) => write!(f, "{}", s),
            Ponga::Array(arr) => write!(f, "{:?}", arr),
            Ponga::List(l) => write!(f, "{:?}", l),
            Ponga::HFunc(id) => write!(f, "Internal function with id {}", id),
            Ponga::CFunc(args, _, state) => write!(f, "Compound function with args {:?} and state {:?}", args, state),
            Ponga::MFunc(args, _) => write!(f, "Macro with args {:?}", args),
            Ponga::Sexpr(a) => write!(f, "S-expression {:?}", a),
            Ponga::Identifier(s) => write!(f, "{}", s),
            Ponga::Ref(obj) => write!(f, "{}", obj),
            Ponga::Object(o) => write!(f, "{:?}", o),
        }
    }
}
