use crate::types::*;
use crate::parser::Program;
use std::collections::HashMap;

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
    d
}

pub fn cons(state: &mut Program, args: Vec<GarbObject>) -> GarbObject {
    if args.len() != 2 {
        Object::Error("Arguments to cons != 2".to_string()).to_garbobject()
    } else {
        println!("{}", args[1].borrow());
        match {
            let b = args[1].borrow();
            b.type_of()
        } {
            Type::List => Object::List(List::Node(ListNode {
                next: args[1].clone(),
                val: args[0].clone(),
            }))
            .to_garbobject(),
            // Propogate errors
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
            let b = args[0].borrow();
            b.type_of()
        } {
            Type::List => match args[0].borrow().head_list() {
                None => Object::Error("Car of empty list".to_string()).to_garbobject(),
                Some(v) => v,
            },
            // Propogate errors
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
            let b = args[0].borrow();
            b.type_of()
        } {
            Type::List => match args[0].borrow().tail_list() {
                None => Object::Error("Cdr of empty list".to_string()).to_garbobject(),
                Some(v) => v,
            },
            // Propogate errors
            Type::Error => args[0].clone(),
            _ => Object::Error("Arg to car is not a list".to_string()).to_garbobject(),
        }
    }
}

pub fn plus(state: &mut Program, args: Vec<GarbObject>) -> GarbObject {
    args.into_iter().fold(
        Object::Num(Number::Int(0)),
        |acc, x| match acc {
            Object::Num(Number::Int(a)) => match &*x.borrow() {
                Object::Num(Number::Int(b)) => Object::Num(Number::Int(a + b)),
                Object::Num(Number::Float(f)) => Object::Num(Number::Float(a as f64 + f)),
                // Propogate errors
                Object::Error(e) => Object::Error(e.to_string()),
                _ => Object::Error("Arg to + is not a number".to_string()),
            },
            Object::Num(Number::Float(a)) => match &*x.borrow() {
                Object::Num(Number::Int(b)) => Object::Num(Number::Float(a + *b as f64)),
                Object::Num(Number::Float(f)) => Object::Num(Number::Float(a as f64 + f)),
                // Propogate errors
                Object::Error(e) => Object::Error(e.to_string()),
                _ => Object::Error("Arg to + is not a number".to_string()),
            },
            // Propogate errors
            Object::Error(e) => Object::Error(e),
            _ => Object::Error("Arg to + is not a number".to_string()),
        }
    ).to_garbobject()
}

pub fn insert_list_(&mut Program, n: &ListNode, curr: &) {
    match &*n.val.borrow() {
        Object::List(List::Node(def)) => {
            match &*def.val.borrow() {
                Object::Symbol(s) => {
                    if let Object::List(List::Node(val)) = &*def.next.borrow() {
                        curr.insert(s.clone(), val.val.clone());
                    } else {
                        return;
                    }
                }
                _ => return Object::Error("first thingy not symbol!".to_string()).to_garbobject(),
            }
        }
        _ => { return Object::Error("Invalid type to let".to_string()).to_garbobject(); },
    }
}

pub fn let_(state: &mut Program, args: Vec<GarbObject>) -> GarbObject {
    if args.len() != 2 {
        Object::Error("Arguments to let != 2".to_string()).to_garbobject()
    } else {
        match &*args[0].borrow() {
            Object::List(List::Node(fst)) => {
                let mut n = fst;
                let mut curr: HashMap<String, GarbObject> = HashMap::new();
                loop {
                    match &*n.val.borrow() {
                        Object::List(List::Node(def)) => {
                            match &*def.val.borrow() {
                                Object::Symbol(s) => {
                                    if let Object::List(List::Node(val)) = &*def.next.borrow() {
                                        curr.insert(s.clone(), val.val.clone());
                                    } else {
                                        return Object::Error("Invalid list in let".to_string()).to_garbobject();
                                    }
                                }
                                _ => return Object::Error("first thingy not symbol!".to_string()).to_garbobject(),
                            }
                        }
                        _ => { return Object::Error("Invalid type to let".to_string()).to_garbobject(); },
                    }
                    let t = n.next.borrow();
                    n = match n.next.borrow().get_node() {
                        None => { break; },
                        Some(v) => v,
                    };
                    //if let Object::List(List::Node(nec)) = &*t {
                    //    n = nec.clone();
                    //} else {
                    //    break;
                    //}
                }
                state.locals.push((curr, state.curr_level));
                Object::Unit.to_garbobject()
            }
            _ => Object::Error("First arg to let is not a list".to_string()).to_garbobject(),
        }
    }
}

pub fn print_ret(x: GarbObject) -> GarbObject {
    println!("{:?}", x);
    x
}
