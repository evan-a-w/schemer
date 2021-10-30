use crate::types::*;
use std::cell::RefCell;
use crate::parser::Program;
use std::collections::HashMap;
use std::rc::Rc;

pub fn get_stdlib() -> HashMap<String, fn(&mut Program, Vec<GarbObject>) -> GarbObject> {
    let mut d: HashMap<String, fn(&mut Program, Vec<GarbObject>) -> GarbObject>
        = HashMap::new();
    d.insert(
        "cons".to_string(),
        cons,
    );
    d.insert(
        "car".to_string(),
        car,
    );
    d.insert(
        "cdr".to_string(),
        cdr,
    );
    d.insert(
        "+".to_string(),
        plus,
    );
    d.insert(
        "let".to_string(),
        let_,
    );
    d
}

pub fn cons(state: &mut Program, args: Vec<GarbObject>) -> GarbObject {
    if args.len() != 2 {
        Object::Error("Arguments to cons != 2".to_string()).to_garbobject()
    } else {
        match {
            let mut b = args[1].clone();
            state.convert_obj(&mut b); 
            let b = b.borrow();
            b.type_of()
        } {
            Type::List => Object::List(List::Node(Rc::new(RefCell::new(ListNode {
                next: args[1].clone(),
                val: args[0].clone(),
            }))))
            .to_garbobject(),
            // Propagate errors
            Type::Error => args[1].clone(),
            _ => Object::Error("2nd arg to cons is not a list".to_string()).to_garbobject(),
        }
    }
}

pub fn car(state: &mut Program, args: Vec<GarbObject>) -> GarbObject {
    if args.len() != 1 {
        Object::Error("Arguments to car != 1".to_string()).to_garbobject()
    } else {
        match {
            let mut b = args[0].clone();
            state.convert_obj(&mut b); 
            let b = b.borrow();
            b.type_of()
        } {
            Type::List => match args[0].borrow().head_list() {
                None => Object::Error("Car of empty list".to_string()).to_garbobject(),
                Some(v) => v,
            },
            // Propagate errors
            Type::Error => args[0].clone(),
            _ => Object::Error("Arg to car is not a list".to_string()).to_garbobject(),
        }
    }
}

pub fn cdr(state: &mut Program, args: Vec<GarbObject>) -> GarbObject {
    if args.len() != 1 {
        Object::Error("Arguments to car != 1".to_string()).to_garbobject()
    } else {
        match {
            let mut b = args[0].clone();
            state.convert_obj(&mut b); 
            let b = b.borrow();
            b.type_of()
        } {
            Type::List => match args[0].borrow().tail_list() {
                None => Object::Error("Cdr of empty list".to_string()).to_garbobject(),
                Some(v) => v,
            },
            // Propagate errors
            Type::Error => args[0].clone(),
            _ => Object::Error("Arg to car is not a list".to_string()).to_garbobject(),
        }
    }
}

pub fn plus(state: &mut Program, args: Vec<GarbObject>) -> GarbObject {
    args.into_iter().fold(
        Object::Num(Number::Int(0)),
        |acc, mut x| match { state.convert_obj(&mut x); acc } {
            Object::Num(Number::Int(a)) => match &*x.borrow() {
                Object::Num(Number::Int(b)) => Object::Num(Number::Int(a + b)),
                Object::Num(Number::Float(f)) => Object::Num(Number::Float(a as f64 + f)),
                // Propagate errors
                Object::Error(e) => Object::Error(e.to_string()),
                _ => Object::Error("Arg to + is not a number".to_string()),
            },
            Object::Num(Number::Float(a)) => match &*x.borrow() {
                Object::Num(Number::Int(b)) => Object::Num(Number::Float(a + *b as f64)),
                Object::Num(Number::Float(f)) => Object::Num(Number::Float(a as f64 + f)),
                // Propagate errors
                Object::Error(e) => Object::Error(e.to_string()),
                _ => Object::Error("Arg to + is not a number".to_string()),
            },
            // Propagate errors
            Object::Error(e) => Object::Error(e),
            _ => Object::Error("Arg to + is not a number".to_string()),
        }
    ).to_garbobject()
}

pub fn let_(state: &mut Program, mut args: Vec<GarbObject>) -> GarbObject {
    if args.len() != 2 {
        Object::Error("Arguments to let != 2".to_string()).to_garbobject()
    } else {
        args.push(Object::Unit.to_garbobject());
        match &*args.swap_remove(0).borrow() {
            Object::L(v) => {
                let mut curr = HashMap::new();
                for p in v.into_iter() {
                    match &*p.borrow() {
                        Object::L(sl) => {
                            if sl.len() != 2 {
                                return Object::Error(
                                    format!("Invalid let binding with {:?} (length {})"
                                            , sl, sl.len())
                                ).to_garbobject();
                            }
                            match &*sl[0].borrow() {
                                Object::Symbol(s) => {
                                    curr.insert(s.clone(), state.eval(sl[1].clone()));
                                },
                                _ => {
                                    return Object::Error(
                                        format!("Invalid let binding with {:?}", sl)
                                    ).to_garbobject();
                                }
                            }
                            
                        }
                        _ => {
                            return Object::Error("Arg to let is not a list".to_string()).to_garbobject();
                        }
                    }
                }
                state.locals.push(curr);
                state.convert_obj(&mut args[1]);
                let res = state.eval(args[1].clone());
                state.locals.pop();
                res
            }
            _ => {
                Object::Error("First arg to let is not a list".to_string()).to_garbobject()
            }
        }
    }
}

pub fn print_ret(x: GarbObject) -> GarbObject {
    println!("{:?}", x);
    x
}
