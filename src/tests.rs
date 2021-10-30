use crate::parser::*;
use crate::stdlib::*;
use crate::types::*;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn test_parse() {
    assert!(
        parse_string("cons '#(1 2 3) '(1 2) '() () \"name\" pee".to_string())
            .unwrap()
            .len()
            == 7
    );
}

#[test]
fn test_basic_evaluation() {
    let mut p = Program::new();
    assert_eq!(
        p.full_string_eval("\"name\"".to_string()),
        Some(Rc::new(RefCell::new(Object::Atom(Atom::Str(
            "name".to_string()
        )))))
    );
    assert_eq!(
        p.full_string_eval("'name".to_string()),
        Some(Rc::new(RefCell::new(Object::Symbol("name".to_string()))))
    );
    assert_eq!(
        p.full_string_eval("#t".to_string()),
        Some(Rc::new(RefCell::new(Object::Bool(true))))
    );
    assert_eq!(
        p.full_string_eval("#f".to_string()),
        Some(Rc::new(RefCell::new(Object::Bool(false))))
    );
    assert_eq!(
        p.full_string_eval("1001234".to_string()),
        Some(Rc::new(RefCell::new(Object::Num(Number::Int(1001234)))))
    );
    assert_eq!(
        p.full_string_eval("5.263".to_string()),
        Some(Rc::new(RefCell::new(Object::Num(Number::Float(5.263)))))
    );
    assert!(!p.full_string_eval("cons".to_string()).is_none());
    assert!(
        format!(
            "{}",
            p.full_string_eval("'(1 2 \"name\")".to_string())
                .unwrap()
                .borrow()
        ) == "'(1 2 \"name\")"
    );
    assert!(
        format!(
            "{}",
            p.full_string_eval("'#(1 2 \"name\")".to_string())
                .unwrap()
                .borrow()
        ) == "'#(1 2 \"name\")"
    );
    // Cons 
    assert!(
        format!(
            "{}",
            p.full_string_eval("(cons 1 '(2 3))".to_string())
                .unwrap()
                .borrow()
        ) == "'(1 2 3)"
    );
    // Car on list
    assert!(
        format!(
            "{}",
            p.full_string_eval("(car '(1 2 3))".to_string())
                .unwrap()
                .borrow()
        ) == "1"
    );
    // cdr on list
    assert!(
        format!(
            "{}",
            p.full_string_eval("(cdr '(1 23 \"name\" #t))".to_string())
                .unwrap()
                .borrow()
        ) == "'(23 \"name\" #t)"
    );
    // Addition
    assert!(
        format!(
            "{}",
            p.full_string_eval("(+ 5 1)".to_string())
                .unwrap()
                .borrow()
        ) == "6"
    );
    // Addition
    assert!(
        format!(
            "{}",
            p.full_string_eval("(+ 5 1 2.0 6)".to_string())
                .unwrap()
                .borrow()
        ) == "14"
    );
    assert!(format!("{}", p.full_string_eval("'(1 '(1 2))".to_string())
            .unwrap().borrow()) == "'(1 '(1 2))");
    assert!(p.full_string_eval("x".to_string())
            == Some(Object::Symbol("x".to_string()).to_garbobject()));
    // Let bindings (not checking scope)
    assert!(
            p.full_string_eval("(let ((x 23))
                                     (+ x 57))".to_string())
                .unwrap()
            == Object::Num(Number::Int(80)).to_garbobject()
    );
    assert!(
        format!(
            "{}",
            p.full_string_eval("(let ((x 23))
                                     (+ x 57))".to_string())
                .unwrap()
                .borrow()
        ) == "80"
    );
    assert!(
            p.full_string_eval("(let ((x 23) (y 57))
                                     (+ x y))".to_string())
                .unwrap()
            == Object::Num(Number::Int(80)).to_garbobject()
    );
    assert!(
            p.full_string_eval("(let ((f +))
                                     (let ((x 23) (y 57))
                                     (f x y)))".to_string())
                .unwrap()
            == Object::Num(Number::Int(80)).to_garbobject()
    );
    assert!(
            p.full_string_eval("(let ()
                                     (let ((x 23) (y 57))
                                     (+ x y)))".to_string())
                .unwrap()
            == Object::Num(Number::Int(80)).to_garbobject()
    );
}
