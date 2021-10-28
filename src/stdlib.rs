use crate::types::*;

pub fn get_stdlib() -> Dict {
    let mut d = Dict::new();
    d.insert(
        "cons".to_string(),
        Object::Func(Function::Base(cons, 0)).to_garbobject(),
    );
    d.insert(
        "car".to_string(),
        Object::Func(Function::Base(car, 1)).to_garbobject(),
    );
    d.insert(
        "cdr".to_string(),
        Object::Func(Function::Base(cdr, 2)).to_garbobject(),
    );
    d
}

pub fn cons(locs: &mut Dict, globs: &mut Dict, args: Vec<GarbObject>) -> GarbObject {
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

pub fn car(locs: &mut Dict, globs: &mut Dict, args: Vec<GarbObject>) -> GarbObject {
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

pub fn cdr(locs: &mut Dict, globs: &mut Dict, args: Vec<GarbObject>) -> GarbObject {
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
