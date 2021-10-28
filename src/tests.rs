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
        p.string_eval("\"name\"".to_string()),
        Some(Rc::new(RefCell::new(Object::Atom(Atom::Str(
            "name".to_string()
        )))))
    );
    assert_eq!(
        p.string_eval("'name".to_string()),
        Some(Rc::new(RefCell::new(Object::Symbol("name".to_string()))))
    );
    assert_eq!(
        p.string_eval("#t".to_string()),
        Some(Rc::new(RefCell::new(Object::Bool(true))))
    );
    assert_eq!(
        p.string_eval("#f".to_string()),
        Some(Rc::new(RefCell::new(Object::Bool(false))))
    );
    assert!(!p.string_eval("cons".to_string()).is_none());
    assert!(
        format!(
            "{}",
            p.string_eval("'(1 2 \"name\")".to_string())
                .unwrap()
                .borrow()
        ) == "'(1 2 \"name\")"
    );
    assert!(
        format!(
            "{}",
            p.string_eval("'#(1 2 \"name\")".to_string())
                .unwrap()
                .borrow()
        ) == "'#(1 2 \"name\")"
    );
    assert!(
        format!(
            "{}",
            p.string_eval("(cons 1 '(2 3))".to_string())
                .unwrap()
                .borrow()
        ) == "'(1 2 3)"
    );
    assert!(
        format!(
            "{}",
            p.string_eval("(car '(1 2 3))".to_string())
                .unwrap()
                .borrow()
        ) == "1"
    );
    println!(
        "{}",
        p.string_eval("(cdr '(1 23 \"name\" #t))".to_string())
            .unwrap()
            .borrow()
    );
    assert!(
        format!(
            "{}",
            p.string_eval("(cdr '(1 23 \"name\" #t))".to_string())
                .unwrap()
                .borrow()
        ) == "'(23 \"name\" #t)"
    );
}
