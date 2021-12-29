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
        _ => Err(RuntimeErr::TypeError(format!(
            "first argument to define must be a S-Expression"
        ))),
    }
}

pub fn iff(runtime: &mut Runtime, mut args: Vec<Ponga>) -> RunRes<Ponga> {
    args_assert_len(&mut args, 3, "if")?;
    let mut iter = args.into_iter();
    let cond = runtime.eval(iter.next().unwrap())?;
    match cond {
        Ponga::True => runtime.eval(iter.next().unwrap()),
        Ponga::False => runtime.eval(iter.nth(1).unwrap()),
        Ponga::Ref(id) => {
            let obj = runtime.get_id_obj(id)?.borrow().unwrap();
            let inner = obj.inner();
            match inner {
                Ponga::True => {
                    drop(obj);
                    runtime.eval(iter.next().unwrap())
                }
                Ponga::False => {
                    drop(obj);
                    runtime.eval(iter.nth(1).unwrap())
                }
                _ => Err(RuntimeErr::TypeError(format!(
                    "if requires a boolean condition (provided {:?})",
                    cond
                ))),
            }
        }
        _ => Err(RuntimeErr::TypeError(format!(
            "if requires a boolean condition (provided {:?})",
            cond
        ))),
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
