use crate::types::*;
use regex::Regex;
use lazy_static::lazy_static;
use std::rc::Rc;
use std::cell::RefCell;

pub struct Program {
    pub locs: Dict,
    pub globs: Dict,
}

pub fn tokenize(chars: String) -> Vec<String> {
    chars.replace("(", " ( ")
        .replace(")", " ) ")
        .split(' ')
        .map(|s| s.to_string())
        .collect()
}

impl Program {
    pub fn new() -> Self {
        Program { globs: Dict::new(), locs: Dict::new() }
    }
    
    pub fn get_inside_list(&self, s: String) -> List {
        let mut curr: List = List::Null;
        for s in s.split_whitespace() {
            if let Some(x) = self.string_eval(s.to_string()) {
                let t = curr.cons(x);
                curr = t;
            }
        }
        curr
    }

    pub fn get_inside_arr(&self, s: String) -> Vec<GarbObject> {
        let mut arr: Vec<GarbObject> = vec![];
        for s in s.split_whitespace() {
            if let Some(x) = self.string_eval(s.to_string()) {
                arr.push(x);
            }
        }
        arr
    }

    pub fn get_ref(&self, s: &str) -> Option<GarbObject> {
        match self.locs.get(s) {
            None => match self.globs.get(s) {
                None => None,
                Some(v) => Some(v.clone()),
            }
            Some(v) => Some(v.clone()),
        }
    }

    pub fn string_eval(&self, s: String) -> Option<Rc<RefCell<Object>>>  {
        lazy_static! {
            static ref STRING_RE: Regex = Regex::new(r#""\w*""#).unwrap();
            static ref NUMBER_RE: Regex = Regex::new(r"[0-9.]+").unwrap();
            static ref REF_RE: Regex = Regex::new(r"\w+").unwrap();
            static ref SYMBOL_RE: Regex = Regex::new(r"'\w+").unwrap();
            static ref ARRAY_RE: Regex = Regex::new(r"#\((.*)\)").unwrap();
            static ref LIST_RE: Regex = Regex::new(r"'\((.*)\)").unwrap();
            static ref FUNC_RE: Regex = Regex::new(r"\(([a-zA-Z0-9_-.]+)(.*)\)").unwrap();
        } 
        if STRING_RE.is_match(&s) {
            Some(Rc::new(RefCell::new(Object::Atom(Atom::Str(s)))))
        } else if NUMBER_RE.is_match(&s) {
            Some(Rc::new(RefCell::new(Object::Num(Number::Int(s.parse().unwrap())))))
        } else if REF_RE.is_match(&s) {
            match self.get_ref(&s) {
                None => None,
                Some(v) => Some(v),
            } 
        } else if SYMBOL_RE.is_match(&s) {
            Some(Rc::new(RefCell::new(Object::Symbol(s))))
        } else if let Some(cap) = FUNC_RE.captures(&s) {
            match self.get_ref(cap.get(1).unwrap().as_str()) {
                None => None,
                Some(v) => {
                    if v.borrow().is_func() {
                        let vec = match cap.get(2) {
                            None => vec![],
                            Some(c) => self.get_inside_arr(c.as_str().to_string()),
                        };
                        Some(Rc::new(RefCell::new(Object::Thonk((v.clone(), vec)))))
                    } else {
                        None
                    }
                }
            }
        } else if let Some(cap) = ARRAY_RE.captures(&s) {
            Some(Rc::new(RefCell::new(Object::Array(
                self.get_inside_arr(cap.get(1).unwrap().as_str().to_string())
            ))))
        } else if let Some(cap) = LIST_RE.captures(&s) {
            Some(Rc::new(RefCell::new(Object::List(
                self.get_inside_list(cap.get(1).unwrap().as_str().to_string())
            ))))
        } else {
            None
        }
    }
}
