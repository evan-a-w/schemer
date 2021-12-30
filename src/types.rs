use crate::ratio::*;
use crate::runtime::Runtime;
use std::collections::HashMap;
use std::collections::LinkedList;

pub type Id = usize;

pub type FuncId = usize;

#[derive(Debug, Clone, Copy, PartialEq)]
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
    CFunc(Vec<String>, Id, Id),
    HFunc(FuncId),
    True,
    False,
    Ref(Id),
}

impl Ponga {
    pub fn is_func(&self) -> bool {
        match self {
            Ponga::CFunc(_, _, _) => true,
            Ponga::HFunc(_) => true,
            _ => false,
        }
    }

    pub fn is_copy(&self) -> bool {
        match self {
            Ponga::Null => true,
            Ponga::Number(_) => true,
            Ponga::String(_) => true,
            Ponga::Char(_) => true,
            Ponga::Symbol(_) => true,
            Ponga::Ref(_) => true,
            Ponga::True => true,
            Ponga::False => true,
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

    pub fn extract_id(&self) -> RunRes<Id> {
        match self {
            Ponga::Ref(id) => Ok(id.clone()),
            _ => Err(RuntimeErr::TypeError(
                "Expected a reference".to_string(),
            )),
        }
    }

    pub fn get_list(&mut self) -> RunRes<&mut LinkedList<Ponga>> {
        match self {
            Ponga::List(list) => Ok(list),
            _ => Err(RuntimeErr::TypeError(format!("get list: {:?} is not a list", self))),
        }
    }

    pub fn get_array(self) -> RunRes<Vec<Ponga>> {
        match self {
            Ponga::Array(array) => Ok(array),
            Ponga::Sexpr(array) => Ok(array),
            _ => Err(RuntimeErr::TypeError(format!("get array: {:?} is not an array", self))),
        }
    }

    pub fn map_replace(self, name: &str, val: &Ponga) -> Ponga {
        match self {
            Ponga::Identifier(s) => {
                if s == name {
                    val.clone()
                } else {
                    Ponga::Identifier(s)
                }
            }
            Ponga::Sexpr(arr) => {
                Ponga::Sexpr(arr.into_iter().map(|v| v.map_replace(name, val)).collect())
            }
            Ponga::Array(arr) => {
                Ponga::Array(arr.into_iter().map(|v| v.map_replace(name, val)).collect())
            }
            Ponga::List(l) => {
                Ponga::List(l.into_iter().map(|v| v.map_replace(name, val)).collect())
            }
            Ponga::Object(obj) => {
                Ponga::Object(obj.into_iter().map(|(k, v)| (k, v.map_replace(name, val))).collect())
            }
            _ => self,
        }
    }

    pub fn extract_name(self) -> RunRes<String> {
        match self {
            Ponga::Identifier(s) => Ok(s),
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

    pub fn extract_map_ref_mut(&mut self) -> Option<&mut HashMap<String, Ponga>> {
        match self {
            Ponga::Object(obj) => Some(obj),
            _ => None,
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
            Ponga::Null => write!(f, "()"),
            Ponga::Symbol(s) => write!(f, "{}", s),
            Ponga::Array(arr) => write!(f, "{:?}", arr),
            Ponga::List(l) => write!(f, "{:?}", l),
            Ponga::HFunc(id) => write!(f, "Internal function with id {}", id),
            Ponga::CFunc(args, _, state) => write!(f, "Compound function with args {:?} and state {:?}", args, state),
            Ponga::Sexpr(a) => write!(f, "S-expression {:?}", a),
            Ponga::Identifier(s) => write!(f, "Identifier {}", s),
            Ponga::Ref(id) => write!(f, "Ref {}", id),
            Ponga::Object(o) => write!(f, "{:?}", o),
        }
    }
}

impl Number {
    pub fn plus(self, rhs: Number) -> Number {
        match self {
            Number::Int(i) => match rhs {
                Number::Int(j) => Number::Int(i + j),
                Number::Float(j) => Number::Float(i as f64 + j),
                Number::Rational(j) => Number::Rational(Ratio::from(i) + j),
            },
            Number::Float(i) => match rhs {
                Number::Int(j) => Number::Float(i + j as f64),
                Number::Float(j) => Number::Float(i + j),
                Number::Rational(j) => Number::Float(i + j.to_f64()),
            },
            Number::Rational(i) => match rhs {
                Number::Int(j) => Number::Rational(i + Ratio::from(j)),
                Number::Float(j) => Number::Float(i.to_f64() + j),
                Number::Rational(j) => Number::Rational(i + j),
            },
        }
    }

    pub fn minus(self, rhs: Number) -> Number {
        match self {
            Number::Int(i) => match rhs {
                Number::Int(j) => Number::Int(i - j),
                Number::Float(j) => Number::Float(i as f64 - j),
                Number::Rational(j) => Number::Rational(Ratio::from(i) - j),
            },
            Number::Float(i) => match rhs {
                Number::Int(j) => Number::Float(i - j as f64),
                Number::Float(j) => Number::Float(i - j),
                Number::Rational(j) => Number::Float(i - j.to_f64()),
            },
            Number::Rational(i) => match rhs {
                Number::Int(j) => Number::Rational(i - Ratio::from(j)),
                Number::Float(j) => Number::Float(i.to_f64() - j),
                Number::Rational(j) => Number::Rational(i - j),
            },
        }
    }

    pub fn times(self, rhs: Number) -> Number {
        match self {
            Number::Int(i) => match rhs {
                Number::Int(j) => Number::Int(i * j),
                Number::Float(j) => Number::Float(i as f64 * j),
                Number::Rational(j) => Number::Rational(Ratio::from(i) * j),
            },
            Number::Float(i) => match rhs {
                Number::Int(j) => Number::Float(i * j as f64),
                Number::Float(j) => Number::Float(i * j),
                Number::Rational(j) => Number::Float(i * j.to_f64()),
            },
            Number::Rational(i) => match rhs {
                Number::Int(j) => Number::Rational(i * Ratio::from(j)),
                Number::Float(j) => Number::Float(i.to_f64() * j),
                Number::Rational(j) => Number::Rational(i * j),
            },
        }
    }
    
    pub fn div(self, rhs: Number) -> Number {
        match self {
            Number::Int(i) => match rhs {
                Number::Int(j) => Number::Int(i / j),
                Number::Float(j) => Number::Float(i as f64 / j),
                Number::Rational(j) => Number::Rational(Ratio::from(i) / j),
            },
            Number::Float(i) => match rhs {
                Number::Int(j) => Number::Float(i / j as f64),
                Number::Float(j) => Number::Float(i / j),
                Number::Rational(j) => Number::Float(i / j.to_f64()),
            },
            Number::Rational(i) => match rhs {
                Number::Int(j) => Number::Rational(i / Ratio::from(j)),
                Number::Float(j) => Number::Float(i.to_f64() / j),
                Number::Rational(j) => Number::Rational(i / j),
            },
        }
    }

    pub fn eq(self, rhs: Number) -> bool {
        match self {
            Number::Int(i) => match rhs {
                Number::Int(j) => i == j,
                Number::Float(j) => i as f64 == j,
                Number::Rational(j) => Ratio::from(i) == j,
            },
            Number::Float(i) => match rhs {
                Number::Int(j) => i == j as f64,
                Number::Float(j) => i == j,
                Number::Rational(j) => i == j.to_f64(),
            },
            Number::Rational(i) => match rhs {
                Number::Int(j) => i == Ratio::from(j),
                Number::Float(j) => i.to_f64() == j,
                Number::Rational(j) => i == j,
            },
        }
    }

    pub fn ge(self, rhs: Number) -> bool {
        match self {
            Number::Int(i) => match rhs {
                Number::Int(j) => i >= j,
                Number::Float(j) => i as f64 >= j,
                Number::Rational(j) => Ratio::from(i) >= j,
            },
            Number::Float(i) => match rhs {
                Number::Int(j) => i >= j as f64,
                Number::Float(j) => i >= j,
                Number::Rational(j) => i >= j.to_f64(),
            },
            Number::Rational(i) => match rhs {
                Number::Int(j) => i >= Ratio::from(j),
                Number::Float(j) => i.to_f64() >= j,
                Number::Rational(j) => i >= j,
            },
        }
    }

    pub fn gt(self, rhs: Number) -> bool {
        match self {
            Number::Int(i) => match rhs {
                Number::Int(j) => i > j,
                Number::Float(j) => i as f64 > j,
                Number::Rational(j) => Ratio::from(i) > j,
            },
            Number::Float(i) => match rhs {
                Number::Int(j) => i > j as f64,
                Number::Float(j) => i > j,
                Number::Rational(j) => i > j.to_f64(),
            },
            Number::Rational(i) => match rhs {
                Number::Int(j) => i > Ratio::from(j),
                Number::Float(j) => i.to_f64() > j,
                Number::Rational(j) => i > j,
            },
        }
    }

    pub fn lt(self, rhs: Number) -> bool {
        !self.ge(rhs)
    }

    pub fn le(self, rhs: Number) -> bool {
        !self.gt(rhs)
    }

    pub fn to_isize(self) -> isize {
        match self {
            Number::Int(i) => i,
            Number::Float(i) => i as isize,
            Number::Rational(r) => r.to_f64() as isize,
        }
    }

    pub fn modulus(self, rhs: Number) -> Number {
        let a = self.to_isize();
        let b = rhs.to_isize();
        Number::Int(a % b)
    }
}
