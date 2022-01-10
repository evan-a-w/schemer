use crate::runtime::*;
use crate::types::*;
use crate::number::*;
use std::collections::HashMap;
#[allow(unused_imports)]
use crate::parser::*;
use gc_rs::gc::Gc;
use crate::env::*;

#[test]
pub fn test_int_parse() {
    let res = int_parser("123");
    assert!(res == Ok(("", 123)));
    let res = int_parser("-123");
    assert!(res == Ok(("", -123)));

    let res = int_parser("#b101001");
    assert!(res == Ok(("", 0b101001)));
    let res = int_parser("#B101001");
    assert!(res == Ok(("", 0b101001)));

    let res = int_parser("#xBEEF");
    assert!(res == Ok(("", 0xBEEF)));
    let res = int_parser("#Xbeef");
    assert!(res == Ok(("", 0xBEEF)));

    let res = int_parser("#o123");
    assert!(res == Ok(("", 0o123)));
    let res = int_parser("#o123");
    assert!(res == Ok(("", 0o123)));

    assert!(int_parser("penis").is_err());
}

#[test]
pub fn test_float_parse() {
    let res = float_parser("0.1");
    assert!(res == Ok(("", 0.1)));

    let res = float_parser(".1");
    assert!(res == Ok(("", 0.1)));

    let res = float_parser("0.");
    assert!(res == Ok(("", 0.)));

    let res = float_parser("0.5e2");
    assert!(res == Ok(("", 0.5e2)));

    let res = float_parser(".5e2");
    assert!(res == Ok(("", 0.5e2)));

    let res = float_parser("-.5e2");
    assert!(res == Ok(("", -0.5e2)));
}

#[test]
pub fn test_num_parse() {
    let res = ponga_parser("123");
    assert!(res == Ok(("", Ponga::Number(Number::Int(123)))));
    let res = ponga_parser("-123");
    assert!(res == Ok(("", Ponga::Number(Number::Int(-123)))));

    let res = ponga_parser("#b101001");
    assert!(res == Ok(("", Ponga::Number(Number::Int(0b101001)))));
    let res = ponga_parser("#B101001");
    assert!(res == Ok(("", Ponga::Number(Number::Int(0b101001)))));

    let res = ponga_parser("#xBEEF");
    assert!(res == Ok(("", Ponga::Number(Number::Int(0xbeef)))));
    let res = ponga_parser("#Xbeef");
    assert!(res == Ok(("", Ponga::Number(Number::Int(0xbeef)))));

    let res = ponga_parser("#o123");
    assert!(res == Ok(("", Ponga::Number(Number::Int(0o123)))));
    let res = ponga_parser("#O123");
    assert!(res == Ok(("", Ponga::Number(Number::Int(0o123)))));

    let res = ponga_parser("0.1");
    assert!(res == Ok(("", Ponga::Number(Number::Float(0.1)))));

    let res = ponga_parser(".1");
    assert!(res == Ok(("", Ponga::Number(Number::Float(0.1)))));

    let res = ponga_parser("0.");
    assert!(res == Ok(("", Ponga::Number(Number::Float(0.)))));

    let res = ponga_parser("0.5e2");
    assert!(res == Ok(("", Ponga::Number(Number::Float(0.5e2)))));

    let res = ponga_parser(".5e2");
    assert!(res == Ok(("", Ponga::Number(Number::Float(0.5e2)))));

    let res = ponga_parser("-.5e2");
    assert!(res == Ok(("", Ponga::Number(Number::Float(-0.5e2)))));
}

#[test]
pub fn test_string_parse() {
    let data = "\"abc\"";
    let result = string_parser(data);
    assert_eq!(result, Ok(("", Ponga::String(String::from("abc")))));

    let data = "\"tab:\\tafter tab, newline:\\nnew line, quote: \\\", emoji: \\u{1F602}, newline:\\nescaped whitespace: \\    abc\"";
    let result = string_parser(data);
    assert_eq!(
        result,
        Ok((
            "",
            Ponga::String(String::from("tab:\tafter tab, newline:\nnew line, quote: \", emoji: 😂, newline:\nescaped whitespace: abc"))
        ))
    );
}

#[test]
pub fn test_array_parser() {
    use crate::number::Number::{Float, Int};
    use Ponga::*;
    let res = ponga_parser("#(1   2.0  \"hi\"  )");
    assert!(
        res == Ok((
            "",
            Array(vec![
                Number(Int(1)),
                Number(Float(2.0)),
                String("hi".to_string())
            ])
        ))
    );
    let res = ponga_parser("#()");
    assert!(res == Ok(("", Array(vec![]))));
}

#[test]
pub fn test_list_parser() {
    use std::collections::LinkedList;
    use crate::number::Number::{Float, Int};
    use Ponga::*;
    let res = ponga_parser("'(1   2.0  \"hi\"  )");
    assert!(
        res == Ok((
            "",
            List(
                vec![Number(Int(1)), Number(Float(2.0)), String("hi".to_string())]
                    .into_iter()
                    .collect()
            )
        ))
    );
    let res = ponga_parser("'()");
    assert!(res == Ok(("", List(LinkedList::new()))));
}

#[test]
pub fn test_identifer_and_symbol_parser() {
    let rest = identifier_parser("abc");
    assert!(rest == Ok(("", Ponga::Identifier("abc".to_string()))));

    let res = ponga_parser("abc");
    assert!(res == Ok(("", Ponga::Identifier("abc".to_string()))));

    let res = ponga_parser("ab,#c");
    assert!(res == Ok(("", Ponga::Identifier("ab,#c".to_string()))));

    let res = ponga_parser(",ab,#c");
    assert!(res.is_err());

    let res = ponga_parser("#abc");
    assert!(res.is_err());

    let res = ponga_parser("'abc");
    assert!(res == Ok(("", Ponga::Symbol("abc".to_string()))));
}

#[test]
pub fn test_bool_parser() {
    assert!(ponga_parser("#t") == Ok(("", Ponga::True)));

    assert!(ponga_parser("#f") == Ok(("", Ponga::False)));

    assert!(bool_parser("#a").is_err());
    println!("{:?}", bool_parser("#tf"));
}

#[test]
pub fn test_char_parser() {
    assert!(ponga_parser("#\\B") == Ok(("", Ponga::Char('B'))));

    assert!(ponga_parser("#B").is_err());
}

#[test]
pub fn test_parser_sexpr() {
    use crate::number::Number::{Float, Int};
    use Ponga::*;
    let res = ponga_parser(
        "(define (foldl func accu alist)
       (if (null? alist)
         accu
         (foldl func (func (car alist) accu) (cdr alist))))",
    );
    assert_eq!(
        res,
        Ok((
            "",
            Sexpr(vec![
                Identifier("define".to_string()),
                Sexpr(vec![
                    Identifier("foldl".to_string()),
                    Identifier("func".to_string()),
                    Identifier("accu".to_string()),
                    Identifier("alist".to_string())
                ]),
                Sexpr(vec![
                    Identifier("if".to_string()),
                    Sexpr(vec![
                        Identifier("null?".to_string()),
                        Identifier("alist".to_string())
                    ]),
                    Identifier("accu".to_string()),
                    Sexpr(vec![
                        Identifier("foldl".to_string()),
                        Identifier("func".to_string()),
                        Sexpr(vec![
                            Identifier("func".to_string()),
                            Sexpr(vec![
                                Identifier("car".to_string()),
                                Identifier("alist".to_string())
                            ]),
                            Identifier("accu".to_string())
                        ]),
                        Sexpr(vec![
                            Identifier("cdr".to_string()),
                            Identifier("alist".to_string())
                        ])
                    ])
                ])
            ])
        ))
    );

    assert!(ponga_parser("").is_err());

    let res = pongascript_parser(
        "(foldl cons '() '(1 2 3 4 5))
     (define (foldl func accu alist)
       (if (null? alist)
         accu
         (foldl func (func (car alist) accu) (cdr alist))))

     (foldl cons '() '(1 2 3 4 5))",
    );
    assert_eq!(
        res,
        Ok((
            "",
            vec![
                Sexpr(vec![
                    Identifier("foldl".to_string()),
                    Identifier("cons".to_string()),
                    List(vec![].into_iter().collect()),
                    List(
                        vec![
                            Number(Int(1)),
                            Number(Int(2)),
                            Number(Int(3)),
                            Number(Int(4)),
                            Number(Int(5))
                        ]
                        .into_iter()
                        .collect()
                    )
                ]),
                Sexpr(vec![
                    Identifier("define".to_string()),
                    Sexpr(vec![
                        Identifier("foldl".to_string()),
                        Identifier("func".to_string()),
                        Identifier("accu".to_string()),
                        Identifier("alist".to_string())
                    ]),
                    Sexpr(vec![
                        Identifier("if".to_string()),
                        Sexpr(vec![
                            Identifier("null?".to_string()),
                            Identifier("alist".to_string())
                        ]),
                        Identifier("accu".to_string()),
                        Sexpr(vec![
                            Identifier("foldl".to_string()),
                            Identifier("func".to_string()),
                            Sexpr(vec![
                                Identifier("func".to_string()),
                                Sexpr(vec![
                                    Identifier("car".to_string()),
                                    Identifier("alist".to_string())
                                ]),
                                Identifier("accu".to_string())
                            ]),
                            Sexpr(vec![
                                Identifier("cdr".to_string()),
                                Identifier("alist".to_string())
                            ])
                        ])
                    ])
                ]),
                Sexpr(vec![
                    Identifier("foldl".to_string()),
                    Identifier("cons".to_string()),
                    List(vec![].into_iter().collect()),
                    List(
                        vec![
                            Number(Int(1)),
                            Number(Int(2)),
                            Number(Int(3)),
                            Number(Int(4)),
                            Number(Int(5))
                        ]
                        .into_iter()
                        .collect()
                    )
                ])
            ]
        ))
    );
}

#[test]
pub fn test_super_basic_run() {
    let parsed = pongascript_parser("
    (define i (foldl cons '() '(1 2 3 4 5)))
    (display i)
    (equal? i '('('('('('() 1) 2) 3) 4) 5))
    ")
    .unwrap();
    let mut runtime = Runtime::new();
    let evald = parsed
        .1
        .into_iter()
        .map(|x| runtime.eval(x))
        .collect::<Vec<RunRes<Ponga>>>();
    println!("{:?}", evald);
    assert!(evald[2] == Ok(Ponga::True));
}

#[test]
pub fn test_basic_run() {
    let parsed = pongascript_parser("
    (foldl cons '() '(1 2 3 4 5))
     (define (foldl func accu alist)
       (if (null? alist)
         accu
         (foldl func (func (car alist) accu) (cdr alist))))

     (define i (foldl cons '() '(1 2 3 4 5)))
     (display i)
     (equal? i '(5 4 3 2 1))
    ")
    .unwrap();
    let mut runtime = Runtime::new();
    let evald = parsed
        .1
        .into_iter()
        .map(|x| runtime.eval(x))
        .collect::<Vec<RunRes<Ponga>>>();
    println!("{:?}", evald);
    assert!(evald[4] == Ok(Ponga::True));


}

#[test]
pub fn test_closures() {
    let parsed = pongascript_parser("
(define count
   (let ((next 0))
     (lambda ()
       (let ((v next))
         (begin
             (set! next (+ next 1))
             v)))))
(count)
(count)
    ")
    .unwrap();
    let mut runtime = Runtime::new();
    let evald = parsed
        .1
        .into_iter()
        .map(|x| runtime.eval(x))
        .collect::<Vec<RunRes<Ponga>>>();
    println!("{:?}", evald);
    assert!(evald[2] == Ok(Ponga::Number(Number::Int(1))));
}

#[test]
pub fn test_closures_better() {
    let parsed = pongascript_parser("
(let ((y 1))
     (begin 
         (define (func) (set! y (+ y 8)))
         (func)
         (func)
         (display y)
         y))
    ")
    .unwrap();
    let mut runtime = Runtime::new();
    let evald = parsed
        .1
        .into_iter()
        .map(|x| runtime.eval(x))
        .collect::<Vec<RunRes<Ponga>>>();
    println!("{:?}", evald);
    assert!(evald[0] == Ok(Ponga::Number(Number::Int(17))));
}

#[test]
pub fn test_vec_to_list() {
    use Ponga::*;
    use RuntimeErr::*;

    let parsed = pongascript_parser("
(define vec #(1 2 3 4 5))
(define list (vector->list vec))
(display list)
(eqv? list '(1 2 3 4 5))
    ")
    .unwrap();
    let mut runtime = Runtime::new();
    let evald = parsed
        .1
        .into_iter()
        .map(|x| runtime.eval(x))
        .collect::<Vec<RunRes<Ponga>>>();
    println!("{:?}", evald);
    assert!(evald[3] == Ok(Ponga::True));


}

#[test]
pub fn test_list_to_vec() {
    use Ponga::*;
    use RuntimeErr::*;

    let parsed = pongascript_parser("
(define list '(1 2 3 4 5))
(define vec (list->vector list))
(display vec)
(eqv? vec #(1 2 3 4 5))
    ")
    .unwrap();
    let mut runtime = Runtime::new();
    let evald = parsed
        .1
        .into_iter()
        .map(|x| runtime.eval(x))
        .collect::<Vec<RunRes<Ponga>>>();
    println!("{:?}", evald);
    assert!(evald[3] == Ok(Ponga::True));


}

#[test]
pub fn test_simple_macro() {
    let program = "
(let ((x 1))
     (while (< x 10)
            (begin
                (set! x (+ x 7))))
     x)
    ";
    let mut prog_res = run_str(program).unwrap();
    let res = prog_res.pop().unwrap().unwrap();
    assert!(res == Ponga::Number(Number::Int(15)));

    let program = "
(let ((x 0))
     (for i in '(1 2 3)
         (set! x (+ x i)))
     x)
    ";
    let mut prog_res = run_str(program).unwrap();
    let res = prog_res.pop().unwrap().unwrap();
    println!("{:?}", res);
    assert!(res == Ponga::Number(Number::Int(6)));
}

#[test]
pub fn test_macro_closures() {
    let program = "
(define count (let ((next 0)) (lambda () (let ((v next)) (begin (set! next (+ next 1)) v)))))
(for i in '(1 2 3)
           (count))
(count)
    ";
    let mut prog_res = run_str(program).unwrap();
    let res = prog_res.pop().unwrap().unwrap();
    assert!(res == Ponga::Number(Number::Int(3)));
}

#[test]
pub fn test_euler_p3() {
    let program = "
(define (prime? n)
    (let ((lim (sqrt n))
          (go (lambda (curr)
              (if (<= curr lim)
                  (if (= 0 (modulo n curr)) 
                      #f
                      (go (+ curr 1)))
                  #t))))
         (go 2)))

(define curr 600851475143)

(define (div-if-prime x)
        (if (and (prime? x) (= 0 (modulo curr x)))
            (set! curr (/ curr x))
            '()))

(let ((x 2))
     (while (< x curr)
            (begin
                (div-if-prime x)
                (set! x (+ x 1)))))

curr
    ";
    let mut prog_res = run_str(program).unwrap();
    let res = prog_res.pop().unwrap().unwrap();
    println!("{}", res);
    assert!(res == Ponga::Number(Number::Int(6857)));
}
