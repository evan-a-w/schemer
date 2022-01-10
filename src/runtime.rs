use crate::env::*;
use crate::instructions::*;
use crate::parser::*;
use crate::stdlib::*;
use crate::types::*;
use gc_rs::{Gc, GcRefMut, Trace};
use std::collections::HashMap;
use std::collections::LinkedList;
use std::fs::File;
use std::io::prelude::*;

pub const MAX_STACK_SIZE: usize = 100_000;

pub struct Runtime {
    pub env: Env,
}

impl Runtime {
    pub fn new() -> Self {
        let env = Env::new();

        for (i, val) in FUNCS.iter().enumerate() {
            env.insert_furthest(val.0.to_string(), Ponga::HFunc(i));
        }

        let mut res = Self { env };

        let stdlib_scm = include_str!("stdlib.scm");
        res.run_str(stdlib_scm).unwrap();

        res
    }

    pub fn bind_global(&mut self, s: String, pong: Ponga) {
        self.env.insert_furthest(s, pong);
    }

    pub fn deep_copy(&mut self, pong: Ponga) -> Ponga {
        match pong {
            Ponga::Ref(val) => (*val).clone(),
            id@Ponga::Identifier(_) => self.id_or_ref_peval(id).unwrap(),
            _ => pong,
        }
    }

    pub fn get_identifier_obj_ref(&self, identifier: &str) -> RunRes<&Ponga> {
        self.env
            .get(identifier)
            .ok_or(RuntimeErr::ReferenceError(format!(
                "Reference to {} not found",
                identifier
            )))
    }

    pub fn set_identifier(&mut self, identifier: &str, pong: Ponga) -> RunRes<()> {
        self.env
            .set(identifier, pong)
            .ok_or(RuntimeErr::ReferenceError(format!(
                "Reference to {} not found",
                identifier
            )))
    }

    pub fn clone_ref(&mut self, pong: Ponga) -> RunRes<Ponga> {
        match pong {
            Ponga::Ref(gc) => Ok((*gc).clone()),
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
        use Instruction::*;
        use Ponga::*;
        let mut data_stack = vec![];
        let mut ins_stack = vec![Instruction::Eval(pong)];
        loop {
            if ins_stack.is_empty() {
                break;
            }
            let ins = ins_stack.pop().unwrap();
            match ins {
                PopEnv(b) => {
                    let map = self.env.pop();
                    if b {
                        for (k, v) in &*map {
                            self.env.set(k, v.clone());
                        }
                    }
                }
                PushEnv(names) => {
                    let mut map = HashMap::new();
                    for name in names {
                        map.insert(name, pop_or(&mut data_stack)?);
                    }
                    self.env.push(Gc::new(map));
                }
                PopStack => {
                    data_stack.pop();
                }
                Define(s) => {
                    self.env.insert_furthest(s, pop_or(&mut data_stack)?);
                    data_stack.push(Ponga::Null);
                }
                Set(s) => {
                    self.env.set(&s, pop_or(&mut data_stack)?).ok_or(
                        RuntimeErr::ReferenceError(format!("Reference to {} not found", s)),
                    )?;
                    data_stack.push(Ponga::Null);
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
                                "Expected function, not {}",
                                func
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
                            ins_stack.push(Instruction::CollectObject(o.keys().cloned().collect()));
                            for (_, v) in o {
                                ins_stack.push(Instruction::Eval(v));
                            }
                        }
                        Sexpr(vals) => {
                            if vals.len() < 1 {
                                return Err(RuntimeErr::Other("Empty sexpr".to_string()));
                            }

                            let mut iter = vals.into_iter();
                            let func = iter.next().unwrap();
                            match &func {
                                Identifier(s) => match s.as_str() {
                                    "lambda" => {
                                        if iter.len() != 2 {
                                            return Err(RuntimeErr::Other(
                                                "Wrong number of arguments for lambda".to_string(),
                                            ));
                                        }
                                        let args =
                                            iter.next().unwrap().extract_names_from_sexpr()?;
                                        let body = iter.next().unwrap();
                                        if !body.is_sexpr() {
                                            return Err(RuntimeErr::Other(
                                                "Wrong type for lambda body".to_string(),
                                            ));
                                        }
                                        let state_map = Gc::new(self.env.copy());
                                        let pushed = CFunc(args, Gc::new(body), state_map);
                                        data_stack.push(pushed);
                                        continue;
                                    }
                                    "open-lambda" => {
                                        if iter.len() != 2 {
                                            return Err(RuntimeErr::Other(
                                                "Wrong number of arguments for open lambda".to_string(),
                                            ));
                                        }
                                        let args =
                                            iter.next().unwrap().extract_names_from_sexpr()?;
                                        let body = iter.next().unwrap();
                                        if !body.is_sexpr() {
                                            return Err(RuntimeErr::Other(
                                                "Wrong type for lambda body".to_string(),
                                            ));
                                        }
                                        let pushed = MFunc(args, Gc::new(body));
                                        data_stack.push(pushed);
                                        continue;
                                    }
                                    "define" => {
                                        if iter.len() != 2 {
                                            return Err(RuntimeErr::Other(
                                                "Wrong number of arguments for define".to_string(),
                                            ));
                                        }
                                        let name = iter.next().unwrap();
                                        if name.is_sexpr() {
                                            let mut arr = name.get_array()?.into_iter();
                                            let name = arr.next().unwrap().extract_name()?;
                                            let names =
                                                Ponga::extract_names_from_vec(arr.collect())?;
                                            let names = names.into_iter().map(Identifier).collect();
                                            let vec = vec![
                                                Identifier("define".to_string()),
                                                Identifier(name),
                                                Sexpr(vec![
                                                    Identifier("lambda".to_string()),
                                                    Sexpr(names),
                                                    iter.next().unwrap(),
                                                ]),
                                            ];
                                            ins_stack.push(Instruction::Eval(Sexpr(vec)));
                                        } else {
                                            let val = iter.next().unwrap();
                                            ins_stack
                                                .push(Instruction::Define(name.extract_name()?));
                                            ins_stack.push(Instruction::Eval(val));
                                        }
                                        data_stack.push(Ponga::Null);
                                        continue;
                                    }
                                    "if" => {
                                        if iter.len() != 3 {
                                            return Err(RuntimeErr::Other(
                                                "if must have three arguments".to_string(),
                                            ));
                                        }
                                        // Can make this better if we push it later but should be fine for now
                                        let cond = self.id_or_ref_peval(iter.next().unwrap())?;
                                        let cond = self.eval(cond)?;
                                        let val = if cond != Ponga::False {
                                            iter.nth(0).unwrap()
                                        } else {
                                            iter.nth(1).unwrap()
                                        };
                                        ins_stack.push(Instruction::Eval(val));
                                        continue;
                                    }
                                    "copy" => {
                                        if iter.len() != 1 {
                                            return Err(RuntimeErr::Other(
                                                "copy must have one argument".to_string(),
                                            ));
                                        }

                                        let val = self.deep_copy(iter.next().unwrap());
                                        data_stack.push(val);
                                        continue;
                                    }
                                    "$EVAL" | "eval" => {
                                        if iter.len() != 1 {
                                            return Err(RuntimeErr::Other(
                                                "$EVAL must have one argument".to_string(),
                                            ));
                                        }
                                        let val = self.deep_copy(iter.next().unwrap());
                                        ins_stack.push(Instruction::Eval(val));
                                        continue;
                                    }
                                    "$FLIP" | "code<->data" | "data<->code" => {
                                        if iter.len() != 1 {
                                            return Err(RuntimeErr::Other(
                                                "$FLIP must have one argument".to_string(),
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
                                        data_stack.push(val.flip_code_vals(self));
                                        continue;
                                    }
                                    "$FLIP-EVAL" | "code<->data.eval" | "data<->code.eval" => {
                                        if iter.len() != 1 {
                                            return Err(RuntimeErr::Other(
                                                "$FLIP-EVAL must have one argument".to_string(),
                                            ));
                                        }
                                        let val = iter.next().unwrap();
                                        ins_stack
                                            .push(Instruction::Eval(val.flip_code_vals(self)));
                                        continue;
                                    }
                                    "$EVAL-FLIP-EVAL"
                                    | "eval.code<->data.eval"
                                    | "eval.data<->code.eval" => {
                                        if iter.len() != 1 {
                                            return Err(RuntimeErr::Other(
                                                "eval.code<->list.eval must have one argument"
                                                    .to_string(),
                                            ));
                                        }
                                        let val = self.eval(iter.next().unwrap())?;
                                        ins_stack
                                            .push(Instruction::Eval(val.flip_code_vals(self)));
                                        continue;
                                    }
                                    "quote" => {
                                        if iter.len() != 1 {
                                            return Err(RuntimeErr::Other(
                                                "quote must have one argument".to_string(),
                                            ));
                                        }
                                        let val = self.deep_copy(iter.next().unwrap());
                                        let val = match val {
                                            Ponga::Sexpr(arr) => {
                                                Ponga::List(arr.into_iter().collect())
                                            }
                                            Ponga::Identifier(s) => Ponga::Symbol(s),
                                            val => val,
                                        };
                                        data_stack.push(val);
                                        continue;
                                    }
                                    "$DELAY" => {
                                        for i in iter {
                                            data_stack.push(i);
                                        }
                                        continue;
                                    }
                                    "sym->id" => {
                                        if iter.len() != 1 {
                                            return Err(RuntimeErr::Other(
                                                "sym->id must have one argument".to_string(),
                                            ));
                                        }
                                        let val = self.id_or_ref_peval(iter.next().unwrap())?;
                                        data_stack.push(self.id_or_ref_peval(
                                            Ponga::Identifier(val.get_symbol_string()?),
                                        )?);
                                        continue;
                                    }
                                    "echo" => {
                                        for val in iter {
                                            data_stack.push(val);
                                        }
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
                                    "let" => {
                                        if iter.len() < 2 {
                                            return Err(RuntimeErr::Other(
                                                "let must have at least two arguments".to_string(),
                                            ));
                                        }
                                        let first = iter.next().unwrap();
                                        let mut iter = iter.rev();
                                        let (names, vals) = first.extract_names_vals_from_sexpr()?;
                                        ins_stack.push(Instruction::PopEnv(false));
                                        ins_stack.push(Instruction::Eval(iter.next().unwrap()));
                                        for i in iter {
                                            ins_stack.push(Instruction::PopStack);
                                            ins_stack.push(Instruction::Eval(i));
                                        }
                                        ins_stack.push(Instruction::PushEnv(names));
                                        for val in vals {
                                            ins_stack.push(Instruction::Eval(val));
                                        }
                                        continue;
                                    }
                                    "let-deref" => {
                                        if iter.len() < 2 {
                                            return Err(RuntimeErr::Other(
                                                "let must have at least two arguments".to_string(),
                                            ));
                                        }
                                        let first = iter.next().unwrap();
                                        let mut iter = iter.rev();
                                        let (names, vals) = first.extract_deref(self)?;
                                        ins_stack.push(Instruction::PopEnv(false));
                                        ins_stack.push(Instruction::Eval(iter.next().unwrap()));
                                        for i in iter {
                                            ins_stack.push(Instruction::PopStack);
                                            ins_stack.push(Instruction::Eval(i));
                                        }
                                        ins_stack.push(Instruction::PushEnv(names));
                                        for val in vals {
                                            ins_stack.push(Instruction::Eval(val));
                                        }
                                        continue;
                                    }
                                    "set!" => {
                                        if iter.len() != 2 {
                                            return Err(RuntimeErr::Other(
                                                "set! must have two arguments".to_string(),
                                            ));
                                        }
                                        let name = iter.next().unwrap().extract_name()?;
                                        let val = iter.next().unwrap();
                                        ins_stack.push(Instruction::Set(name));
                                        ins_stack.push(Instruction::Eval(val));
                                        continue;
                                    }
                                    "defmacro" => {
                                        if iter.len() != 2 {
                                            return Err(RuntimeErr::Other(
                                                "defmacro must have two arguments".to_string()
                                            ));
                                        }
                                        let name = iter.next().unwrap();
                                        let val = iter.next().unwrap();

                                        if !name.is_sexpr() {
                                            return Err(RuntimeErr::Other(
                                                "define first argument must be an identifier or
                                                 an S-Expr of identifiers".to_string()
                                            ));
                                        }
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

                                        data_stack.push(MFunc(cargs, Gc::new(body)));
                                        continue;
                                    }
                                    "set-deref!" => {
                                        if iter.len() != 2 {
                                            return Err(RuntimeErr::Other(
                                                "set-deref! must have two arguments".to_string()
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
                                    _ => {},
                                }
                                _ => {},
                            }
                            let func = self.eval(func)?;

                            match func {
                                HFunc(id) => {
                                    ins_stack.push(Instruction::Call(iter.len()));
                                    data_stack.push(HFunc(id));
                                    for arg in iter {
                                        ins_stack.push(Instruction::Eval(arg));
                                    }
                                }
                                CFunc(names, sexpr, state) => {
                                    self.env.push(state.clone());
                                    ins_stack.push(Instruction::PopEnv(true));

                                    let len = names.len();

                                    ins_stack.push(Instruction::PopEnv(false));
                                    ins_stack.push(Instruction::Eval((*sexpr).clone()));
                                    ins_stack.push(Instruction::PushEnv(names));

                                    for i in 0..len {
                                        ins_stack.push(Instruction::Eval(iter.next().ok_or(
                                            RuntimeErr::Other(format!(
                                                "Expected {} arguments, got {}",
                                                len, i
                                            )),
                                        )?));
                                    }
                                }
                                MFunc(names, sexpr) => {
                                    ins_stack.push(Instruction::PopEnv(false));
                                    let len = names.len();
                                    ins_stack.push(Instruction::Eval((*sexpr).clone()));
                                    ins_stack.push(Instruction::PushEnv(names));

                                    let mut iter = iter.rev();
                                    data_stack.push(MFunc(vec![], sexpr));
                                    for i in 0..len {
                                        let next = iter.next().ok_or(RuntimeErr::Other(
                                            format!("Expected {} arguments, got {}", len, i),
                                        ))?;
                                        data_stack.push(next);
                                    }
                                }
                                _ => {
                                    return Err(RuntimeErr::Other(format!(
                                        "First element of sexpr `{}` is not function (rest {:?})",
                                        func, iter
                                    )));
                                }
                            }
                        }
                        Identifier(_) => ins_stack.push(Instruction::Eval(self.id_or_ref_peval(pong)?)),
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
            println!("Unexpected tokens: {:?}", parsed.0);
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
    vec.pop()
        .ok_or(RuntimeErr::Other(format!("Expected value in stack")))
}

pub fn is_keyword(pong: &Ponga) -> bool {
    match pong {
        Ponga::Identifier(s) => KEYWORDS.iter().any(|x| x == &s),
        _ => false,
    }
}
