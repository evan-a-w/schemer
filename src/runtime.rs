use crate::gc::*;
use crate::gc_obj::*;
use crate::parser::*;
use crate::stdlib::*;
use crate::types::*;
use itertools::Itertools;
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::collections::LinkedList;
use std::fs::File;
use std::io::prelude::*;
use std::ptr::{self, NonNull};

pub const DEBUG_PRINT: bool = false;

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

        Self {
            globals: Namespace::new(),
            global_funcs,
            locals: PriorityNamespace::new(),
            gc,
        }
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

    pub fn pop_local(&mut self, identifier: &str) {
        let vec = self.locals.get_mut(identifier).unwrap();
        vec.pop();
        if vec.len() == 0 {
            self.locals.remove(identifier);
        }
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

    pub fn func_eval(&mut self, pong: &Ponga, args: Vec<Ponga>) -> RunRes<Ponga> {
        match pong {
            Ponga::HFunc(id) => {
                if *id >= FUNCS.len() {
                    return Err(RuntimeErr::ReferenceError(format!(
                        "Function {} not found",
                        id
                    )));
                }
                FUNCS[*id].1(self, args)
            }
            Ponga::CFunc(names, id, state) => {
                args_assert_len(&args, names.len(), "func")?;
                let args = eval_args(self, args)?;
                let mut sexpr = self
                    .get_id_obj(*id)?
                    .borrow()
                    .unwrap()
                    .inner()
                    .clone();

                for (k, v) in state.iter() {
                    self.push_local(k, v.clone());
                }

                for (name, arg) in names.iter().zip(args.into_iter()) {
                    self.push_local(name, arg);
                }
                
                let res = self.eval(sexpr)?;

                for name in names.iter() {
                    self.pop_local(name);
                }

                for name in state.keys() {
                    self.pop_local(name);
                }

                Ok(res)
            }
            Ponga::Identifier(identifier) => {
                let func = self.get_identifier_obj_ref(identifier)?.clone();
                self.func_eval(&func, args)
            }
            Ponga::Ref(id) => {
                let mut obj = self.get_id_obj(*id)?.clone();
                let res = self.func_eval(obj.borrow_mut().unwrap().inner(), args)?;
                Ok(res)
            }
            _ => Err(RuntimeErr::TypeError(
                "Using non-callable value as function".to_string(),
            )),
        }
    }

    pub fn eval(&mut self, pong: Ponga) -> RunRes<Ponga> {
        use Ponga::*;
        match pong {
            Sexpr(mut v) => {
                if v.len() == 0 {
                    return Ok(Ponga::Null);
                } else if v.len() == 1 {
                    return self.func_eval(&v.pop().unwrap(), vec![]);
                }
                let mut iter = v.into_iter();
                let func = iter.next().unwrap();
                let args = iter.collect();
                self.func_eval(&func, args)
            }
            Array(arr) => {
                let mut res = Vec::new();
                for pong in arr {
                    res.push(self.eval(pong)?);
                }
                Ok(Ponga::Ref(self.gc.add_obj(Ponga::Array(res))))
            }
            List(l) => {
                let mut res = LinkedList::new();
                for pong in l {
                    res.push_back(self.eval(pong)?);
                }
                Ok(Ponga::Ref(self.gc.add_obj(Ponga::List(res))))
            }
            Ref(id) => {
                let obj = self
                    .gc
                    .take_id(id)
                    .ok_or(RuntimeErr::ReferenceError(format!(
                        "Reference {} not found",
                        id
                    )))?;

                let res = self.eval(obj)?;

                if res.is_copy() {
                    self.gc.add_obj_with_id(res.clone(), id);
                    Ok(res)
                } else {
                    self.gc.add_obj_with_id(res, id);
                    self.eval(Ponga::Ref(id))
                }
            }
            Identifier(s) => {
                let (obj, wh) = self.pop_identifier_obj(&s)?;
                let obj = self.eval(obj)?;
                self.push_whereval(&s, obj.clone(), wh);
                Ok(obj)
            }
            _ => Ok(pong),
        }
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
            Ponga::HFunc(id) => format!("Internal function with id {}", id),
            Ponga::CFunc(args, _, state) => format!("Compound function with args {:?}, state {:?}", args, state),
            Ponga::Sexpr(a) => format!("S-expression: `{:?}`", a),
            Ponga::Identifier(s) => {
                let obj = self.get_identifier_obj_ref(s).unwrap();
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
                        .map(|(k, v)| format!("({}, {})", k.to_string(), self.ponga_to_string(v)))
                        .format(", ")
                )
            }
            Ponga::Array(arr) => {
                format!("#({})", arr.iter().map(|p| p.to_string()).format(", "))
            }
            Ponga::List(l) => {
                format!("'({})", l.iter().map(|p| p.to_string()).format(", "))
            }
        }
    }
}

pub fn run_str(s: &str) -> RunRes<Vec<RunRes<Ponga>>> {
    let mut runtime = Runtime::new();
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
        .map(|x| runtime.eval(x))
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
