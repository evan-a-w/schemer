use crate::gc::*;
use crate::gc_obj::*;
use crate::parser::*;
use crate::stdlib::*;
use crate::types::*;
use crate::instructions::*;
use itertools::Itertools;
use std::collections::HashMap;
use std::collections::LinkedList;
use std::fs::File;
use std::io::prelude::*;
use std::ptr::{self, NonNull};

pub type Namespace = HashMap<String, Ponga>;

pub type PriorityNamespace = HashMap<String, Vec<Ponga>>;

pub struct Runtime {
    pub globals: Namespace,
    pub global_funcs: Namespace,
    pub locals: PriorityNamespace,
    pub gc: Gc,
}

pub enum WhereVar {
    Global,
    Local,
    GlobalFunc,
}

impl Runtime {
    pub fn new() -> Self {
        let mut global_funcs = Namespace::new();
        let mut gc = Gc::new();

        for (i, val) in FUNCS.iter().enumerate() {
            global_funcs.insert(val.0.to_string(), Ponga::HFunc(i));
        }

        let mut res = Self {
            globals: Namespace::new(),
            global_funcs,
            locals: PriorityNamespace::new(),
            gc,
        };

        // let stdlib_scm = include_str!("stdlib.scm");
        // res.run_str(stdlib_scm).unwrap();

        res
    }

    pub fn condense_locals(&self) -> HashMap<String, Ponga> {
        let mut res = HashMap::new();
        for (k, v) in self.locals.iter() {
            res.insert(k.clone(), v[v.len() - 1].clone());
        }
        res
    }

    pub fn bind_global(&mut self, s: String, pong: Ponga) {
        self.globals.insert(s, pong);
    }

    pub fn unbind_global(&mut self, s: &str) {
        self.globals.remove(s);
    }

    pub fn add_roots_to_gc(&mut self) {
        self.gc.roots = std::collections::HashSet::new();
        for v in self.globals.values() {
            match v {
                Ponga::Ref(id) => { self.gc.roots.insert(*id); },
                _ => (),
            }
        }
        for vec in self.locals.values() {
            for i in vec.iter() {
                match i {
                    Ponga::Ref(id) => { self.gc.roots.insert(*id); },
                    _ => (),
                }
            }
        }
    }

    pub fn collect_garbage(&mut self) {
        self.add_roots_to_gc();
        self.gc.collect_garbage();
        self.gc.roots.clear();
    }

    pub fn get_id_obj_ref(&self, id: Id) -> RunRes<&GcObj> {
        self.gc
            .ptrs
            .get(&id)
            .ok_or(RuntimeErr::ReferenceError(format!(
                "Reference {} not found",
                id
            )))
    }

    pub fn get_id_obj(&mut self, id: Id) -> RunRes<&mut GcObj> {
        self.gc
            .ptrs
            .get_mut(&id)
            .ok_or(RuntimeErr::ReferenceError(format!(
                "Reference {} not found",
                id
            )))
    }

    pub fn get_identifier_obj_ref(&self, identifier: &str) -> RunRes<&Ponga> {
        match self.locals.get(identifier) {
            Some(v) => {
                return Ok(&v[v.len() - 1])
            }
            None => (),
        }
        match self.globals.get(identifier) {
            Some(v) => return Ok(v),
            None => (),
        }
        match self.global_funcs.get(identifier) {
            Some(v) => Ok(v),
            None => Err(RuntimeErr::ReferenceError(format!(
                "Identifier {} not found",
                identifier
            )))
        }

    }

    pub fn set_identifier(&mut self, identifier: &str, pong: Ponga) -> RunRes<()> {
        match self.locals.get_mut(identifier) {
            Some(v) => {
                let l = v.len();
                v[l - 1] = pong;
                return Ok(());
            }
            None => (),
        }
        match self.globals.get_mut(identifier) {
            Some(v) => {
                *v = pong;
                return Ok(());
            }
            None => (),
        }
        Err(RuntimeErr::ReferenceError(format!(
                    "Identifier {} used in set is unknown", identifier
        )))
    }

    pub fn pop_local(&mut self, identifier: &str) -> Ponga {
        let vec = self.locals.get_mut(identifier).unwrap();
        let res = vec.pop().unwrap();
        if vec.len() == 0 {
            self.locals.remove(identifier);
        }
        res
    }

    pub fn pop_identifier_obj(&mut self, identifier: &str) -> RunRes<(Ponga, WhereVar)> {
        let entry = self.locals.get_mut(identifier);
        match entry {
            Some(v) => {
                let res = v.pop().unwrap();
                if v.is_empty() {
                    drop(v);
                    self.locals.remove(identifier);
                }
                return Ok((res, WhereVar::Local));
            }
            None => (),
        }
        match self.globals.remove_entry(identifier) {
            Some((_, v)) => return Ok((v, WhereVar::Global)),
            None => (),
        }
        match self.global_funcs.remove_entry(identifier) {
            Some((_, v)) => Ok((v, WhereVar::GlobalFunc)),
            None => Err(RuntimeErr::ReferenceError(format!(
                "Identifier {} not found",
                identifier
            )))
        }

    }

    pub fn push_whereval(&mut self, identifier: &str, pong: Ponga, wh: WhereVar) {
        match wh {
            WhereVar::Global => {
                self.globals.insert(identifier.to_string(), pong);
            }
            WhereVar::Local => {
                self.push_local(identifier, pong);
            }
            WhereVar::GlobalFunc => {
                self.global_funcs.insert(identifier.to_string(), pong);
            }
        }
    }

    pub fn push_local(&mut self, identifier: &str, pong: Ponga) {
        self.locals
            .entry(identifier.to_string())
            .or_insert(Vec::new())
            .push(pong);
    }

    pub fn id_or_ref_peval(&self, pong: Ponga) -> RunRes<Ponga> {
        match pong {
            Ponga::Ref(id) => {
                let ref_obj = self.get_id_obj_ref(id)?;
                let inner = ref_obj.borrow().unwrap();
                if inner.is_copy() {
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
                    Ok(Ponga::Identifier(s))
                }
            }
            _ => Ok(pong),
        }
    }

    pub fn state_to_string(&self) -> String {
        format!("Locals: {:?}\nGlobals: {:?}", self.locals, self.globals)
    }

    pub fn eval(&mut self, pong: Ponga) -> RunRes<Ponga> {
        use Ponga::*;
        let mut data_stack = vec![pong];
        let mut ins_stack = vec![Instruction::Eval];
        loop {
            println!("Data stack: {:?}", data_stack);
            println!("Ins stack: {:?}", ins_stack);
            println!("State: {}", self.state_to_string());
            println!("\n--------------\n");
            match ins_stack.pop().unwrap() {
                Instruction::Define(s) => {
                    self.bind_global(s, pop_or(&mut data_stack)?);
                    data_stack.push(Ponga::Null);
                }
                Instruction::Push(s) => self.push_local(&s, pop_or(&mut data_stack)?),
                Instruction::Pop(s) => { self.pop_local(&s); },
                Instruction::Set(s) => { 
                    self.set_identifier(&s, pop_or(&mut data_stack)?)?;
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
                Instruction::CollectSexpr(n) => {
                    let mut res = Vec::new();
                    for _ in 0..n {
                        res.push(pop_or(&mut data_stack)?);
                    }
                    data_stack.push(Ponga::Sexpr(res));
                }
                Instruction::Call(num_args) => {
                    let mut args = Vec::new();
                    let func = data_stack.pop().ok_or(
                        RuntimeErr::Other(
                            format!("Expected {} args for function", num_args)
                        )
                    )?;
                    for _ in 0..num_args {
                        args.push(data_stack.pop().ok_or(
                            RuntimeErr::Other(
                                format!("Expected {} args for function", num_args)
                            )
                        )?);
                    }
                    match func {
                        HFunc(id) => {
                            data_stack.push(FUNCS[id].1(self, args)?)
                        }
                        CFunc(args_names, sexpr_id, state_id) => {
                            let state_obj = self.get_id_obj_ref(state_id)?.clone();

                            // Pop the entire state after we're done evaluating
                            let ref_gc_obj = state_obj.borrow().unwrap();
                            let state_map_ref = ref_gc_obj.extract_map_ref().unwrap();
                            for (name, _) in state_map_ref.into_iter() {
                                ins_stack.push(Instruction::Pop(name.clone()));
                            } 

                            // Pop all of the args before popping state
                            for name in args_names.iter() {
                                ins_stack.push(Instruction::Pop(name.clone()));
                            } 

                            // Push S-Expr to be evaluated
                            let sexpr_obj = self.get_id_obj_ref(sexpr_id)?;
                            let sexpr = sexpr_obj.borrow().unwrap().clone();
                            data_stack.push(sexpr);
                            ins_stack.push(Instruction::Eval);

                            let state_map = state_obj.borrow()
                                                     .unwrap()
                                                     .clone()
                                                     .extract_map()
                                                     .unwrap();

                            // Push the entire state, can just do now, don't need to delay
                            for (name, val) in state_map.into_iter() {
                                self.push_local(&name, val);
                            } 

                            // Push all of the names and arg values onto the stack
                            for (name, val) in args_names.iter().zip(args.into_iter()) {
                                self.push_local(&name, val);
                            } 
                        } 
                        _ => return Err(RuntimeErr::TypeError(
                            format!("Expected function, received {:?}", func)
                        )),
                    }
                }
                Instruction::Eval => {
                    match pop_or(&mut data_stack)? {
                        Ponga::Array(v) => {
                            ins_stack.push(Instruction::CollectArray(v.len()));
                            for i in v.into_iter() {
                                data_stack.push(i);
                                ins_stack.push(Instruction::Eval);
                            }
                        }
                        Ponga::List(v) => {
                            ins_stack.push(Instruction::CollectList(v.len()));
                            for i in v.into_iter() {
                                data_stack.push(i);
                                ins_stack.push(Instruction::Eval);
                            }
                        }
                        Ponga::Object(v) => {
                            ins_stack.push(Instruction::CollectObject(
                                v.keys().cloned().collect()
                            ));
                            for (_, i) in v.into_iter() {
                                data_stack.push(i);
                                ins_stack.push(Instruction::Eval);
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
                    "if must have at three arguments".to_string()
                ));
            }
            // Can make this better if we push it later but should be fine for now
            let cond = self.eval(iter.next().unwrap())?;
            ins_stack.push(Instruction::Eval);
            if cond != Ponga::False {
                data_stack.push(iter.nth(0).unwrap()); 
            } else {
                data_stack.push(iter.nth(1).unwrap()); 
            }
            continue;
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

            let new_state = self.condense_locals();
            let state_id = self.gc.add_obj(Ponga::Object(new_state));
            let body_id = self.gc.add_obj(body);

            data_stack.push(CFunc(cargs, body_id, state_id));
            continue;
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
                ins_stack.push(Instruction::Eval);
                data_stack.push(val);
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
                data_stack.push(new_sexpr);
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
            ins_stack.push(Instruction::Eval);
            data_stack.push(val);
            continue;
        }
        other => {
            // must refer to a CFunc or HFunc
            let ref_obj = self.get_identifier_obj_ref(other)?;
            if !ref_obj.is_func() {
                return Err(RuntimeErr::TypeError(
                    format!("Expected function, received {:?}", ref_obj)
                ));
            }

            let n_args = iter.len();
            // Call this func with the rest of thingies as args
            for arg in iter.rev() {
                data_stack.push(arg);
            }
            data_stack.push(ref_obj.clone());
            ins_stack.push(Instruction::Call(n_args));
            continue;
        }
    }
    cfunc@CFunc(_, _, _,) => {
        let n_args = iter.len();
        for arg in iter.rev() {
            data_stack.push(arg);
        }
        data_stack.push(cfunc);
        ins_stack.push(Instruction::Call(n_args));
        continue;
    }
    hfunc@HFunc(_) => {
        let n_args = iter.len();
        for arg in iter.rev() {
            data_stack.push(arg);
        }
        data_stack.push(hfunc);
        ins_stack.push(Instruction::Call(n_args));
        continue;
    }
    sexpr@Sexpr(_) => {
        // Evaluate the first arg and then call
        let n_args = iter.len();
        for arg in iter.rev() {
            data_stack.push(arg);
        }
        ins_stack.push(Instruction::Call(n_args));
        data_stack.push(sexpr);
        ins_stack.push(Instruction::Eval);
        continue;
    }
    _ => return Err(RuntimeErr::TypeError(
        format!("Expected function, received {:?}", func)
    )),
}
                        }
                        val => data_stack.push(val),
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
            Ponga::Number(n) => format!("{}", ponga),
            Ponga::String(s) => format!("{}", ponga),
            Ponga::False => format!("{}", ponga),
            Ponga::True => format!("{}", ponga),
            Ponga::Char(c) => format!("{}", ponga),
            Ponga::Null => format!("{}", ponga),
            Ponga::Symbol(s) => format!("{}", ponga),
            Ponga::HFunc(id) => format!("Internal function: {}", FUNCS[*id].0),
            Ponga::CFunc(args, _, stateid) => {
                let obj = self.get_id_obj_ref(*stateid).unwrap();
                let obj_ref = obj.borrow().unwrap();
                let state = obj_ref.inner();
                let state_str = self.ponga_to_string(state);
                format!("Compound function with args {:#?}, state {}", args, state_str)
            }
            Ponga::Sexpr(a) => format!("S-expression: `{:?}`", a),
            Ponga::Identifier(s) => {
                let obj = match self.get_identifier_obj_ref(s) {
                    Ok(val) => val,
                    Err(_) => {
                        return format!("Identifier {} (not found)", s);
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
                    "'({})",
                    o.iter()
                        .map(|(k, v)| format!("'({} {})", k.to_string(), self.ponga_to_string(v)))
                        .format(" ")
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

    pub fn run_str(&mut self, s: &str) -> RunRes<()> {
        let parsed = pongascript_parser(s)?;
        if parsed.0.len() != 0 {
            return Err(RuntimeErr::ParseError(format!(
                "Unexpected tokens: {:?}",
                parsed.0
            )));
        }
        let evald = parsed
            .1
            .into_iter()
            .map(|x| self.eval(x))
            .collect::<Vec<RunRes<Ponga>>>();
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
        Ok(v) => println!("Program returned: {}", runtime.ponga_to_string(v)),
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
