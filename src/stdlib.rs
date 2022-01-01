use crate::runtime::*;
use crate::types::*;
use crate::number::*;
use std::collections::LinkedList;
use std::collections::HashMap;

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
    ("begin", begin),
    ("display", disp),
    ("let", let_),
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
];

pub fn transform_args(runtime: &mut Runtime, args: Vec<Ponga>) -> RunRes<Vec<Ponga>> {
    let mut res = Vec::new();
    for arg in args {
        res.push(runtime.id_or_ref_peval(arg)?);
    }
    Ok(res)
}

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
    let mut args = transform_args(runtime, args)?;
    let snd = args.pop().unwrap();
    let first = args.pop().unwrap();
    match snd {
        Ponga::Ref(id) => {
            let mut taken_obj =
                runtime
                    .gc
                    .take_id(id)
                    .ok_or(RuntimeErr::ReferenceError(format!(
                        "Reference {} not found",
                        id
                    )))?;
            if taken_obj.is_list() {
                let list = taken_obj.get_list()?;
                list.push_front(first);
                runtime.gc.add_obj_with_id(taken_obj, id);
                Ok(Ponga::Ref(id))
            } else {
                Ok(runtime
                    .gc
                    .ponga_into_gc_ref(Ponga::List([first, Ponga::Ref(id)].into_iter().collect())))
            }
        }
        _ => Ok(runtime
            .gc
            .ponga_into_gc_ref(Ponga::List([first, snd].into_iter().collect()))),
    }
}

pub fn null(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 1, "null?")?;
    let mut args = transform_args(runtime, args)?;
    let arg = args.pop().unwrap();
    println!("NULL ARG: {} ({:?})", runtime.ponga_to_string(&arg), arg);
    match arg {
        Ponga::List(list) => Ok(bool_to_ponga(list.is_empty())),
        Ponga::Null => Ok(Ponga::True),
        Ponga::Ref(id) => {
            let mut obj = runtime.get_id_obj(id)?.borrow_mut().unwrap();
            let r = obj.inner();
            match r {
                Ponga::List(list) => {
                    let res = bool_to_ponga(list.is_empty());
                    drop(obj);
                    Ok(res)
                }
                Ponga::Null => Ok(Ponga::True),
                _ => Err(RuntimeErr::TypeError(format!(
                    "null? requires a list or null (not {:?})", r
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
    let mut args = transform_args(runtime, args)?;
    let arg = args.pop().unwrap();
    match arg {
        Ponga::List(list) => Ok(list
            .iter()
            .next()
            .ok_or(RuntimeErr::TypeError(format!("car of empty list")))?
            .clone()),
        Ponga::Ref(id) => {
            let mut obj = runtime.get_id_obj(id)?.borrow_mut().unwrap();
            let r = obj.inner();
            match r {
                Ponga::List(list) => Ok(list
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
    let mut args = transform_args(runtime, args)?;
    let iterable = args.pop().unwrap();
    let func = args.pop().unwrap();
    if !func.is_func() {
        let mut fail = true;
        if let Ponga::Ref(id) = &func {
            let obj = runtime.get_id_obj_ref(*id)?;
            let obj_ref = obj.borrow().unwrap();
            if obj_ref.inner().is_func() {
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
            let obj = runtime.get_id_obj_ref(id)?.borrow().unwrap();
            let r = obj.inner();
            match r {
                Ponga::List(list) => {
                    let cloned: LinkedList<Ponga> = list.clone();
                    drop(list);
                    drop(obj);
                    let mut res = LinkedList::new();
                    for i in cloned.into_iter() {
                        let sexpr = Ponga::Sexpr(vec![func.clone(), i]);
                        res.push_back(runtime.eval(sexpr)?);
                    }
                    let res_ref = runtime.gc.ponga_into_gc_ref(Ponga::List(res));
                    Ok(res_ref)
                }
                Ponga::Array(arr) => {
                    let cloned: Vec<Ponga> = arr.clone();
                    drop(arr);
                    drop(obj);
                    let mut res = Vec::new();
                    for i in cloned.into_iter() {
                        let sexpr = Ponga::Sexpr(vec![func.clone(), i]);
                        res.push(runtime.eval(sexpr)?);
                    }
                    let res_ref = runtime.gc.ponga_into_gc_ref(Ponga::Array(res));
                    Ok(res_ref)
                }
                Ponga::Object(o) => {
                    let cloned: HashMap<String, Ponga> = o.clone();
                    drop(o);
                    drop(obj);
                    let mut res = HashMap::new();
                    for (k, v) in cloned.into_iter() {
                        let sexpr = Ponga::Sexpr(vec![func.clone(), v]);
                        res.insert(k, runtime.eval(sexpr)?);
                    }
                    let res_ref = runtime.gc.ponga_into_gc_ref(Ponga::Object(res));
                    Ok(res_ref)
                }
                _ => Err(RuntimeErr::TypeError(format!("map requires an iterable"))),
            }
        }
        _ => Err(RuntimeErr::TypeError(format!("map requires an iterable"))),
    }
}

pub fn vector_query(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 1, "vector?")?;
    let mut args = transform_args(runtime, args)?;
    let arg = args.pop().unwrap();
    Ok(bool_to_ponga(runtime.is_vector(&arg)))
}

pub fn cdr(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 1, "cdr")?;
    let mut args = transform_args(runtime, args)?;
    let arg = args.pop().unwrap();
    match arg {
        Ponga::List(mut list) => {
            list.pop_front();
            Ok(Ponga::List(list))
        }
        Ponga::Ref(id) => {
            let mut obj = runtime.get_id_obj(id)?.borrow_mut().unwrap();
            let r = obj.inner();
            match r {
                Ponga::List(list) => Ok(Ponga::List(list.iter().skip(1).cloned().collect())),
                _ => Err(RuntimeErr::TypeError(format!("car requires a list"))),
            }
        }
        _ => Err(RuntimeErr::TypeError(format!("car requires a list"))),
    }
}

pub fn plus(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 2, "+")?;
    let mut args = transform_args(runtime, args)?;
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
    let mut args = transform_args(runtime, args)?;
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
    let mut args = transform_args(runtime, args)?;
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
    let mut args = transform_args(runtime, args)?;
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
    let mut args = transform_args(runtime, args)?;
    let snd = args.pop().unwrap();
    let fst = args.pop().unwrap();
    Ok(bool_to_ponga(fst == snd))
}

pub fn teq(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 2, "eq?")?;
    let mut args = transform_args(runtime, args)?;
    // println!("EQUAL {:?}", args);
    let snd = args.pop().unwrap();
    let fst = args.pop().unwrap();
    Ok(bool_to_ponga(
        runtime.ponga_to_string(&fst) == runtime.ponga_to_string(&snd)
    ))
}

pub fn peq(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 2, "eq?")?;
    let mut args = transform_args(runtime, args)?;
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

pub fn or(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_gt(&args, 0, "or")?;
    let mut args = transform_args(runtime, args)?;
    for i in args.into_iter() {
        if i != Ponga::False {
            return Ok(i);
        }
    }
    Ok(Ponga::False)
}

pub fn and(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_gt(&args, 0, "and")?;
    let mut args = transform_args(runtime, args)?;
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
    let mut args = transform_args(runtime, args)?;
    let fst = args.pop().unwrap().to_bool()?;
    Ok(bool_to_ponga(!fst))
}

pub fn ge(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 2, ">=")?;
    let mut args = transform_args(runtime, args)?;
    let snd = args.pop().unwrap().to_number()?;
    let fst = args.pop().unwrap().to_number()?;
    Ok(bool_to_ponga(fst.ge(snd)))
}

pub fn gt(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 2, ">")?;
    let mut args = transform_args(runtime, args)?;
    let snd = args.pop().unwrap().to_number()?;
    let fst = args.pop().unwrap().to_number()?;
    Ok(bool_to_ponga(fst.gt(snd)))
}

pub fn lt(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 2, "<")?;
    let mut args = transform_args(runtime, args)?;
    let snd = args.pop().unwrap().to_number()?;
    let fst = args.pop().unwrap().to_number()?;
    Ok(bool_to_ponga(fst.lt(snd)))
}

pub fn le(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 2, "<=")?;
    let mut args = transform_args(runtime, args)?;
    let snd = args.pop().unwrap().to_number()?;
    let fst = args.pop().unwrap().to_number()?;
    Ok(bool_to_ponga(fst.le(snd)))
}

pub fn modulus(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 2, "<=")?;
    let mut args = transform_args(runtime, args)?;
    let snd = args.pop().unwrap().to_number()?;
    let fst = args.pop().unwrap().to_number()?;
    Ok(Ponga::Number(fst.modulus(snd)))
}

pub fn cond(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
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

pub fn let_(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&args, 2, "let")?;
    let body = args.pop().unwrap();
    let pairs = args.pop().unwrap();

    match pairs {
        Ponga::Sexpr(v) => {
            let mut names = Vec::new();
            for i in v.into_iter() {
                match i {
                    Ponga::Sexpr(v2) => {
                        if v2.len() != 2 {
                            return Err(RuntimeErr::TypeError(format!(
                                "First arg to let must be S-Expr of S-Expr pairs"
                            )));
                        }
                        let mut iter = v2.into_iter();
                        let first = iter.next().unwrap();
                        if !first.is_identifier() {
                            return Err(RuntimeErr::TypeError(format!(
                                "Name in let must be an identifier"
                            )));
                        }
                        let second = runtime.eval(iter.next().unwrap())?;
                        let name = first.extract_name()?;
                        runtime.push_local(&name, second);
                        names.push(name);
                    }
                    _ => return Err(RuntimeErr::TypeError(format!(
                        "let requires S-Exprs inside first arg (not {:?})",
                        i
                    ))),
                }
            }
            let res = runtime.eval(body)?;
            for name in names.into_iter() {
                runtime.pop_identifier_obj(&name)?;
            }
            Ok(res)
        }
        _ => Err(RuntimeErr::TypeError(format!(
            "let requires S-Expr first arg"
        ))),
    }
}

pub fn begin(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_gt(&args, 0, "begin")?;
    Ok(args.pop().unwrap())
}

pub fn disp(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&args, 1, "display")?;
    let mut args = transform_args(runtime, args)?;
    let arg = runtime.eval(args.pop().unwrap())?;
    println!("{}", runtime.ponga_to_string(&arg));
    Ok(Ponga::Null)
}

pub fn foldl(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 3, "foldl")?;
    let mut args = transform_args(runtime, args)?;
    // println!("ARGS: {:?}", args);
    let iterable = args.pop().unwrap();
    let mut acc = args.pop().unwrap();
    let func = args.pop().unwrap();
    if !func.is_func() {
        let mut fail = true;
        if let Ponga::Ref(id) = &func {
            let obj = runtime.get_id_obj_ref(*id)?;
            let obj_ref = obj.borrow().unwrap();
            if obj_ref.inner().is_func() {
                fail = false;
            }
        }
        if fail {
            // println!("FAIL: {:?}", func);
            return Err(RuntimeErr::TypeError(format!(
                "first argument to foldl must be a function"
            )));
        }
    }
    
    match iterable {
        Ponga::Ref(id) => {
            let obj = runtime.get_id_obj_ref(id)?.borrow().unwrap();
            let r = obj.inner();
            match r {
                Ponga::List(list) => {
                    let cloned: LinkedList<Ponga> = list.clone();
                    drop(list);
                    drop(obj);
                    for i in cloned.into_iter() {
                        let sexpr = Ponga::Sexpr(vec![func.clone(), acc, i]);
                        acc = runtime.eval(sexpr)?;
                    }
                    Ok(acc)
                }
                Ponga::Array(arr) => {
                    let cloned: Vec<Ponga> = arr.clone();
                    drop(arr);
                    drop(obj);
                    for i in cloned.into_iter() {
                        let sexpr = Ponga::Sexpr(vec![func.clone(), acc, i]);
                        acc = runtime.eval(sexpr)?;
                    }
                    Ok(acc)
                }
                Ponga::Object(o) => {
                    let cloned: HashMap<String, Ponga> = o.clone();
                    drop(o);
                    drop(obj);
                    for (k, v) in cloned.into_iter() {
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
    let mut args = transform_args(runtime, args)?;
    let iterable = args.pop().unwrap();
    let mut acc = args.pop().unwrap();
    let func = args.pop().unwrap();
    if !func.is_func() {
        let mut fail = true;
        if let Ponga::Ref(id) = &func {
            let obj = runtime.get_id_obj_ref(*id)?;
            let obj_ref = obj.borrow().unwrap();
            if obj_ref.inner().is_func() {
                fail = false;
            }
        }
        if fail {
            return Err(RuntimeErr::TypeError(format!(
                "first argument to foldr must be a function"
            )));
        }
    }
    
    match iterable {
        Ponga::Ref(id) => {
            let obj = runtime.get_id_obj_ref(id)?.borrow().unwrap();
            let r = obj.inner();
            match r {
                Ponga::List(list) => {
                    let cloned: LinkedList<Ponga> = list.clone();
                    drop(list);
                    drop(obj);
                    for i in cloned.into_iter().rev() {
                        let sexpr = Ponga::Sexpr(vec![func.clone(), i, acc]);
                        acc = runtime.eval(sexpr)?;
                    }
                    Ok(acc)
                }
                Ponga::Array(arr) => {
                    let cloned: Vec<Ponga> = arr.clone();
                    drop(arr);
                    drop(obj);
                    for i in cloned.into_iter().rev() {
                        let sexpr = Ponga::Sexpr(vec![func.clone(), i, acc]);
                        acc = runtime.eval(sexpr)?;
                    }
                    Ok(acc)
                }
                Ponga::Object(o) => {
                    let cloned: HashMap<String, Ponga> = o.clone();
                    drop(o);
                    drop(obj);
                    for (k, v) in cloned.into_iter() {
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
    let mut args = transform_args(runtime, args)?;
    let arg = args.pop().unwrap();
    match arg {
        Ponga::Ref(id) => {
            let obj = runtime.get_id_obj_ref(id)?.borrow().unwrap();
            let r = obj.inner();
            match r {
                Ponga::Array(v) => Ok(Ponga::Number(Number::Int(v.len() as isize))),
                _ => Err(RuntimeErr::TypeError(format!("vector-length requires a vector"))),
            } 
        }
        _ => Err(RuntimeErr::TypeError(format!("vector-length requires a vector"))),
    }
}

pub fn vector_ref(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&args, 2, "vector-length")?;
    let mut args = transform_args(runtime, args)?;
    let n = args.pop().unwrap();
    if !n.is_number() {
        return Err(RuntimeErr::TypeError(format!("vector-ref requires a number")));
    }
    let arg = runtime.eval(args.pop().unwrap())?;
    match arg {
        Ponga::Ref(id) => {
            let obj = runtime.get_id_obj_ref(id)?.borrow().unwrap();
            let r = obj.inner();
            match r {
                Ponga::Array(v) => {
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
    let mut args = transform_args(runtime, args)?;
    // println!("APPEND: {:?}", args);
    let n = args.pop().unwrap();
    let arg = args.pop().unwrap();
    match arg {
        Ponga::Ref(id) => {
            let mut obj = runtime.get_id_obj(id)?.borrow_mut().unwrap();
            let r = obj.inner();
            match r {
                Ponga::Array(v) => {
                    v.push(n);
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
    let mut args = transform_args(runtime, args)?;
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
    let mut args = transform_args(runtime, args)?;
    let fst = args.pop().unwrap();
    match fst {
        Ponga::Number(n) => {
            Ok(Ponga::Number(n.floor()))
        }
        _ => Err(RuntimeErr::TypeError(format!("ceiling requires a number"))),
    }
}

pub fn sqrt(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 1, "sqrt")?;
    let mut args = transform_args(runtime, args)?;
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
    let mut args = transform_args(runtime, args)?;
    let n = args.pop().unwrap();
    let arg = args.pop().unwrap();
    match arg {
        Ponga::Ref(id) => {
            let obj = runtime.get_id_obj_ref(id)?.borrow().unwrap();
            let r = obj.inner();
            match r {
                Ponga::Object(o) => {
                    let s = runtime.ponga_to_string(&n);
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
    let mut args = transform_args(runtime, args)?;
    let val = args.pop().unwrap();
    let key = args.pop().unwrap();
    let arg = args.pop().unwrap();
    match arg {
        Ponga::Ref(id) => {
            let s = runtime.ponga_to_string(&key);
            let mut obj = runtime.get_id_obj(id)?.borrow_mut().unwrap();
            let r = obj.inner();
            match r {
                Ponga::Object(o) => {
                    o.insert(s, val);
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
    let mut args = transform_args(runtime, args)?;
    let n = args.pop().unwrap();
    let arg = args.pop().unwrap();
    match arg {
        Ponga::Ref(id) => {
            let obj = runtime.get_id_obj_ref(id)?.borrow().unwrap();
            let r = obj.inner();
            match r {
                Ponga::Object(o) => {
                    let s = runtime.ponga_to_string(&n);
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
            let obj = runtime.get_id_obj_ref(*id)?.borrow().unwrap();
            let r = obj.inner();
            if let Ponga::List(list) = r {
                if list.len() == 2 {
                    let mut iter = list.iter();
                    let key = iter.next().unwrap();
                    let val = iter.next().unwrap();
                    let s = runtime.ponga_to_string(key);
                    map.insert(s, val.clone());
                    return Ok(());
                }
            }
            Err(RuntimeErr::TypeError(format!("list->map requires list of list pairs")))
        }
        _ => {
            Err(RuntimeErr::TypeError(format!("list->map requires list of list pairs")))
        }
    }

}

pub fn list_to_map(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 1, "list->map")?;
    let mut args = transform_args(runtime, args)?;
    let list = args.pop().unwrap();
    
    match list {
        Ponga::Ref(id) => {
            let obj = runtime.get_id_obj_ref(id)?.borrow().unwrap();
            let r = obj.inner();
            match r {
                Ponga::List(list) => {
                    let mut map = HashMap::new();
                    for item in list {
                        insert_list_pair_into_map(runtime, &mut map, item)?;
                    }
                    Ok(Ponga::Object(map))
                }
                _ => Err(RuntimeErr::TypeError(format!("list->map requires a list"))),
            }
        }
        _ => Err(RuntimeErr::TypeError(format!("list->map requires a list"))),
    }
}
