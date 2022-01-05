use crate::gc::*;
use crate::gc_obj::*;
use crate::parser::*;
use crate::stdlib::*;
use crate::types::*;
use crate::env::*;
use crate::instructions::*;
use itertools::Itertools;
use std::collections::HashMap;
use std::collections::LinkedList;
use std::fs::File;
use std::io::prelude::*;

pub const MAX_STACK_SIZE: usize = 100_000;

pub type Namespace = HashMap<String, Ponga>;

pub type PriorityNamespace = HashMap<String, Vec<Ponga>>;

pub struct Runtime {
    pub env: Env,
    pub gc: Gc<Ponga>,
    pub env_gc: Gc<Env>,
}

pub enum WhereVar {
    Global,
    Local,
    GlobalFunc,
}

impl Runtime {
    pub fn new() -> Self {
        let mut env = Env::new(None);

        for (i, val) in FUNCS.iter().enumerate() {
            env.map.insert(val.0.to_string(), Ponga::HFunc(i));
        }

        let mut res = Self {
            env,
            gc: Gc::new(),
            env_gc: Gc::new(),
        };

        let stdlib_scm = include_str!("stdlib.scm");
        res.run_str(stdlib_scm).unwrap();

        res
    }

    pub fn bind_global(&mut self, s: String, pong: Ponga) {
        self.env.insert_furthest(&mut self.env_gc, s, pong);
    }

    pub fn collect_garbage(&mut self) {
        self.env.trace(&self.gc);
        self.gc.collect_garbage();

        self.env.trace(&self.env_gc);
        self.env_gc.collect_garbage();
    }

    pub fn get_id_obj_ref(&self, id: Id) -> RunRes<&GcObj<Ponga>> {
        self.gc.get(id)
    }

    pub fn get_id_obj(&mut self, id: Id) -> RunRes<&mut GcObj<Ponga>> {
        self.gc.get_mut(id)
    }

    pub fn get_identifier_obj_ref(&self, identifier: &str) -> RunRes<&Ponga> {
        self.env.get(&self.env_gc, identifier)
    }

    pub fn set_identifier(&mut self, identifier: &str, pong: Ponga) -> RunRes<()> {
        self.env.set(&mut self.env_gc, identifier, pong)
    }

    pub fn clone_ref(&mut self, pong: Ponga) -> RunRes<Ponga> {
        match pong {
            Ponga::Ref(id) => {
                let ref_obj = self.get_id_obj_ref(id)?;
                let inner = ref_obj.borrow().unwrap();
                Ok(inner.inner().clone())
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
                let ref_obj = self.get_id_obj_ref(id)?;
                let inner = ref_obj.borrow().unwrap();
                if inner.inner().is_copy() {
                    Ok(inner.inner().clone())
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
                    let r = self.gc.ponga_into_gc_ref(cloned);
                    self.set_identifier(&s, r.clone())?;
                    Ok(r)
                }
            }
            _ => {
                if pong.is_copy() {
                    Ok(pong)
                } else {
                    Ok(self.gc.ponga_into_gc_ref(pong))
                }
            }
        }
    }

    pub fn eval(&mut self, pong: Ponga) -> RunRes<Ponga> {
        use Ponga::*;
        let mut data_stack = vec![];
        let mut ins_stack = vec![Instruction::Eval(pong)];
        loop {
            if ins_stack.len() > MAX_STACK_SIZE {
                return Err(RuntimeErr::Other(format!(
                    "Stack size exceeded max of {}", MAX_STACK_SIZE
                )));
            }
            match ins_stack.pop().unwrap() {
                Instruction::PopStack => {
                    data_stack.pop();
                }
                Instruction::Define(s) => {
                    self.bind_global(s, pop_or(&mut data_stack)?);
                    data_stack.push(Ponga::Null);
                }
                Instruction::PopEnv(n) => {
                    let map = std::mem::replace(&mut self.env.map, HashMap::new());
                    if let Some(id) = n {
                        self.env_gc.add_id(Env::from_map(map), id);
                    }
                    let id = self.env.outer.ok_or(
                        RuntimeErr::Other("Expected environment".to_string())
                    )?;
                    self.env = self.env_gc.take(id).ok_or(
                        RuntimeErr::Other(format!("Expected env ref {}", id))
                    )?;
                }
                Instruction::PushEnv(names) => {
                    let mut map = HashMap::new();
                    for name in names {
                        let val = pop_or(&mut data_stack)?;
                        map.insert(name, val);
                    }
                    let old_env = std::mem::replace(&mut self.env, Env::new(None));
                    self.env = old_env.integrate_hashmap(&mut self.env_gc, map);
                }
                Instruction::Set(s) => { 
                    let data = pop_or(&mut data_stack)?;
                    self.set_identifier(&s, data)?;
                    data_stack.push(Ponga::Null);
                }
                Instruction::CollectArray(n) => {
                    let mut res = Vec::new();
                    for _ in 0..n {
                        res.push(pop_or(&mut data_stack)?);
                    }
                    data_stack.push(self.gc.ponga_into_gc_ref(Ponga::Array(res)));
                }
                Instruction::CollectList(n) => {
                    let mut res = LinkedList::new();
                    for _ in 0..n {
                        res.push_back(pop_or(&mut data_stack)?);
                    }
                    data_stack.push(self.gc.ponga_into_gc_ref(Ponga::List(res)));
                }
                Instruction::CollectObject(strings) => {
                    let mut res = HashMap::new();
                    for name in strings.into_iter() {
                        res.insert(name, pop_or(&mut data_stack)?);
                    }
                    data_stack.push(self.gc.ponga_into_gc_ref(Ponga::Object(res)));
                }
                Instruction::Call(num_args) => {
                    let mut args = Vec::new();
                    for _ in 0..num_args {
                        args.push(data_stack.pop().ok_or(
                            RuntimeErr::Other(
                                format!("Expected {} args for function", num_args)
                            )
                        )?);
                    }
                    let func = data_stack.pop().ok_or(
                        RuntimeErr::Other(
                            format!("Expected {} args for function", num_args)
                        )
                    )?;
                    match func {
                        HFunc(id) => {
                            data_stack.push(FUNCS[id].1(self, args)?);
                        }
                        CFunc(args_names, sexpr_id, state_id) => {
                            ins_stack.push(Instruction::PopEnv(Some(state_id)));
                            ins_stack.push(Instruction::PopEnv(None));

                            // Push S-Expr to be evaluated
                            let sexpr_obj = self.get_id_obj_ref(sexpr_id)?;
                            let sexpr = sexpr_obj.borrow().unwrap().clone();
                            ins_stack.push(Instruction::Eval(sexpr));

                            let state_obj = self.env_gc.take(state_id).unwrap();
                            let state_map = state_obj.map;

                            // Push the state
                            let old_env = std::mem::replace(&mut self.env, Env::new(None));
                            self.env = old_env.integrate_hashmap(&mut self.env_gc, state_map);

                            // Push all of the names and arg values
                            let args_map = args_names.iter()
                                                     .cloned()
                                                     .zip(args.into_iter())
                                                     .collect();
                            let old_env = std::mem::replace(&mut self.env, Env::new(None));
                            self.env = old_env.integrate_hashmap(&mut self.env_gc, args_map);
                        } 
                        MFunc(args_names, sexpr_id) => {
                            ins_stack.push(Instruction::PopEnv(None));

                            // Push all of the names and arg values
                            let args_map = args_names.iter()
                                                     .rev()
                                                     .cloned()
                                                     .zip(args.into_iter())
                                                     .collect();
                            let old_env = std::mem::replace(&mut self.env, Env::new(None));
                            self.env = old_env.integrate_hashmap(&mut self.env_gc, args_map);

                            // Push S-Expr to be evaluated
                            let sexpr_obj = self.get_id_obj_ref(sexpr_id)?;
                            let sexpr = sexpr_obj.borrow().unwrap().clone();

                            ins_stack.push(Instruction::Eval(sexpr));
                        }
                        o => return Err(RuntimeErr::TypeError(
                            format!("Expected function, received {:?}!", o)
                        )),
                    }
                }
                Instruction::Eval(val) => {
                    match val {
                        Ponga::Array(v) => {
                            ins_stack.push(Instruction::CollectArray(v.len()));
                            for i in v.into_iter() {
                                ins_stack.push(Instruction::Eval(i));
                            }
                        }
                        Ponga::List(v) => {
                            ins_stack.push(Instruction::CollectList(v.len()));
                            for i in v.into_iter() {
                                ins_stack.push(Instruction::Eval(i));
                            }
                        }
                        Ponga::Object(v) => {
                            ins_stack.push(Instruction::CollectObject(
                                v.keys().cloned().collect()
                            ));
                            for (_, i) in v.into_iter() {
                                ins_stack.push(Instruction::Eval(i));
                            }
                        }
                        Ponga::Sexpr(v) => {
                            let mut iter = v.into_iter();
                            let func = iter.next().unwrap();
                            // Goes in level for greater readability
match func {
    Identifier(s) => match s.as_str() {
        "if" => {
            if iter.len() != 3 {
                return Err(RuntimeErr::Other(
                    "if must have three arguments".to_string()
                ));
            }
            // Can make this better if we push it later but should be fine for now
            //println!("iter: {:?}", iter);
            let cond = self.id_or_ref_peval(iter.next().unwrap())?;
            let cond = self.eval(cond)?;
            //println!("cond: {}", cond);
            let val = if cond != Ponga::False {
                iter.nth(0).unwrap()
            } else {
                iter.nth(1).unwrap()
            };
            ins_stack.push(Instruction::Eval(val));
            continue;
        }
        "$PRINT_RAW" => {
            if iter.len() != 1 {
                return Err(RuntimeErr::Other(
                    "$PRINT_RAW must have one argument".to_string()
                ));
            }
            println!("{}", iter.next().unwrap().deep_copy(&self));
            data_stack.push(Ponga::Null);
            continue;
        }
        "copy" => {
            if iter.len() != 1 {
                return Err(RuntimeErr::Other(
                    "copy must have one argument".to_string()
                ));
            }
            let val = iter.next().unwrap().deep_copy(&self);
            data_stack.push(val);
            continue;
        }
        "$EVAL" | "eval" => {
            if iter.len() != 1 {
                return Err(RuntimeErr::Other(
                    "$EVAL must have one argument".to_string()
                ));
            }
            let val = iter.next().unwrap().deep_copy(&self);
            ins_stack.push(Instruction::Eval(val));
            continue;
        }
        "$FLIP"
        | "code<->data"
        | "data<->code" => {
            if iter.len() != 1 {
                return Err(RuntimeErr::Other(
                    "$FLIP must have one argument".to_string()
                ));
            }
            let val = match iter.next().unwrap() {
                Ponga::Identifier(s) => {
                    match self.get_identifier_obj_ref(&s) {
                        Ok(o) => o.clone(),
                        Err(_) => Ponga::Identifier(s),
                    }
                }
                val => val,
            };
            data_stack.push(val.flip_code_vals(&self));
            continue;
        }
        "$FLIP-EVAL"
        | "code<->data.eval"
        | "data<->code.eval" => {
            if iter.len() != 1 {
                return Err(RuntimeErr::Other(
                    "$FLIP-EVAL must have one argument".to_string()
                ));
            }
            let val = iter.next().unwrap();
            ins_stack.push(Instruction::Eval(val.flip_code_vals(&self)));
            continue;
        }
        "$EVAL-FLIP-EVAL"
        | "eval.code<->data.eval"
        | "eval.data<->code.eval" => {
            if iter.len() != 1 {
                return Err(RuntimeErr::Other(
                    "eval.code<->list.eval must have one argument".to_string()
                ));
            }
            let val = self.eval(iter.next().unwrap())?;
            ins_stack.push(Instruction::Eval(val.flip_code_vals(&self)));
            continue;
        }
        "quote" => {
            if iter.len() != 1 {
                return Err(RuntimeErr::Other(
                    "quote must have one argument".to_string()
                ));
            }
            let val = iter.next().unwrap().deep_copy(&self);
            let val = match val {
                Ponga::Sexpr(arr) => Ponga::List(arr.into_iter().collect()),
                Ponga::Identifier(s) => Ponga::Symbol(s),
                val => val,
            };
            data_stack.push(val);
        }
        "$DELAY" => {
            for i in iter {
                data_stack.push(i);
            }
        }
        "sym->id" => {
            if iter.len() != 1 {
                return Err(RuntimeErr::Other(
                    "sym->id must have one argument".to_string()
                ));
            }
            let val = self.id_or_ref_peval(iter.next().unwrap())?;
            data_stack.push(
                self.id_or_ref_peval(Ponga::Identifier(val.get_symbol_string()?))?
            );
        }
        "lambda" => {
            if iter.len() != 2 {
                return Err(RuntimeErr::Other(
                    "lambda must have two arguments".to_string()
                ));
            }
            let first = iter.next().unwrap();
            let body = iter.next().unwrap();

            let mut cargs = Vec::new();
            if !first.is_sexpr() {
                return Err(RuntimeErr::Other(
                    "first argument to lambda must be an s-expr with identifiers"
                        .to_string()
                ));
            }

            let inner = first.get_array()?;
            for i in inner {
                if !i.is_identifier() {
                    return Err(RuntimeErr::Other(
                        "first argument to lambda must be an s-expr with identifiers"
                            .to_string()
                    ));
                }
                cargs.push(i.extract_name()?);
            }

            let new_state = self.env.copy(&self.env_gc);
            let state_id = self.env_gc.add(new_state);
            let body_id = self.gc.add(body);


            data_stack.push(CFunc(cargs, body_id, state_id));
            continue;
        }
        "open-lambda" => {
            if iter.len() != 2 {
                return Err(RuntimeErr::Other(
                    "lambda must have two arguments".to_string()
                ));
            }
            let first = iter.next().unwrap();
            let body = iter.next().unwrap();

            let mut cargs = Vec::new();
            if !first.is_sexpr() {
                return Err(RuntimeErr::Other(
                    "first argument to lambda must be an s-expr with identifiers"
                        .to_string()
                ));
            }

            let inner = first.get_array()?;
            for i in inner {
                if !i.is_identifier() {
                    return Err(RuntimeErr::Other(
                        "first argument to lambda must be an s-expr with identifiers"
                            .to_string()
                    ));
                }
                cargs.push(i.extract_name()?);
            }

            let new_state = Env::new(None);
            let state_id = self.env_gc.add(new_state);
            let body_id = self.gc.add(body);


            data_stack.push(CFunc(cargs, body_id, state_id));
            continue;
        }
        "mac" => {
            if iter.len() != 2 {
                return Err(RuntimeErr::Other(
                    "mac must have two arguments".to_string()
                ));
            }
            let first = iter.next().unwrap();
            let body = iter.next().unwrap();

            let mut cargs = Vec::new();
            if !first.is_sexpr() {
                return Err(RuntimeErr::Other(
                    "first argument to mac must be an s-expr with identifiers"
                        .to_string()
                ));
            }

            let inner = first.get_array()?;
            for i in inner {
                if !i.is_identifier() {
                    return Err(RuntimeErr::Other(
                        "first argument to mac must be an s-expr with identifiers"
                            .to_string()
                    ));
                }
                cargs.push(i.extract_name()?);
            }

            let body_id = self.gc.add(body);

            data_stack.push(MFunc(cargs, body_id));
            continue;
        }
        "begin" => {
            if iter.len() < 1 {
                return Err(RuntimeErr::Other(
                    "begin must have at least one argument".to_string()
                ));
            }
            let mut iter = iter.rev();
            let first = iter.next().unwrap();
            ins_stack.push(Instruction::Eval(first));
            for i in iter {
                ins_stack.push(Instruction::PopStack);
                ins_stack.push(Instruction::Eval(i));
            }
            continue;
        }
        "deref" => {
            if iter.len() != 1 {
                return Err(RuntimeErr::Other(
                    "deref must have one argument".to_string()
                ));
            }
            let val = iter.next().unwrap();
            let deref = match val {
                Ponga::Identifier(name) => {
                    let r = self.get_identifier_obj_ref(&name)?; 
                    if !r.is_identifier() {
                        Err(RuntimeErr::Other(format!(
                            "identifier in deref must refer to an identifier (not {:?})", r
                        )))
                    } else {
                        Ok(r.clone())
                    }
                }
                _ => Err(RuntimeErr::Other(format!(
                    "deref requires an identifier as argument"
                ))),
            }?;
            data_stack.push(deref);
            continue;
        }
        "let-deref" => {
            if iter.len() < 2 {
                return Err(RuntimeErr::Other(
                    "let must have at least two arguments".to_string()
                ));
            }
            let first = iter.next().unwrap();
            let mut body = vec![Ponga::Identifier("begin".to_string())];
            for arg in iter {
                body.push(arg);
            }
            let body = Ponga::Sexpr(body);

            if !first.is_sexpr() {
                return Err(RuntimeErr::Other(
                    "first argument to let must be an s-expr with identifiers"
                        .to_string()
                ));
            }
            
            let v = first.get_array()?;
            let mut names = Vec::new();
            let mut vals = Vec::new();
            for pair in v.into_iter().rev() {
                if !pair.is_sexpr() {
                    return Err(RuntimeErr::Other(
                        "each pair in let-deref must be S-Expr".to_string()
                    ));
                }
                let inner_arr = pair.get_array()?;
                if inner_arr.len() != 2 {
                    return Err(RuntimeErr::Other(
                        "let-deref requires pairs of S-Exprs".to_string()
                    ));
                }
                let mut inner_iter = inner_arr.into_iter();
                let id = match inner_iter.next().unwrap() {
                    Ponga::Identifier(name) => {
                        let r = self.get_identifier_obj_ref(&name)?; 
                        if !r.is_identifier() {
                            Err(RuntimeErr::Other(format!(
                                "identifiers in let-deref must refer to identifiers (not {:?})", r
                            )))
                        } else {
                            r.clone().extract_name()
                        }
                    }
                    _ => Err(RuntimeErr::Other(format!(
                        "let-deref requires identifiers as first element of each pair"
                    ))),
                }?;
                let val = inner_iter.next().unwrap();

                names.push(id.clone());
                vals.push(val);
            }
            ins_stack.push(Instruction::PopEnv(None));
            ins_stack.push(Instruction::Eval(body));

            ins_stack.push(Instruction::PushEnv(names));
            for val in vals {
                ins_stack.push(Instruction::Eval(val));
            }

        }
        "let" => {
            if iter.len() < 2 {
                return Err(RuntimeErr::Other(
                    "let must have at least two arguments".to_string()
                ));
            }
            let first = iter.next().unwrap();
            let mut body = vec![Ponga::Identifier("begin".to_string())];
            for arg in iter {
                body.push(arg);
            }
            let body = Ponga::Sexpr(body);

            if !first.is_sexpr() {
                return Err(RuntimeErr::Other(
                    "first argument to let must be an s-expr with identifiers"
                        .to_string()
                ));
            }
            
            let v = first.get_array()?;
            let mut names = Vec::new();
            let mut vals = Vec::new();
            for pair in v.into_iter().rev() {
                if !pair.is_sexpr() {
                    return Err(RuntimeErr::Other(
                        "each pair in let must be S-Expr".to_string()
                    ));
                }
                let inner_arr = pair.get_array()?;
                if inner_arr.len() != 2 {
                    return Err(RuntimeErr::Other(
                        "let requires pairs of S-Exprs".to_string()
                    ));
                }
                let mut inner_iter = inner_arr.into_iter();
                let id = inner_iter.next().unwrap().extract_name()?;
                let val = inner_iter.next().unwrap();

                names.push(id.clone());
                vals.push(val);
            }
            ins_stack.push(Instruction::PopEnv(None));
            ins_stack.push(Instruction::Eval(body));

            ins_stack.push(Instruction::PushEnv(names));
            for val in vals {
                ins_stack.push(Instruction::Eval(val));
            }
        }
        "define" => {
            if iter.len() != 2 {
                return Err(RuntimeErr::Other(
                    "define must have two arguments".to_string()
                ));
            }
            let name = iter.next().unwrap();
            let val = iter.next().unwrap();

            if name.is_identifier() {
                ins_stack.push(Instruction::Define(name.extract_name()?));
                ins_stack.push(Instruction::Eval(val));
                continue;
            } else if name.is_sexpr() {
                let v = name.get_array()?;
                if v.len() < 1 {
                    return Err(RuntimeErr::Other(
                        "define first argument must be an identifier or
                         an S-Expr of at least one identifier".to_string()
                    ));
                }
                let mut sexpr_iter = v.into_iter();
                let new_name = sexpr_iter.next().unwrap();
                let other_args = sexpr_iter.collect();

                // Define new_name by the lambda created from the other args
                ins_stack.push(Instruction::Define(new_name.extract_name()?));
                
                
                let new_sexpr = Sexpr(vec![Identifier("lambda".to_string()),
                                           Sexpr(other_args),
                                           val]);
                ins_stack.push(Instruction::Eval(new_sexpr));
                continue;
            } else {
                return Err(RuntimeErr::Other(
                    "define first argument must be an identifier or
                     an S-Expr of identifiers".to_string()
                ));

            }
        }
        "defmacro" => {
            if iter.len() != 2 {
                return Err(RuntimeErr::Other(
                    "defmacro must have two arguments".to_string()
                ));
            }
            let name = iter.next().unwrap();
            let val = iter.next().unwrap();

            if name.is_sexpr() {
                let v = name.get_array()?;
                if v.len() < 1 {
                    return Err(RuntimeErr::Other(
                        "defmacro first arg must be sexpr".to_string()
                    ));
                }
                let mut sexpr_iter = v.into_iter();
                let new_name = sexpr_iter.next().unwrap();
                let other_args = sexpr_iter.collect();

                // Define new_name by the lambda created from the other args
                ins_stack.push(Instruction::Define(new_name.extract_name()?));
                
                
                let new_sexpr = Sexpr(vec![Identifier("mac".to_string()),
                                           Sexpr(other_args),
                                           val]);
                ins_stack.push(Instruction::Eval(new_sexpr));
                continue;
            } else {
                return Err(RuntimeErr::Other(
                    "define first argument must be an identifier or
                     an S-Expr of identifiers".to_string()
                ));

            }
        }
        "set!" => {
            if iter.len() != 2 {
                return Err(RuntimeErr::Other(
                    "set! must have two arguments".to_string()
                ));
            }
            let name = iter.next().unwrap();
            let val = iter.next().unwrap();
            if !name.is_identifier() {
                return Err(RuntimeErr::Other(
                    "set! first argument must be an identifier".to_string()
                ));
            }

            ins_stack.push(Instruction::Set(name.extract_name()?));
            ins_stack.push(Instruction::Eval(val));
            continue;
        }
        "set-deref!" => {
            if iter.len() != 2 {
                return Err(RuntimeErr::Other(
                    "set! must have two arguments".to_string()
                ));
            }
            let name = match iter.next().unwrap() {
                Ponga::Identifier(name) => {
                    let r = self.get_identifier_obj_ref(&name)?; 
                    if !r.is_identifier() {
                        Err(RuntimeErr::Other(format!(
                            "identifier in set-deref! must refer to an identifier (not {:?})", r
                        )))
                    } else {
                        r.clone().extract_name()
                    }
                }
                _ => Err(RuntimeErr::Other(format!(
                    "set-deref! requires an identifier as argument"
                ))),
            }?;
            let val = iter.next().unwrap();

            ins_stack.push(Instruction::Set(name));
            ins_stack.push(Instruction::Eval(val));
            continue;
        }
        other => {
            let ref_obj = self.get_identifier_obj_ref(other)?;
            if ref_obj.is_func() {
                let n_args = iter.len();
                ins_stack.push(Instruction::Call(n_args));
                data_stack.push(ref_obj.clone());
                if ref_obj.is_macro() {
                    for arg in iter {
                        let val = arg.deep_copy(&self);
                        data_stack.push(val);
                    }
                } else {
                    // Call this func with the rest of thingies as args
                    for arg in iter {
                        ins_stack.push(Instruction::Eval(arg));
                    }
                }
                continue;
            } else {
                return Err(RuntimeErr::TypeError(
                    format!("Expected function, received {:?}", ref_obj)
                ));
            }
        }
    }
    cfunc@CFunc(_, _, _,) => {
        let n_args = iter.len();
        ins_stack.push(Instruction::Call(n_args));
        for arg in iter {
            ins_stack.push(Instruction::Eval(arg));
        }
        data_stack.push(cfunc);
        continue;
    }
    mfunc@MFunc(_, _) => {
        let n_args = iter.len();
        ins_stack.push(Instruction::Call(n_args));
        for arg in iter {
            let val = arg.deep_copy(&self);
            data_stack.push(val);
        }
        data_stack.push(mfunc);
        continue;
    }
    hfunc@HFunc(_) => {
        let n_args = iter.len();
        ins_stack.push(Instruction::Call(n_args));
        for arg in iter {
            ins_stack.push(Instruction::Eval(arg));
        }
        data_stack.push(hfunc);
        continue;
    }
    sexpr@Sexpr(_) => {
        // Evaluate the first arg and then call
        let n_args = iter.len();
        ins_stack.push(Instruction::Call(n_args));
        for arg in iter {
            ins_stack.push(Instruction::Eval(arg));
        }
        ins_stack.push(Instruction::Eval(sexpr));
        continue;
    }
    _ => return Err(RuntimeErr::TypeError(
        format!("Expected function, received {:?}", func)
    )),
}
                        }
                        Identifier(s) => {
                            let ref_obj = self.get_identifier_obj_ref(&s)?;
                            if !ref_obj.is_identifier() {
                                ins_stack.push(Instruction::Eval(ref_obj.clone()));
                            } else {
                                data_stack.push(ref_obj.clone());
                            }
                        }
                        _ => {
                            data_stack.push(val);
                        }
                    }
                }
            }
            if ins_stack.len() == 0 {
                break;
            }
        }
        pop_or(&mut data_stack)
    }

    pub fn is_func(&self, pong: &Ponga) -> bool {
        match pong {
            Ponga::HFunc(_) => true,
            Ponga::CFunc(_, _, _) => true,
            Ponga::MFunc(_, _) => true,
            Ponga::Identifier(s) => {
                let f = self.get_identifier_obj_ref(s).unwrap();
                f.is_func()
            }
            Ponga::Ref(id) => {
                let obj = self.get_id_obj_ref(*id).unwrap().borrow().unwrap();
                obj.inner().is_func()
            }
            _ => false,
        }
    }

    pub fn is_list(&self, pong: &Ponga) -> bool {
        match pong {
            Ponga::List(_) => true,
            Ponga::Identifier(s) => {
                let f = self.get_identifier_obj_ref(s).unwrap();
                self.is_list(f)
            }
            Ponga::Ref(id) => {
                let obj = self.get_id_obj_ref(*id).unwrap().borrow().unwrap();
                obj.inner().is_list()
            }
            _ => false,
        }
    }

    pub fn is_vector(&self, pong: &Ponga) -> bool {
        match pong {
            Ponga::Array(_) => true,
            Ponga::Identifier(s) => {
                let f = self.get_identifier_obj_ref(s).unwrap();
                self.is_vector(f)
            }
            Ponga::Ref(id) => {
                let obj = self.get_id_obj_ref(*id).unwrap().borrow().unwrap();
                obj.inner().is_vector()
            }
            _ => false,
        }
    }

    pub fn is_char(&self, pong: &Ponga) -> bool {
        match pong {
            Ponga::Char(_) => true,
            Ponga::Identifier(s) => {
                let f = self.get_identifier_obj_ref(s).unwrap();
                self.is_char(f)
            }
            Ponga::Ref(id) => {
                let obj = self.get_id_obj_ref(*id).unwrap().borrow().unwrap();
                obj.inner().is_char()
            }
            _ => false,
        }
    }

    pub fn is_number(&self, pong: &Ponga) -> bool {
        match pong {
            Ponga::Number(_) => true,
            Ponga::Identifier(s) => {
                let f = self.get_identifier_obj_ref(s).unwrap();
                self.is_number(f)
            }
            Ponga::Ref(id) => {
                let obj = self.get_id_obj_ref(*id).unwrap().borrow().unwrap();
                obj.inner().is_number()
            }
            _ => false,
        }
    }

    pub fn is_string(&self, pong: &Ponga) -> bool {
        match pong {
            Ponga::String(_) => true,
            Ponga::Identifier(s) => {
                let f = self.get_identifier_obj_ref(s).unwrap();
                self.is_string(f)
            }
            Ponga::Ref(id) => {
                let obj = self.get_id_obj_ref(*id).unwrap().borrow().unwrap();
                obj.inner().is_string()
            }
            _ => false,
        }
    }

    pub fn is_symbol(&self, pong: &Ponga) -> bool {
        match pong {
            Ponga::Symbol(_) => true,
            Ponga::Identifier(s) => {
                let f = self.get_identifier_obj_ref(s).unwrap();
                self.is_symbol(f)
            }
            Ponga::Ref(id) => {
                let obj = self.get_id_obj_ref(*id).unwrap().borrow().unwrap();
                obj.inner().is_symbol()
            }
            _ => false,
        }
    }

    pub fn ponga_to_string(&self, ponga: &Ponga) -> String {
        match ponga {
            Ponga::Number(_) => format!("{}", ponga),
            Ponga::String(_) => format!("{}", ponga),
            Ponga::False => format!("{}", ponga),
            Ponga::True => format!("{}", ponga),
            Ponga::Char(_) => format!("{}", ponga),
            Ponga::Null => format!("{}", ponga),
            Ponga::Symbol(_) => format!("'{}", ponga),
            Ponga::HFunc(id) => format!("{}", FUNCS[*id].0),
            Ponga::CFunc(args, _, stateid) => {
                let obj = self.get_id_obj_ref(*stateid).unwrap();
                let obj_ref = obj.borrow().unwrap();
                let state = obj_ref.inner();
                let state_str = self.ponga_to_string_no_id(state);
                format!("Compound function with args {:#?}, state {}", args, state_str)
            }
            Ponga::MFunc(args, _) => {
                format!("Macro with args {:#?}", args)
            }
            Ponga::Sexpr(a) => format!("({})", a.iter().map(|p| self.ponga_to_string(p)).format(" ")),
            Ponga::Identifier(s) => {
                let obj = match self.get_identifier_obj_ref(s) {
                    Ok(val) => val,
                    Err(_) => {
                        return format!("{}", s);
                    }
                };
                self.ponga_to_string(obj)
            }
            Ponga::Ref(id) => {
                let obj = self.get_id_obj_ref(*id).unwrap().borrow().unwrap();
                self.ponga_to_string(obj.inner())
            }
            Ponga::Object(o) => {
                format!(
                    "[{}]",
                    o.iter()
                        .map(|(k, v)| format!("{}: {}", k.to_string(), self.ponga_to_string(v)))
                        .format(", ")
                )
            }
            Ponga::Array(arr) => {
                format!("#({})", arr.iter().map(|p| self.ponga_to_string(p)).format(" "))
            }
            Ponga::List(l) => {
                format!("'({})", l.iter().map(|p| self.ponga_to_string(p)).format(" "))
            }
        }
    }

    pub fn ponga_to_string_no_id(&self, ponga: &Ponga) -> String {
        match ponga {
            Ponga::Number(_) => format!("{}", ponga),
            Ponga::String(_) => format!("{}", ponga),
            Ponga::False => format!("{}", ponga),
            Ponga::True => format!("{}", ponga),
            Ponga::Char(_) => format!("{}", ponga),
            Ponga::Null => format!("{}", ponga),
            Ponga::Symbol(_) => format!("'{}", ponga),
            Ponga::HFunc(id) => format!("{}", FUNCS[*id].0),
            Ponga::CFunc(args, _, stateid) => {
                let obj = self.get_id_obj_ref(*stateid).unwrap();
                let obj_ref = obj.borrow().unwrap();
                let state = obj_ref.inner();
                let state_str = self.ponga_to_string_no_id(state);
                format!("Compound function with args {:#?}, state {}", args, state_str)
            }
            Ponga::MFunc(args, _) => {
                format!("Macro with args {:#?}", args)
            }
            Ponga::Sexpr(a) => format!("({})", a.iter().map(|p| self.ponga_to_string_no_id(p)).format(" ")),
            Ponga::Identifier(s) => {
                format!("{}", s)
            }
            Ponga::Ref(id) => {
                let obj = self.get_id_obj_ref(*id).unwrap().borrow().unwrap();
                self.ponga_to_string_no_id(obj.inner())
            }
            Ponga::Object(o) => {
                format!(
                    "[{}]",
                    o.iter()
                        .map(|(k, v)| format!("{}: {}", k.to_string(), self.ponga_to_string_no_id(v)))
                        .format(", ")
                )
            }
            Ponga::Array(arr) => {
                format!("#({})", arr.iter().map(|p| self.ponga_to_string_no_id(p)).format(" "))
            }
            Ponga::List(l) => {
                format!("'({})", l.iter().map(|p| self.ponga_to_string_no_id(p)).format(" "))
            }
        }
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
                    println!("{}", self.ponga_to_string(&last));
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
                println!("Program returned: {}", runtime.ponga_to_string(v));
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
