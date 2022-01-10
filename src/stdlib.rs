use crate::runtime::*;
use crate::types::*;
use crate::number::*;
use std::collections::LinkedList;
use std::collections::HashMap;
use gc_rs::{Gc, GcRefMut};

pub const KEYWORDS: &[&str] = &[
    "true",
    "false",
    "if",
    "let",
    "define",
    "lambda",
    "echo",
    "set!",
    "set-deref!",
    "let-deref",
    "mac",
    "defmacro",
    "begin", 
    "echo",
    "quote",
    "sym->id",
    "$DELAY",
    "$EVAL-FLIP-EVAL",
    "eval.code<->data.eval",
    "eval.data<->code.eval",
    "$FLIP-EVAL",
    "code<->data.eval",
    "data<->code.eval",
    "$FLIP",
    "code<->data",
    "data<->code",
    "$EVAL",
    "eval",
    "copy",
    "open-lambda",
];

pub const FUNCS: &[(&str, fn(&mut Runtime, Vec<Ponga>) -> RunRes<Ponga>)] = &[
    ("cons", cons),
    ("null?", null),
    ("car", car),
    ("cdr", cdr),
    ("vector?", vector_query),
    ("+", plus),
    ("-", minus),
    ("*", times),
    ("/", div),
    ("eq?", eq),
    ("eqv?", teq),
    ("equal?", teq),
    ("=", peq),
    ("or", or),
    ("and", and),
    ("not", not),
    ("<", lt),
    ("<=", le),
    (">=", ge),
    (">", gt),
    ("modulo", modulus),
    ("cond", cond),
    ("display", disp),
    ("map", map_),
    ("foldl", foldl),
    ("foldr", foldr),
    ("vector-length", vector_len),
    ("vector-ref", vector_ref),
    ("vector-append!", vector_append),
    ("list->map", list_to_map),
    ("map-contains?", map_contains),
    ("map-ref", map_ref),
    ("map-set!", map_set),
    ("floor", floor),
    ("ceiling", ceiling),
    ("sqrt", sqrt),
    ("string->list", string_to_list),
    ("list->string", list_to_string),
    ("show", show),
    ("len", len),
    ("reverse", reverse),
];

pub fn args_assert_len(args: &Vec<Ponga>, len: usize, name: &str) -> RunRes<()> {
    if args.len() != len {
        return Err(RuntimeErr::TypeError(format!(
            "{} requires {} arguments",
            name, len
        )));
    }
    Ok(())
}

pub fn args_assert_gt(args: &Vec<Ponga>, len: usize, name: &str) -> RunRes<()> {
    if args.len() <= len {
        return Err(RuntimeErr::TypeError(format!(
            "{} requires at least {} arguments",
            name, len
        )));
    }
    Ok(())
}

pub fn bool_to_ponga(b: bool) -> Ponga {
    if b {
        Ponga::True
    } else {
        Ponga::False
    }
}

pub fn cons(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 2, "cons")?;
    let snd = args.pop().unwrap();
    let first = args.pop().unwrap();
    match snd {
        Ponga::Ref(mut id) => {
            if id.is_list() {
                let mut mut_id = id.borrow_mut().unwrap();
                let list = mut_id.get_list()?;
                list.push_front(first);
                drop(mut_id);
                Ok(Ponga::Ref(id))
            } else {
                Ok(Ponga::Ref(
                    Gc::new(Ponga::List([first, Ponga::Ref(id)].into_iter().collect()))
                ))
            }
        }
        _ => {
            Ok(Ponga::Ref(
                Gc::new(Ponga::List([first, snd].into_iter().collect()))
            ))
        }
    }
}

pub fn null(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 1, "null?")?;
    let arg = args.pop().unwrap();
    match arg {
        Ponga::List(list) => Ok(bool_to_ponga(list.is_empty())),
        Ponga::Null => Ok(Ponga::True),
        Ponga::Ref(id) => {
            match *id {
                Ponga::List(ref list) => {
                    let res = bool_to_ponga(list.is_empty());
                    Ok(res)
                }
                Ponga::Null => Ok(Ponga::True),
                _ => Err(RuntimeErr::TypeError(format!(
                    "null? requires a list or null (not {:?})", id
                ))),
            }
        }
        Ponga::Identifier(id) => {
            let obj = runtime.get_identifier_obj_ref(&id)?;
            match obj {
                Ponga::List(list) => {
                    let res = bool_to_ponga(list.is_empty());
                    drop(obj);
                    Ok(res)
                }
                Ponga::Null => Ok(Ponga::True),
                _ => Err(RuntimeErr::TypeError(format!(
                    "null? requires a list or null (not {:?})", obj
                ))),
            }
        }
        _ => Err(RuntimeErr::TypeError(
            format!("null? requires a list (not {:?})", arg)
        )),
    }
}

pub fn car(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 1, "car")?;
    let arg = args.pop().unwrap();
    match arg {
        Ponga::Ref(id) => {
            match *id {
                Ponga::List(ref list) => Ok(list
                    .iter()
                    .next()
                    .ok_or(RuntimeErr::TypeError(format!("car of empty list")))?
                    .clone()),
                _ => Err(RuntimeErr::TypeError(format!("car requires a list"))),
            }
        }
        _ => Err(RuntimeErr::TypeError(format!("car requires a list"))),
    }
}

pub fn map_(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 2, "map")?;
    let iterable = args.pop().unwrap();
    let func = args.pop().unwrap();
    if !func.is_func() {
        let mut fail = true;
        if let Ponga::Ref(id) = &func {
            if id.is_func() {
                fail = false;
            }
        }
        if fail {
            return Err(RuntimeErr::TypeError(format!(
                "first argument to map must be a function"
            )));
        }
    }
    
    match iterable {
        Ponga::Ref(id) => {
            match *id {
                Ponga::List(ref list) => {
                    let cloned: LinkedList<Ponga> = list.clone();
                    drop(list);
                    let mut res = LinkedList::new();
                    for i in cloned.into_iter() {
                        let sexpr = Ponga::Sexpr(vec![func.clone(), i]);
                        res.push_back(runtime.eval(sexpr)?);
                    }
                    Ok(Ponga::Ref(Gc::new(Ponga::List(res))))
                }
                Ponga::Array(ref arr) => {
                    let cloned: Vec<Ponga> = arr.clone();
                    drop(arr);
                    let mut res = Vec::new();
                    for i in cloned.into_iter() {
                        let sexpr = Ponga::Sexpr(vec![func.clone(), i]);
                        res.push(runtime.eval(sexpr)?);
                    }
                    Ok(Ponga::Ref(Gc::new(Ponga::Array(res))))
                }
                Ponga::Object(ref o) => {
                    let cloned: HashMap<String, Ponga> = o.clone();
                    drop(o);
                    let mut res = HashMap::new();
                    for (k, v) in cloned.into_iter() {
                        let sexpr = Ponga::Sexpr(vec![func.clone(), v]);
                        res.insert(k, runtime.eval(sexpr)?);
                    }
                    Ok(Ponga::Ref(Gc::new(Ponga::Object(res))))
                }
                _ => Err(RuntimeErr::TypeError(format!("map requires an iterable"))),
            }
        }
        _ => Err(RuntimeErr::TypeError(format!("map requires an iterable"))),
    }
}

pub fn vector_query(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 1, "vector?")?;
    let arg = args.pop().unwrap();
    Ok(bool_to_ponga(arg.is_vector()))
}

pub fn cdr(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 1, "cdr")?;
    let arg = args.pop().unwrap();
    match arg {
        Ponga::Ref(id) => {
            match *id {
                Ponga::List(ref list) => Ok(Ponga::List(list.iter().skip(1).cloned().collect())),
                _ => Err(RuntimeErr::TypeError(format!("car requires a list"))),
            }
        }
        _ => Err(RuntimeErr::TypeError(format!("car requires a list"))),
    }
}

pub fn plus(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 2, "+")?;
    let snd = args.pop().unwrap();
    let fst = args.pop().unwrap();
    match fst {
        Ponga::Number(n) => match snd {
            Ponga::Number(m) => Ok(Ponga::Number(n.plus(m))),
            _ => Err(RuntimeErr::TypeError(format!("+ requires two numbers"))),
        },
        _ => Err(RuntimeErr::TypeError(format!("+ requires two numbers"))),
    }
}

pub fn minus(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 2, "-")?;
    let snd = args.pop().unwrap();
    let fst = args.pop().unwrap();
    match fst {
        Ponga::Number(n) => match snd {
            Ponga::Number(m) => Ok(Ponga::Number(n.minus(m))),
            _ => Err(RuntimeErr::TypeError(format!("- requires two numbers"))),
        },
        _ => Err(RuntimeErr::TypeError(format!("- requires two numbers"))),
    }
}

pub fn times(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 2, "*")?;
    let snd = args.pop().unwrap();
    let fst = args.pop().unwrap();
    match fst {
        Ponga::Number(n) => match snd {
            Ponga::Number(m) => Ok(Ponga::Number(n.times(m))),
            _ => Err(RuntimeErr::TypeError(format!("* requires two numbers"))),
        },
        _ => Err(RuntimeErr::TypeError(format!("* requires two numbers"))),
    }
}

pub fn div(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 2, "/")?;
    let snd = args.pop().unwrap();
    let fst = args.pop().unwrap();
    match fst {
        Ponga::Number(n) => match snd {
            Ponga::Number(m) => Ok(Ponga::Number(n.div(m))),
            _ => Err(RuntimeErr::TypeError(format!("/ requires two numbers"))),
        },
        _ => Err(RuntimeErr::TypeError(format!("/ requires two numbers"))),
    }
}

pub fn eq(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 2, "eq?")?;
    let snd = args.pop().unwrap();
    let fst = args.pop().unwrap();
    Ok(bool_to_ponga(fst == snd))
}

pub fn teq(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 2, "eq?")?;
    let snd = args.pop().unwrap();
    let fst = args.pop().unwrap();
    Ok(bool_to_ponga(
        format!("{}", fst) == format!("{}", snd)
    ))
}

pub fn peq(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 2, "eq?")?;
    let snd = args.pop().unwrap();
    let fst = args.pop().unwrap();
    match fst {
        Ponga::Number(n) => match snd {
            Ponga::Number(m) => Ok(bool_to_ponga(n.eq(m))),
            _ => Err(RuntimeErr::TypeError(format!("= requires two numbers"))),
        },
        _ => Err(RuntimeErr::TypeError(format!("= requires two numbers"))),
    }
}

pub fn or(runtime: &mut Runtime, args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_gt(&args, 0, "or")?;
    for i in args.into_iter() {
        if i != Ponga::False {
            return Ok(i);
        }
    }
    Ok(Ponga::False)
}

pub fn and(runtime: &mut Runtime, args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_gt(&args, 0, "and")?;
    let len = args.len();
    for (i, v) in args.into_iter().enumerate() {
        if v == Ponga::False {
            return Ok(Ponga::False);
        }
        if i == len - 1 {
            return Ok(v);
        }
    }
    Ok(Ponga::Null)
}

pub fn not(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 1, "not")?;
    let fst = args.pop().unwrap().to_bool()?;
    Ok(bool_to_ponga(!fst))
}

pub fn ge(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 2, ">=")?;
    let snd = args.pop().unwrap().to_number()?;
    let fst = args.pop().unwrap().to_number()?;
    Ok(bool_to_ponga(fst.ge(snd)))
}

pub fn gt(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 2, ">")?;
    let snd = args.pop().unwrap().to_number()?;
    let fst = args.pop().unwrap().to_number()?;
    Ok(bool_to_ponga(fst.gt(snd)))
}

pub fn lt(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 2, "<")?;
    let snd = args.pop().unwrap().to_number()?;
    let fst = args.pop().unwrap().to_number()?;
    Ok(bool_to_ponga(fst.lt(snd)))
}

pub fn le(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 2, "<=")?;
    let snd = args.pop().unwrap().to_number()?;
    let fst = args.pop().unwrap().to_number()?;
    Ok(bool_to_ponga(fst.le(snd)))
}

pub fn modulus(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 2, "<=")?;
    let snd = args.pop().unwrap().to_number()?;
    let fst = args.pop().unwrap().to_number()?;
    Ok(Ponga::Number(fst.modulus(snd)))
}

pub fn cond(runtime: &mut Runtime, args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_gt(&args, 1, "cond")?;
    for arg in args.into_iter() {
        match arg {
            Ponga::Sexpr(v) => {
                if v.len() != 2 {
                    return Err(RuntimeErr::TypeError(format!(
                        "Args to cond must be S-Expr pairs"
                    )));
                }
                let mut iter = v.into_iter();
                let first = iter.next().unwrap();
                match &first {
                    Ponga::Identifier(s) => {
                        if s == "else" {
                            return runtime.eval(iter.next().unwrap());
                        }
                    }
                    _ => (),
                }
                let cond = runtime.eval(first)?;
                match cond {
                    Ponga::True => return runtime.eval(iter.next().unwrap()),
                    _ => continue,
                }
            }
            _ => {
                return Err(RuntimeErr::TypeError(format!(
                    "cond requires a S-Expr args"
                )))
            }
        }
    }
    Ok(Ponga::Null)
}

pub fn disp(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&args, 1, "display")?;
    let arg = runtime.eval(args.pop().unwrap())?;
    println!("{}", arg);
    Ok(Ponga::Null)
}

pub fn foldl(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 3, "foldl")?;
    let iterable = args.pop().unwrap();
    let mut acc = args.pop().unwrap();
    let func = args.pop().unwrap();
    if !func.is_func() {
        return Err(RuntimeErr::TypeError(format!(
            "first argument to foldl must be a function"
        )));
    }
    
    match iterable {
        Ponga::Ref(id) => {
            match *id {
                Ponga::List(ref list) => {
                    let cloned: LinkedList<Ponga> = list.clone();
                    for i in cloned.into_iter() {
                        let sexpr = Ponga::Sexpr(vec![func.clone(), acc, i]);
                        acc = runtime.eval(sexpr)?;
                    }
                    Ok(acc)
                }
                Ponga::Array(ref arr) => {
                    let cloned: Vec<Ponga> = arr.clone();
                    for i in cloned.into_iter() {
                        let sexpr = Ponga::Sexpr(vec![func.clone(), acc, i]);
                        acc = runtime.eval(sexpr)?;
                    }
                    Ok(acc)
                }
                Ponga::Object(ref o) => {
                    let cloned: HashMap<String, Ponga> = o.clone();
                    for (_, v) in cloned.into_iter() {
                        let sexpr = Ponga::Sexpr(vec![func.clone(), acc, v]);
                        acc = runtime.eval(sexpr)?;
                    }
                    Ok(acc)
                }
                _ => Err(RuntimeErr::TypeError(format!("foldl requires an iterable"))),
            }
        }
        _ => Err(RuntimeErr::TypeError(format!("foldl requires an iterable"))),
    }
}

pub fn foldr(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 3, "foldl")?;
    let iterable = args.pop().unwrap();
    let mut acc = args.pop().unwrap();
    let func = args.pop().unwrap();
    if !func.is_func() {
        return Err(RuntimeErr::TypeError(format!(
            "first argument to foldr must be a function"
        )));
    }
    
    match iterable {
        Ponga::Ref(id) => {
            match *id {
                Ponga::List(ref list) => {
                    let cloned: LinkedList<Ponga> = list.clone();
                    for i in cloned.into_iter().rev() {
                        let sexpr = Ponga::Sexpr(vec![func.clone(), i, acc]);
                        acc = runtime.eval(sexpr)?;
                    }
                    Ok(acc)
                }
                Ponga::Array(ref arr) => {
                    let cloned: Vec<Ponga> = arr.clone();
                    for i in cloned.into_iter().rev() {
                        let sexpr = Ponga::Sexpr(vec![func.clone(), i, acc]);
                        acc = runtime.eval(sexpr)?;
                    }
                    Ok(acc)
                }
                Ponga::Object(ref o) => {
                    let cloned: HashMap<String, Ponga> = o.clone();
                    for (_, v) in cloned.into_iter() {
                        let sexpr = Ponga::Sexpr(vec![func.clone(), v, acc]);
                        acc = runtime.eval(sexpr)?;
                    }
                    Ok(acc)
                }
                _ => Err(RuntimeErr::TypeError(format!("foldr requires an iterable"))),
            }
        }
        _ => Err(RuntimeErr::TypeError(format!("foldr requires an iterable"))),
    }
}

pub fn vector_len(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&args, 1, "vector-length")?;
    let arg = args.pop().unwrap();
    match arg {
        Ponga::Ref(id) => {
            match *id {
                Ponga::Array(ref v) => Ok(Ponga::Number(Number::Int(v.len() as isize))),
                _ => Err(RuntimeErr::TypeError(format!("vector-length requires a vector"))),
            } 
        }
        _ => Err(RuntimeErr::TypeError(format!("vector-length requires a vector"))),
    }
}

pub fn vector_ref(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&args, 2, "vector-length")?;
    let n = args.pop().unwrap();
    if !n.is_number() {
        return Err(RuntimeErr::TypeError(format!("vector-ref requires a number")));
    }
    let arg = runtime.eval(args.pop().unwrap())?;
    match arg {
        Ponga::Ref(id) => {
            match *id {
                Ponga::Array(ref v) => {
                    let n = n.get_number()?.to_isize();
                    if n < 0 || n >= v.len() as isize {
                        return Err(RuntimeErr::TypeError(format!(
                            "vector-ref index out of bounds"
                        )));
                    }
                    Ok(v[n as usize].clone())
                }
                _ => Err(RuntimeErr::TypeError(format!("vector-length requires a vector"))),
            } 
        }
        _ => Err(RuntimeErr::TypeError(format!("vector-length requires a vector"))),
    }
}

pub fn vector_append(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&args, 2, "vector-append!")?;
    let n = args.pop().unwrap();
    let arg = args.pop().unwrap();
    match arg {
        Ponga::Ref(mut id) => {
            let mut mut_ref = id.borrow_mut().unwrap();
            match *mut_ref {
                Ponga::Array(ref mut v) => {
                    v.push(n);
                    drop(mut_ref);
                    Ok(Ponga::Ref(id))
                }
                _ => Err(RuntimeErr::TypeError(format!("vector-append! requires a vector"))),
            } 
        }
        _ => Err(RuntimeErr::TypeError(format!("vector-append! requires a vector"))),
    }
}

pub fn floor(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 1, "floor")?;
    let fst = args.pop().unwrap();
    match fst {
        Ponga::Number(n) => {
            Ok(Ponga::Number(n.floor()))
        }
        _ => Err(RuntimeErr::TypeError(format!("floor requires a number"))),
    }
}

pub fn ceiling(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 1, "ceiling")?;
    let fst = args.pop().unwrap();
    match fst {
        Ponga::Number(n) => {
            Ok(Ponga::Number(n.ceiling()))
        }
        _ => Err(RuntimeErr::TypeError(format!("ceiling requires a number"))),
    }
}

pub fn sqrt(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 1, "sqrt")?;
    let fst = args.pop().unwrap();
    match fst {
        Ponga::Number(n) => {
            Ok(Ponga::Number(n.sqrt()))
        }
        _ => Err(RuntimeErr::TypeError(format!("sqrt requires a number"))),
    }
}

pub fn map_ref(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&args, 2, "map-ref")?;
    let n = args.pop().unwrap();
    let arg = args.pop().unwrap();
    match arg {
        Ponga::Ref(id) => {
            match *id {
                Ponga::Object(ref o) => {
                    let s = format!("{}", n);
                    let v = o.get(s.as_str()).unwrap_or(&Ponga::Null);
                    Ok(v.clone())
                }
                _ => Err(RuntimeErr::TypeError(format!("map-ref requires a map"))),
            } 
        }
        _ => Err(RuntimeErr::TypeError(format!("map-ref requires a map"))),
    }
}

pub fn map_set(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&args, 3, "map-set!")?;
    let val = args.pop().unwrap();
    let key = args.pop().unwrap();
    let arg = args.pop().unwrap();
    match arg {
        Ponga::Ref(mut id) => {
            let s = format!("{}", key);
            let mut mut_ref = id.borrow_mut().unwrap();
            match *mut_ref {
                Ponga::Object(ref mut o) => {
                    o.insert(s, val);
                    drop(mut_ref);
                    Ok(Ponga::Ref(id))
                }
                _ => Err(RuntimeErr::TypeError(format!("map-set! requires a map"))),
            } 
        }
        _ => Err(RuntimeErr::TypeError(format!("map-set! requires a map"))),
    }
}

pub fn map_contains(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&args, 2, "map-contains?")?;
    let n = args.pop().unwrap();
    let arg = args.pop().unwrap();
    match arg {
        Ponga::Ref(id) => {
            match *id {
                Ponga::Object(ref o) => {
                    let s = format!("{}", n);
                    let v = o.get(s.as_str()).is_some();
                    Ok(bool_to_ponga(v))
                }
                _ => Err(RuntimeErr::TypeError(format!("map-contains? requires a map"))),
            } 
        }
        _ => Err(RuntimeErr::TypeError(format!("map-contains? requires a map"))),
    }
}

fn insert_list_pair_into_map(runtime: &Runtime, map: &mut HashMap<String, Ponga>,
                             pair: &Ponga) -> RunRes<()> {
    match pair {
        Ponga::Ref(id) => {
            match **id {
                Ponga::List(ref list) => {
                    if list.len() == 2 {
                        let mut iter = list.iter();
                        let key = iter.next().unwrap();
                        let val = iter.next().unwrap();
                        let s = format!("{}", key);
                        map.insert(s, val.clone());
                        return Ok(());
                    }
                    Err(RuntimeErr::TypeError(format!("list->map requires list of list pairs")))
                }
                _ => Err(RuntimeErr::TypeError(format!("list->map requires list of list pairs")))
            }
        }
        _ => {
            Err(RuntimeErr::TypeError(format!("list->map requires list of list pairs")))
        }
    }

}

pub fn list_to_map(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 1, "list->map")?;
    let list = args.pop().unwrap();
    
    match list {
        Ponga::Ref(id) => {
            match *id {
                Ponga::List(ref list) => {
                    let mut map = HashMap::new();
                    for item in list {
                        insert_list_pair_into_map(runtime, &mut map, &item)?;
                    }
                    Ok(Ponga::Object(map))
                }
                _ => Err(RuntimeErr::TypeError(format!("list->map requires a list"))),
            }
        }
        _ => Err(RuntimeErr::TypeError(format!("list->map requires a list"))),
    }
}

pub fn string_to_list(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 1, "string->list")?;
    let s = args.pop().unwrap();
    match s {
        Ponga::String(s) => {
            let mut list = LinkedList::new();
            for c in s.chars() {
                list.push_back(Ponga::Char(c));
            }
            Ok(Ponga::List(list))
        }
        _ => Err(RuntimeErr::TypeError(format!("string->list requires a string"))),
    }
}

pub fn list_to_string(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 1, "list->string")?;
    let s = args.pop().unwrap();
    match s {
        Ponga::Ref(id) => {
            let chars = id.get_list_ref()?.iter();
            let mut string = String::new();
            for ch in chars {
                string.push(ch.char_to_char()?);
            }
            Ok(Ponga::String(string))
        }
        _ => Err(RuntimeErr::TypeError(format!("string->list requires a string"))),
    }
}

pub fn show(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 1, "show")?;
    let s = args.pop().unwrap();
    Ok(Ponga::String(format!("{}", s)))
}

pub fn len(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 1, "len")?;
    let iterable = args.pop().unwrap();
    
    match iterable {
        Ponga::Ref(id) => {
            match *id {
                Ponga::List(ref list) => Ok(Ponga::Number(Number::Int(list.len() as isize))),
                Ponga::Array(ref arr) => Ok(Ponga::Number(Number::Int(arr.len() as isize))),
                Ponga::Object(ref o) => Ok(Ponga::Number(Number::Int(o.len() as isize))),
                _ => Ok(Ponga::Number(Number::Int(1))),
            }
        }
        Ponga::String(s) => Ok(Ponga::Number(Number::Int(s.len() as isize))),
        _ => Ok(Ponga::Number(Number::Int(1))),
    }
}

pub fn reverse(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 1, "len")?;
    let iterable = args.pop().unwrap();
    
    match iterable {
        Ponga::Ref(id) => {
            match *id {
                Ponga::List(ref list) => {
                    let res = Ponga::List(list.iter().rev().cloned().collect());
                    Ok(Ponga::Ref(Gc::new(res)))
                }
                Ponga::Array(ref arr) => {
                    let res = Ponga::Array(arr.iter().rev().cloned().collect());
                    Ok(Ponga::Ref(Gc::new(res)))
                }
                _ => Ok(Ponga::Ref(id)),
            }
        }
        Ponga::String(s) => Ok(Ponga::String(s.chars().rev().collect())),
        _ => Ok(Ponga::Number(Number::Int(1))),
    }
}
