use crate::parser::*;
use crate::stdlib::*;
use crate::types::*;
use crate::env::*;
use crate::instructions::*;
use gc_rs::{Gc, GcRefMut, Trace};
use std::collections::HashMap;
use std::collections::LinkedList;
use std::fs::File;
use std::io::prelude::*;

pub const MAX_STACK_SIZE: usize = 100_000;

pub struct Runtime {
    pub env: Gc<Env>,
}

impl Runtime {
    pub fn new() -> Self {
        let mut env = Gc::new(Env::new(None));

        for (i, val) in FUNCS.iter().enumerate() {
            env.borrow_mut().unwrap().map.insert(val.0.to_string(), Ponga::HFunc(i));
        }

        let mut res = Self {
            env,
        };

        // let stdlib_scm = include_str!("stdlib.scm");
        // res.run_str(stdlib_scm).unwrap();

        res
    }

    pub fn bind_global(&mut self, s: String, pong: Ponga) {
        self.env.borrow_mut().unwrap().insert_furthest(s, pong);
    }

    pub fn get_identifier_obj_ref(&self, identifier: &str) -> RunRes<&Ponga> {
        self.env.get(identifier).ok_or(
            RuntimeErr::ReferenceError(format!("Reference to {} not found", identifier)),
        )
    }

    pub fn set_identifier(&mut self, identifier: &str, pong: Ponga) -> RunRes<()> {
        Env::set(
            self.env.clone(), identifier, pong
        ).ok_or(
            RuntimeErr::ReferenceError(format!("Reference to {} not found", identifier))
        )
    }

    pub fn clone_ref(&mut self, pong: Ponga) -> RunRes<Ponga> {
        match pong {
            Ponga::Ref(gc) => {
                Ok((*gc).clone())
            }
            Ponga::Identifier(s) => {
                let ref_obj = self.get_identifier_obj_ref(&s)?;
                Ok(ref_obj.clone())
            }
            _ => Ok(pong),
        }
    }

    pub fn id_or_ref_peval(&mut self, pong: Ponga) -> RunRes<Ponga> {
        match pong {
            Ponga::Ref(id) => {
                if id.is_copy() {
                    Ok((*id).clone())
                } else {
                    Ok(Ponga::Ref(id))
                }
            }
            Ponga::Identifier(s) => {
                let ref_obj = self.get_identifier_obj_ref(&s)?;
                if ref_obj.is_copy() {
                    Ok(ref_obj.clone())
                } else {
                    let cloned = ref_obj.clone();
                    drop(ref_obj);
                    let gc = Gc::new(cloned);
                    self.set_identifier(&s, Ponga::Ref(gc.clone()))?;
                    Ok(Ponga::Ref(gc))
                }
            }
            _ => {
                if pong.is_copy() {
                    Ok(pong)
                } else {
                    Ok(Ponga::Ref(Gc::new(pong)))
                }
            }
        }
    }

    pub fn eval(&mut self, pong: Ponga) -> RunRes<Ponga> {
        use Ponga::*;
        use Instruction::*;
        let mut data_stack = vec![];
        let mut ins_stack = vec![Instruction::Eval(pong)];
        loop {
            if ins_stack.is_empty() {
                break;
            }
            match ins_stack.pop().unwrap() {
                PopEnv(opt_lev) => {
                    if let Some(level) = opt_lev {
                        self.env.outer.as_ref().unwrap().root();
                        self.env.outer.as_ref().unwrap().borrow_mut().unwrap().in_use = false;
                        self.env = self.env
                                       .borrow_mut()
                                       .unwrap()
                                       .remove_env_at_level(level)
                                       .ok_or(
                            RuntimeErr::Other("PopEnv: no outer env".to_string()),
                        )?;
                    } else {
                        self.env = self.env.borrow_mut().unwrap().outer.take().ok_or(
                            RuntimeErr::Other("PopEnv: no outer env".to_string()),
                        )?;
                    }
                }
                PushEnv(names) => {
                    let mut map = HashMap::new();
                    for name in names {
                        map.insert(name, pop_or(&mut data_stack)?);
                    }
                    self.env = Gc::new(Env {
                        map,
                        outer: Some(self.env.clone()),
                        in_use: true,
                    });
                }
                PopStack => {
                    data_stack.pop();
                }
                Define(s) => {
                    self.env.borrow_mut().unwrap().insert_furthest(
                        s, pop_or(&mut data_stack)?
                    );
                }
                Set(s) => {
                    Env::set(
                        self.env.clone(), &s, pop_or(&mut data_stack)?
                    ).ok_or(
                        RuntimeErr::ReferenceError(format!("Reference to {} not found", s))
                    )?;
                }
                CollectArray(n) => {
                    let mut v = Vec::with_capacity(n);
                    for _ in 0..n {
                        v.push(pop_or(&mut data_stack)?);
                    }
                    data_stack.push(Ponga::Ref(Gc::new(Ponga::Array(v))));
                }
                CollectList(usize) => {
                    let mut v = LinkedList::new();
                    for _ in 0..usize {
                        v.push_back(pop_or(&mut data_stack)?);
                    }
                    data_stack.push(Ponga::Ref(Gc::new(Ponga::List(v))));
                }
                CollectObject(names) => {
                    let mut map = HashMap::new();
                    for name in names {
                        let val = pop_or(&mut data_stack)?;
                        map.insert(name, val);
                    }
                    data_stack.push(Ponga::Ref(Gc::new(Ponga::Object(map))));
                }
                Call(n) => {
                    let mut args = Vec::with_capacity(n);
                    for _ in 0..n {
                        args.push(pop_or(&mut data_stack)?);
                    }
                    let func = pop_or(&mut data_stack)?;
                    match func {
                        HFunc(id) => {
                            data_stack.push(FUNCS[id].1(self, args)?);
                        }
                        _ => {
                            return Err(RuntimeErr::TypeError(format!(
                                "Expected function, not {}", func
                            )));
                        }
                    } 
                }
                Eval(pong) => {
                    match pong {
                        Array(a) => {
                            ins_stack.push(Instruction::CollectArray(a.len()));
                            for pong in a {
                                ins_stack.push(Instruction::Eval(pong));
                            }
                        }
                        List(a) => {
                            ins_stack.push(Instruction::CollectList(a.len()));
                            for pong in a {
                                ins_stack.push(Instruction::Eval(pong));
                            }
                        }
                        Object(o) => {
                            ins_stack.push(Instruction::CollectObject(
                                o.keys().cloned().collect())
                            );
                            for (_, v) in o {
                                ins_stack.push(Instruction::Eval(v));
                            }
                        }
                        Sexpr(vals) => {
                            if vals.len() < 1 {
                                return Err(RuntimeErr::Other("Empty sexpr".to_string()));
                            }
                            if is_keyword(&vals[0]) {
                                let mut iter = vals.into_iter();
                                let name = iter.next().unwrap().extract_name()?;
                                match name.as_str() {
                                    "lambda" => {
                                        if iter.len() != 2 {
                                            return Err(RuntimeErr::Other(
                                                "Wrong number of arguments for lambda".to_string()
                                            ));
                                        }
                                        let args = iter.next()
                                                       .unwrap()
                                                       .extract_names_from_sexpr()?;
                                        let body = iter.next().unwrap();
                                        if !body.is_sexpr() {
                                            return Err(RuntimeErr::Other(
                                                "Wrong type for lambda body".to_string()
                                            ));
                                        }
                                        let state_map = self.env.copy();
                                        let new_state = Gc::new(Env {
                                            map: state_map,
                                            outer: None,
                                            in_use: false,
                                        });
                                        let pushed = CFunc(args, Gc::new(body), new_state);
                                        data_stack.push(pushed);
                                    }
                                    "define" => {
                                        if iter.len() != 2 {
                                            return Err(RuntimeErr::Other(
                                                "Wrong number of arguments for define".to_string()
                                            ));
                                        }
                                        let name = iter.next().unwrap();
                                        if name.is_sexpr() {
                                            let mut arr = name.get_array()?.into_iter();
                                            let name = arr.next().unwrap().extract_name()?;
                                            let names = Ponga::extract_names_from_vec(arr.collect())?;
                                            let names = names.into_iter()
                                                             .map(Identifier)
                                                             .collect();
                                            let vec = vec![
                                                Identifier("define".to_string()),
                                                Identifier(name),
                                                Sexpr(vec![
                                                    Identifier("lambda".to_string()),
                                                    Sexpr(names),
                                                    iter.next().unwrap(),
                                                ])
                                            ];
                                            ins_stack.push(Instruction::Eval(Sexpr(vec)));
                                        } else {
                                            let val = iter.next().unwrap();
                                            ins_stack.push(Instruction::Define(name.extract_name()?));
                                            ins_stack.push(Instruction::Eval(val));
                                        }
                                        data_stack.push(Ponga::Null);
                                    }
                                    "echo" => {
                                        for val in iter {
                                            data_stack.push(val);
                                        }
                                    }
                                    _ => {
                                        return Err(RuntimeErr::Other(format!(
                                            "Unimplemented keyword {}", name
                                        )));
                                    }
                                }
                                continue;
                            }
                            let mut iter = vals.into_iter();
                            let func = self.eval(iter.next().unwrap())?;

                            match func {
                                HFunc(id) => {
                                    ins_stack.push(Instruction::Call(iter.len()));
                                    data_stack.push(HFunc(id));
                                    for arg in iter {
                                        ins_stack.push(Instruction::Eval(arg));
                                    }
                                }
                                CFunc(names, sexpr, state) => {
                                    if !state.in_use {
                                        state.borrow_mut().unwrap().in_use = true;
                                        let new_env = Gc::new(Env {
                                            map: HashMap::new(),
                                            outer: Some(state.clone()),
                                            in_use: true,
                                        });
                                        let level = new_env.borrow_mut()
                                                       .unwrap()
                                                       .add_env_furthest(self.env.clone());
                                        ins_stack.push(Instruction::PopEnv(Some(level)));
                                        self.env = new_env;
                                    }

                                    let len = names.len();

                                    ins_stack.push(Instruction::PopEnv(None));
                                    ins_stack.push(Instruction::Eval((*sexpr).clone()));
                                    ins_stack.push(Instruction::PushEnv(names));

                                    for i in 0..len {
                                        ins_stack.push(Instruction::Eval(iter.next().ok_or(
                                            RuntimeErr::Other(format!(
                                                "Expected {} arguments, got {}", len, i 
                                            ))
                                        )?));
                                    }
                                }
                                MFunc(names, sexpr) => {
                                    ins_stack.push(Instruction::PopEnv(None));
                                    let len = names.len();
                                    ins_stack.push(Instruction::Call(0));
                                    ins_stack.push(Instruction::PushEnv(names));
                                    for i in 0..len {
                                        data_stack.push(iter.next().ok_or(
                                            RuntimeErr::Other(format!(
                                                "Expected {} arguments, got {}", len, i 
                                            ))
                                        )?);
                                    }

                                    data_stack.push(MFunc(vec![], sexpr));
                                }
                                _ => {
                                    return Err(RuntimeErr::Other(format!(
                                        "First element of sexpr `{}` is not function",
                                        func
                                    )));
                                }
                            }
                        }
                        _ => data_stack.push(self.id_or_ref_peval(pong)?),
                    }
                }
            }
        }
        pop_or(&mut data_stack)
    }

    pub fn run_str(&mut self, s: &str) -> RunRes<()> {
        let parsed = match pongascript_parser(s) {
            Ok(val) => val,
            Err(e) => {
                println!("{}", e);
                return Ok(());
            }
        };
        if parsed.0.len() != 0 {
            println!(
                "Unexpected tokens: {:?}",
                parsed.0
            );
            return Ok(());
        }
        let mut evald = parsed
            .1
            .into_iter()
            .map(|x| {
                let res = self.eval(x);
                match &res {
                    Ok(_) => (),
                    Err(e) => {
                        println!("{}", e);
                    }
                }
                res
            })
            .collect::<Vec<RunRes<Ponga>>>();
        match evald.pop().unwrap() {
            Ok(last) => {
                if last != Ponga::Null {
                    println!("{}", last);
                    self.bind_global("last".to_string(), last);
                }
            }
            _ => (),
        }
        Ok(())
    }

    pub fn run_file(&mut self, filename: &str) -> RunRes<()> {
        let mut file = File::open(filename)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        self.run_str(&contents)
    }
}

pub fn run_str(s: &str) -> RunRes<Vec<RunRes<Ponga>>> {
    let mut runtime = Runtime::new();
    let parsed = pongascript_parser(s);
    let parsed = parsed?;
    if parsed.0.len() != 0 {
        return Err(RuntimeErr::ParseError(format!(
            "Unexpected tokens: {:?}",
            parsed.0
        )));
    }
    let evald = parsed
        .1
        .into_iter()
        .map(|x| {
            let res = runtime.eval(x);
            match &res {
                Ok(_) => (),
                Err(e) => println!("{}", e),
            }
            res
        })
        .collect::<Vec<RunRes<Ponga>>>();
    let last = &evald[evald.len() - 1];
    match last {
        Ok(v) => {
            if !v.is_null() {
                println!("Program returned: {}", v);
            }
        }
        Err(e) => println!("Error: {:?}", e),
    };
    Ok(evald)
}

pub fn run_file(s: &str) -> RunRes<Vec<RunRes<Ponga>>> {
    let mut file = File::open(s)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    run_str(&contents)
}

pub fn pop_or<T>(vec: &mut Vec<T>) -> RunRes<T> {
    vec.pop().ok_or(RuntimeErr::Other(format!("Expected value in stack")))
}

pub fn is_keyword(pong: &Ponga) -> bool {
    match pong {
        Ponga::Identifier(s) => KEYWORDS.iter().any(|x| x == &s),
        _ => false,
    }
}
