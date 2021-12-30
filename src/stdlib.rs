use crate::runtime::*;
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
    ("equal?", eq),
    ("=", peq),
    ("set!", set),
    ("or", or),
    ("and", and),
    ("not", not),
    ("<", lt),
    ("<=", le),
    (">", ge),
    (">=", gt),
    ("modulo", modulus),
    ("cond", cond),
    ("begin", begin),
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
    let first = args.pop().unwrap();
    match first {
        Ponga::List(mut list) => {
            list.push_front(args[0].clone());
            Ok(runtime.gc.ponga_into_gc_ref(Ponga::List(list)))
        }
        Ponga::Ref(id) => {
            let mut obj = runtime.get_id_obj(id)?.borrow_mut().unwrap();
            let r = obj.inner();
            match r {
                Ponga::List(list) => {
                    list.push_front(args.into_iter().next().unwrap());
                    Ok(Ponga::Ref(id))
                }
                Ponga::Null => {
                    let list = args.into_iter().collect();
                    drop(obj);
                    Ok(runtime.gc.ponga_into_gc_ref(Ponga::List(list)))
                }
                _ => {
                    let mut list = std::collections::LinkedList::new();
                    list.push_front(first);
                    list.push_front(args.into_iter().next().unwrap());
                    drop(obj);
                    Ok(runtime.gc.ponga_into_gc_ref(Ponga::List(list)))
                }
            }
        }
        _ => {
            let mut list = std::collections::LinkedList::new();
            list.push_front(first);
            list.push_front(args.into_iter().next().unwrap());
            Ok(runtime.gc.ponga_into_gc_ref(Ponga::List(list)))
        }
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
    match arg {
        Ponga::Sexpr(v) => {
            let mut iter = v.into_iter();
            let name = iter
                .next()
                .ok_or(RuntimeErr::TypeError(format!("define requires a name")))?;
            let name = match name {
                Ponga::Identifier(s) => s,
                _ => {
                    return Err(RuntimeErr::TypeError(format!(
                        "define requires an identifier as the first argument"
                    )))
                }
            };
            let mut new_args: Vec<String> = Vec::new();
            for i in iter {
                match i {
                    Ponga::Identifier(s) => new_args.push(s),
                    _ => {
                        return Err(RuntimeErr::TypeError(format!(
                            "arguments to defined functions must be identifiers"
                        )))
                    }
                }
            }
            let id = runtime.gc.add_obj(func);
            let cfunc = Ponga::CFunc(new_args, id);
            let id = runtime.gc.add_obj(cfunc);
            runtime.bind_global(name, id);
            Ok(Ponga::Null)
        }
        Ponga::Identifier(s) => {
            let id = runtime.gc.add_obj(func);
            runtime.bind_global(s, id);
            Ok(Ponga::Null)
        }
        _ => Err(RuntimeErr::TypeError(format!(
            "first argument to define must be a S-Expression (for function definition) or an identifier"
        ))),
    }
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

pub fn map_(runtime: &mut Runtime, mut args: Vec<Ponga>)-> RunRes<Ponga> {
    args_assert_len(&mut args, 2, "map")?;
    let args = eval_args(runtime, args)?;
    let first = &args[0];
    if !args[0].is_func() {
        return Err(RuntimeErr::TypeError(format!("first argument to map must be a function")));
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
            let cfunc = Ponga::CFunc(new_args, id);
            Ok(cfunc)
        }
        _ => Err(RuntimeErr::TypeError(format!(
            "first argument to define must be a S-Expression"
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
            let id = runtime.gc.add_obj(res);
            runtime.bind_global(s, id);
            Ok(Ponga::Ref(id))
        }
        _ => Err(RuntimeErr::TypeError(format!("set! requires an identifier"))),
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
                    return Err(RuntimeErr::TypeError(format!("Args to cond must be S-Expr pairs")));
                }
                let mut iter = v.into_iter();
                let first = iter.next().unwrap();
                match &first {
                    Ponga::Identifier(s) => if s == "else" {
                        return runtime.eval(iter.next().unwrap());
                    },
                    _ => (),
                }
                let cond = runtime.eval(first)?;
                match cond {
                    Ponga::True => return runtime.eval(iter.next().unwrap()),
                    _ => continue,
                }
            }
            _ => return Err(RuntimeErr::TypeError(format!("cond requires a S-Expr args"))),
        }
    }
    Ok(Ponga::Null)
}

pub fn let_(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    Ok(Ponga::Null)
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
