use crate::runtime::*;
use crate::take_obj::*;
use crate::types::*;
use std::collections::LinkedList;

pub const FUNCS: &[(&str, fn(&mut Runtime, Vec<Ponga>) -> RunRes<Ponga>)] = &[
    ("cons", cons),
    ("null?", null),
    ("define", define),
    ("if", iff),
    ("car", car),
    ("cdr", cdr),
    ("vector?", vector_query),
    ("lambda", lambda),
    ("+", plus),
    ("-", minus),
    ("*", times),
    ("/", div),
    ("eq?", eq),
    // Should not be same but ceebs
    ("eqv?", eq),
    ("equal?", teq),
    ("=", peq),
    ("set!", set),
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

pub fn eval_args(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Vec<Ponga>> {
    let mut res = Vec::new();
    res.reserve(args.len());
    for arg in args.into_iter() {
        res.push(runtime.eval(arg)?);
    }
    Ok(res)
}

pub fn cons(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 2, "cons")?;
    let mut args = eval_args(runtime, args)?;
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
    let mut args = eval_args(runtime, args)?;
    let arg = runtime.eval(args.pop().unwrap())?;
    match arg {
        Ponga::List(list) => Ok(runtime.gc.ponga_into_gc_ref(bool_to_ponga(list.is_empty()))),
        Ponga::Null => Ok(Ponga::True),
        Ponga::Ref(id) => {
            let mut obj = runtime.get_id_obj(id)?.borrow_mut().unwrap();
            let r = obj.inner();
            match r {
                Ponga::List(list) => {
                    let res = bool_to_ponga(list.is_empty());
                    drop(obj);
                    Ok(runtime.gc.ponga_into_gc_ref(res))
                }
                Ponga::Null => Ok(Ponga::True),
                _ => Err(RuntimeErr::TypeError(format!(
                    "null? requires a list or null"
                ))),
            }
        }
        _ => Err(RuntimeErr::TypeError(format!("null requires a list"))),
    }
}

pub fn define(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 2, "define")?;
    let func = args.pop().unwrap();
    let arg = args.pop().unwrap();
    if !arg.is_sexpr() {
        if arg.is_identifier() {
            let name = arg.extract_name()?;
            let res = runtime.eval(func)?;
            runtime.bind_global(name, res);
            return Ok(Ponga::Null);
        }
        return Err(RuntimeErr::TypeError(format!(
            "define requires an S-Expr as first argument"
        )));
    }
    let arr = arg.get_array()?;
    if arr.len() < 1 {
        return Err(RuntimeErr::TypeError(format!("define requires a name")));
    } else if !arr[0].is_identifier() {
        return Err(RuntimeErr::TypeError(format!(
            "first arg to define must be an identifier"
        )));
    }

    let mut iter = arr.into_iter();
    let name = iter.next().unwrap().extract_name()?;
    let new_args = vec![Ponga::Sexpr(iter.collect()), func];

    let cfunc = lambda(runtime, new_args)?;
    runtime.bind_global(name, cfunc);
    Ok(Ponga::Null)
}

pub fn iff(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 3, "if")?;
    let mut iter = args.into_iter();
    let cond = runtime.eval(iter.next().unwrap())?;
    match cond {
        Ponga::False => runtime.eval(iter.nth(1).unwrap()),
        Ponga::Ref(id) => {
            let obj = runtime.get_id_obj(id)?.borrow().unwrap();
            let inner = obj.inner();
            match inner {
                Ponga::False => {
                    drop(obj);
                    runtime.eval(iter.nth(1).unwrap())
                }
                _ => {
                    drop(obj);
                    runtime.eval(iter.next().unwrap())
                }
            }
        }
        _ => runtime.eval(iter.next().unwrap()),
    }
}

pub fn car(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 1, "car")?;
    let mut args = eval_args(runtime, args)?;
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
    let args = eval_args(runtime, args)?;
    let first = &args[0];
    if !args[0].is_func() {
        return Err(RuntimeErr::TypeError(format!(
            "first argument to map must be a function"
        )));
    }

    Ok(Ponga::Null)
}

pub fn vector_query(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 1, "vector?")?;
    let mut args = eval_args(runtime, args)?;
    let arg = args.pop().unwrap();
    Ok(bool_to_ponga(runtime.is_vector(&arg)))
}

pub fn cdr(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 1, "cdr")?;
    let mut args = eval_args(runtime, args)?;
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

pub fn lambda(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 2, "lambda")?;
    let func = args.pop().unwrap();
    let arg = args.pop().unwrap();
    match arg {
        Ponga::Sexpr(v) => {
            let mut iter = v.into_iter();
            let mut new_args: Vec<String> = Vec::new();
            for i in iter {
                match i {
                    Ponga::Identifier(s) => new_args.push(s),
                    _ => {
                        return Err(RuntimeErr::TypeError(format!(
                            "arguments to lambda function must be identifiers"
                        )))
                    }
                }
            }
            let id = runtime.gc.add_obj(func);
            let state = runtime.condense_locals();
            let stateid = runtime.gc.add_obj(Ponga::Object(state));
            let cfunc = Ponga::CFunc(new_args, id, stateid);
            Ok(cfunc)
        }
        _ => Err(RuntimeErr::TypeError(format!(
            "first argument to lambda must be a S-Expression"
        ))),
    }
}

pub fn plus(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 2, "+")?;
    let mut args = eval_args(runtime, args)?;
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
    let mut args = eval_args(runtime, args)?;
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
    let mut args = eval_args(runtime, args)?;
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
    let mut args = eval_args(runtime, args)?;
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
    let mut args = eval_args(runtime, args)?;
    let snd = args.pop().unwrap();
    let fst = args.pop().unwrap();
    Ok(bool_to_ponga(fst == snd))
}

pub fn teq(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 2, "eq?")?;
    let mut args = eval_args(runtime, args)?;
    let snd = args.pop().unwrap();
    let fst = args.pop().unwrap();

    match fst {
        Ponga::Identifier(s1) => {
            let fre1 = runtime.get_identifier_obj_ref(&s1)?;
            match snd {
                Ponga::Identifier(s2) => {
                    let fre2 = runtime.get_identifier_obj_ref(&s2)?;
                    Ok(bool_to_ponga(fre1 == fre2))
                }
                Ponga::Ref(id) => {
                    let obj = runtime.get_id_obj_ref(id)?;
                    let borrowed = obj.borrow().unwrap();
                    let fre2 = borrowed.inner();
                    Ok(bool_to_ponga(fre1 == fre2))
                }
                ponga => Ok(bool_to_ponga(fre1 == &ponga)),
            }
        }
        Ponga::Ref(id) => {
            let obj1 = runtime.get_id_obj_ref(id)?;
            let borrowed1 = obj1.borrow().unwrap();
            let fre1 = borrowed1.inner();
            match snd {
                Ponga::Identifier(s2) => {
                    let fre2 = runtime.get_identifier_obj_ref(&s2)?;
                    Ok(bool_to_ponga(fre1 == fre2))
                }
                Ponga::Ref(id) => {
                    let obj = runtime.get_id_obj_ref(id)?;
                    let borrowed = obj.borrow().unwrap();
                    let fre2 = borrowed.inner();
                    Ok(bool_to_ponga(fre1 == fre2))
                }
                ponga => Ok(bool_to_ponga(fre1 == &ponga)),
            }
        }
        fre1 => match snd {
            Ponga::Identifier(s2) => {
                let fre2 = runtime.get_identifier_obj_ref(&s2)?;
                Ok(bool_to_ponga(&fre1 == fre2))
            }
            Ponga::Ref(id) => {
                let obj = runtime.get_id_obj_ref(id)?;
                let borrowed = obj.borrow().unwrap();
                let fre2 = borrowed.inner();
                Ok(bool_to_ponga(&fre1 == fre2))
            }
            ponga => Ok(bool_to_ponga(fre1 == ponga)),
        },
    }
}

pub fn peq(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 2, "eq?")?;
    let mut args = eval_args(runtime, args)?;
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

pub fn set(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 2, "set!")?;
    let snd = args.pop().unwrap();
    let fst = args.pop().unwrap();
    match fst {
        Ponga::Identifier(s) => {
            let res = runtime.eval(snd)?;
            runtime.set_identifier(&s, res)?;
            Ok(Ponga::Null)
        }
        _ => Err(RuntimeErr::TypeError(format!(
            "set! requires an identifier"
        ))),
    }
}

pub fn or(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_gt(&args, 0, "or")?;
    let mut args = eval_args(runtime, args)?;
    for i in args.into_iter() {
        if i != Ponga::False {
            return Ok(i);
        }
    }
    Ok(Ponga::False)
}

pub fn and(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_gt(&args, 0, "and")?;
    let mut args = eval_args(runtime, args)?;
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
    let mut args = eval_args(runtime, args)?;
    let fst = args.pop().unwrap().to_bool()?;
    Ok(bool_to_ponga(!fst))
}

pub fn ge(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 2, ">=")?;
    let mut args = eval_args(runtime, args)?;
    let snd = args.pop().unwrap().to_number()?;
    let fst = args.pop().unwrap().to_number()?;
    Ok(bool_to_ponga(fst.ge(snd)))
}

pub fn gt(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 2, ">")?;
    let mut args = eval_args(runtime, args)?;
    let snd = args.pop().unwrap().to_number()?;
    let fst = args.pop().unwrap().to_number()?;
    Ok(bool_to_ponga(fst.gt(snd)))
}

pub fn lt(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 2, "<")?;
    let mut args = eval_args(runtime, args)?;
    let snd = args.pop().unwrap().to_number()?;
    let fst = args.pop().unwrap().to_number()?;
    Ok(bool_to_ponga(fst.lt(snd)))
}

pub fn le(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 2, "<=")?;
    let mut args = eval_args(runtime, args)?;
    let snd = args.pop().unwrap().to_number()?;
    let fst = args.pop().unwrap().to_number()?;
    Ok(bool_to_ponga(fst.le(snd)))
}

pub fn modulus(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 2, "<=")?;
    let mut args = eval_args(runtime, args)?;
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
    let len = args.len();
    for (i, arg) in args.into_iter().enumerate() {
        if i == len - 1 {
            return runtime.eval(arg);
        }
        runtime.eval(arg)?;
    }
    Ok(Ponga::Null)
}

pub fn disp(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&args, 1, "display")?;
    println!("{}", runtime.ponga_to_string(&args[0]));
    Ok(Ponga::Null)
}
