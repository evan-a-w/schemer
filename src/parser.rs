use crate::stdlib::*;
use crate::types::*;
use std::collections::HashMap;
use lazy_static::lazy_static;
use regex::Regex;
use std::cell::RefCell;
use std::rc::Rc;

pub struct Program {
    pub locals: Frames,
    pub globs: Dict,
    pub curr_level: usize,
    pub stdlib: HashMap<String, fn(&mut Program, Vec<GarbObject>) -> GarbObject>,
}

pub fn parse_string(s: String) -> Option<Vec<String>> {
    let mut res = vec![];
    let mut opens = 0;
    let mut curr_buff = String::new();
    for c in s.chars() {
        if c == ')' {
            opens -= 1;
            if opens < 0 {
                return None;
            }
            curr_buff += &c.to_string();
            if opens == 0 {
                res.push(curr_buff);
                curr_buff = String::new();
            }
        } else if c == '(' {
            curr_buff += &c.to_string();
            opens += 1;
        } else if opens == 0 {
            if !c.is_whitespace() {
                curr_buff += &c.to_string();
            } else if curr_buff.len() > 0 {
                res.push(curr_buff);
                curr_buff = String::new();
            }
        } else {
            curr_buff += &c.to_string();
        }
    }
    if opens != 0 {
        return None;
    }
    if curr_buff.len() > 0 {
        res.push(curr_buff);
    }
    Some(res)
}

impl Program {
    pub fn new() -> Self {
        Program {
            globs: Dict::new(),
            locals: vec![],
            stdlib: get_stdlib(),
            curr_level: 0,
        }
    }

    pub fn get_inside_list(&mut self, s: String) -> List {
        let mut curr: List = List::Null;
        self.curr_level += 1;
        for st in parse_string(s).unwrap().into_iter().rev() {
            if let Some(x) = self.string_eval(st) {
                let t = List::Node(ListNode {
                    val: x,
                    next: Rc::new(RefCell::new(Object::List(curr))),
                });
                curr = t;
            }
        }
        self.curr_level -= 1;
        if self.locals.len() > 0
           && self.locals[self.locals.len() - 1].1 > self.curr_level {
            self.locals.pop();
        }
        curr
    }

    pub fn get_inside_arr(&mut self, s: String) -> Vec<GarbObject> {
        let mut arr: Vec<GarbObject> = vec![];
        self.curr_level += 1;
        for st in parse_string(s).unwrap().into_iter() {
            if let Some(x) = self.string_eval(st) {
                arr.push(x);
            }
        }
        self.curr_level -= 1;
        if self.locals.len() > 0
           && self.locals[self.locals.len() - 1].1 > self.curr_level {
            self.locals.pop();
        }
        arr
    }

    pub fn find_frames(&self, s: &str) -> Option<GarbObject> {
        for (frame, _) in self.locals.iter().rev() {
            if let Some(x) = frame.get(s) {
                return Some(x.clone());
            }
        }
        None
    }

    pub fn get_ref(&self, s: &str) -> Option<GarbObject> {
        match self.find_frames(s) {
            None => match self.globs.get(s) {
                None => match self.stdlib.get(s) {
                    None => None,
                    Some(x) => Some(Object::Func(Function::Base(s.to_string()))
                                   .to_garbobject()),
                },
                Some(v) => Some(v.clone()),
            },
            Some(v) => Some(v),
        }
    }

    pub fn func_eval(&mut self, v: GarbObject, args: Vec<GarbObject>) -> GarbObject {
        match &*v.borrow() {
            Object::Func(Function::Base(s)) => self.stdlib.get(s).unwrap()(self, args),
            Object::Func(Function::Sequence(seq)) => {
                Object::Error("Unimplemented".to_string()).to_garbobject()
            }
            _ => Object::Error("Evaluating a non-function".to_string()).to_garbobject(),
        }
    }

    pub fn string_eval(&mut self, s: String) -> Option<Rc<RefCell<Object>>> {
        lazy_static! {
            static ref STRING_RE: Regex = Regex::new(r#"^"([^'\#\s]*)"$"#).unwrap();
            static ref INT_RE: Regex = Regex::new(r"^[0-9]+$").unwrap();
            static ref FLOAT_RE: Regex = Regex::new(r"^[0-9]+\.[0-9]+$").unwrap();
            static ref BOOL_RE: Regex = Regex::new(r"^#([tT])?([fF])?$").unwrap();
            static ref REF_RE: Regex = Regex::new(r"^\w+$").unwrap();
            static ref SYMBOL_RE: Regex = Regex::new(r"^'(\w+)$").unwrap();
            static ref ARRAY_RE: Regex = Regex::new(r"^'#\((.*)\)$").unwrap();
            static ref LIST_RE: Regex = Regex::new(r"^'\((.*)\)$").unwrap();
            static ref FUNC_RE: Regex = Regex::new(r"^\(([a-zA-Z0-9_+/*%$^<>?:;\[\]{}\.-]+)(.*)\)$").unwrap();
        }
        if let Some(c) = STRING_RE.captures(&s) {
            Some(Rc::new(RefCell::new(Object::Atom(Atom::Str(
                c.get(1).unwrap().as_str().to_string(),
            )))))
        } else if INT_RE.is_match(&s) {
            Some(Rc::new(RefCell::new(Object::Num(Number::Int(
                s.parse().unwrap(),
            )))))
        } else if FLOAT_RE.is_match(&s) {
            Some(Rc::new(RefCell::new(Object::Num(Number::Float(
                s.parse().unwrap(),
            )))))
        } else if let Some(x) = BOOL_RE.captures(&s) {
            if let Some(_) = x.get(1) {
                Some(Object::Bool(true).to_garbobject())
            } else {
                Some(Object::Bool(false).to_garbobject())
            }
        } else if let Some(c) = SYMBOL_RE.captures(&s) {
            Some(Rc::new(RefCell::new(Object::Symbol(
                c.get(1).unwrap().as_str().to_string(),
            ))))
        } else if REF_RE.is_match(&s) {
            match self.get_ref(&s) {
                None => None,
                Some(v) => Some(v),
            }
        } else if let Some(cap) = FUNC_RE.captures(&s) {
            match self.get_ref(cap.get(1).unwrap().as_str()) {
                None => None,
                Some(v) => {
                    if v.borrow().is_func() {
                        let vec = match cap.get(2) {
                            None => vec![],
                            Some(c) => self.get_inside_arr(c.as_str().to_string()),
                        };
                        Some(self.func_eval(v, vec))
                    } else {
                        None
                    }
                }
            }
        } else if let Some(cap) = ARRAY_RE.captures(&s) {
            Some(Rc::new(RefCell::new(Object::Array(
                self.get_inside_arr(cap.get(1).unwrap().as_str().to_string()),
            ))))
        } else if let Some(cap) = LIST_RE.captures(&s) {
            Some(Rc::new(RefCell::new(Object::List(
                self.get_inside_list(cap.get(1).unwrap().as_str().to_string()),
            ))))
        } else {
            None
        }
    }
}
