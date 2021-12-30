use crate::gc::*;
use crate::gc_obj::*;
use crate::parser::*;
use crate::stdlib::*;
use crate::types::*;
use itertools::Itertools;
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::ptr::{self, NonNull};

pub const DEBUG_PRINT: bool = false;

pub type Namespace = HashMap<String, Id>;

pub type PriorityNamespace = HashMap<String, Vec<Id>>;

pub struct Runtime {
    pub globals: Namespace,
    pub global_funcs: Namespace,
    pub locals: PriorityNamespace,
    pub gc: Gc,
}

impl Runtime {
    pub fn new() -> Self {
        let mut global_funcs = Namespace::new();
        let mut gc = Gc::new();

        for val in FUNCS.iter() {
            global_funcs.insert(val.0.to_string(), gc.get_new_id());
        }

        Self {
            globals: Namespace::new(),
            global_funcs,
            locals: PriorityNamespace::new(),
            gc,
        }
    }

    pub fn bind_global(&mut self, s: String, id: Id) {
        self.globals.insert(s, id);
    }

    pub fn unbind_global(&mut self, s: &str) {
        self.globals.remove(s);
    }

    pub fn add_roots_to_gc(&mut self) {
        self.gc.roots = self.globals.values().cloned().collect();
        for vec in self.locals.values() {
            for i in vec.iter() {
                self.gc.roots.insert(*i);
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

    pub fn get_identifier_id(&mut self, identifier: &str) -> RunRes<Id> {
        let x = self.locals.get(identifier).map(|vec| vec[vec.len() - 1]);
        match x {
            Some(id) => return Ok(id),
            None => (),
        }
        match self.globals.get(identifier) {
            Some(id) => return Ok(*id),
            None => (),
        }
        let id = self
            .global_funcs
            .get(identifier)
            .ok_or(RuntimeErr::ReferenceError(format!(
                "Identifier {} not found",
                identifier
            )))?;
        Ok(*id)
    }

    pub fn get_identifier_gc_obj_ref(&self, identifier: &str) -> RunRes<GcObj> {
        let x = self.locals.get(identifier).map(|vec| vec[vec.len() - 1]);
        match x {
            Some(id) => {
                let res = self.get_id_obj_ref(id)?;
                return Ok(res.clone());
            }
            None => (),
        }
        let x = self.globals.get(identifier).cloned();
        match x {
            Some(id) => {
                let res = self.get_id_obj_ref(id)?;
                return Ok(res.clone());
            }
            None => (),
        }
        let id = self
            .global_funcs
            .get(identifier)
            .ok_or(RuntimeErr::ReferenceError(format!(
                "Identifier {} not found",
                identifier
            )))?;
        Ok(GcObj {
            data: NonNull::new(Box::into_raw(Box::new(Ponga::HFunc(*id)))).unwrap(),
            flags: UnsafeCell::new(Flags {
                marker: MarkerFlag::Unseen,
                taken: TakenFlag::NotTaken,
                to_free: true,
            }),
            id: *id,
        })
    }

    pub fn get_identifier_gc_obj(&mut self, identifier: &str) -> RunRes<GcObj> {
        let x = self.locals.get(identifier).map(|vec| vec[vec.len() - 1]);
        match x {
            Some(id) => {
                let res = self.get_id_obj(id)?;
                return Ok(res.clone());
            }
            None => (),
        }
        let x = self.globals.get(identifier).cloned();
        match x {
            Some(id) => {
                let res = self.get_id_obj(id)?;
                return Ok(res.clone());
            }
            None => (),
        }
        let id = self
            .global_funcs
            .get(identifier)
            .ok_or(RuntimeErr::ReferenceError(format!(
                "Identifier {} not found",
                identifier
            )))?;
        Ok(GcObj {
            data: NonNull::new(Box::into_raw(Box::new(Ponga::HFunc(*id)))).unwrap(),
            flags: UnsafeCell::new(Flags {
                marker: MarkerFlag::Unseen,
                taken: TakenFlag::NotTaken,
                to_free: true,
            }),
            id: *id,
        })
    }

    pub fn func_eval(&mut self, pong: &Ponga, args: Vec<Ponga>) -> RunRes<Ponga> {
        //println!("Evaluating func {:?} with args {:?}", pong, args);
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
            Ponga::CFunc(names, id) => {
                args_assert_len(&args, names.len(), "func")?;
                let args = eval_args(self, args)?;
                for (name, arg) in names.iter().zip(args.into_iter()) {
                    if DEBUG_PRINT {
                        println!("Binding {} to {:?}", name, arg);
                    }
                    let n_id = self.gc.add_obj(arg);
                    self.locals
                        .entry(name.to_string())
                        .or_insert(Vec::new())
                        .push(n_id);
                }

                let sexpr = self.get_id_obj(*id)?.borrow().unwrap().inner().clone();

                let res = self.eval(sexpr)?;

                for name in names.iter() {
                    let vec = self.locals.get_mut(name).unwrap();
                    vec.pop();
                    if vec.len() == 0 {
                        self.locals.remove(name);
                    }
                }

                Ok(res)
            }
            Ponga::Identifier(identifier) => {
                let mut obj = self.get_identifier_gc_obj(identifier)?;
                let res = self.func_eval(obj.borrow_mut().unwrap().inner(), args)?;
                Ok(res)
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
        //println!("Evaluating pong {:?}", pong);
        use Ponga::*;
        match pong {
            Sexpr(v) => {
                if v.len() == 0 {
                    return Ok(Ponga::Null);
                } else if v.len() == 1 {
                    return self.eval(v.into_iter().next().unwrap());
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
                Ok(Ponga::Array(res))
            }
            List(l) => {
                use std::collections::LinkedList;
                let mut res = LinkedList::new();
                for pong in l {
                    res.push_back(self.eval(pong)?);
                }
                Ok(Ponga::List(res))
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

                self.gc.add_obj_with_id(res, id);

                Ok(Ponga::Ref(id))
            }
            Identifier(s) => {
                //println!("Matched here");
                let obj = self.get_identifier_gc_obj(&s)?.borrow().unwrap().clone();
                //println!("{}: {:?}", s, obj);
                self.eval(obj)
            }
            _ => Ok(pong),
        }
    }

    pub fn is_func(&self, pong: &Ponga) -> bool {
        match pong {
            Ponga::HFunc(_) => true,
            Ponga::CFunc(_, _) => true,
            Ponga::Identifier(s) => {
                let f = self.get_identifier_gc_obj_ref(s).unwrap();
                let x = f.borrow().unwrap();
                x.inner().is_func()
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
                let f = self.get_identifier_gc_obj_ref(s).unwrap();
                let x = f.borrow().unwrap();
                x.inner().is_list()
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
                let f = self.get_identifier_gc_obj_ref(s).unwrap();
                let x = f.borrow().unwrap();
                x.inner().is_vector()
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
                let f = self.get_identifier_gc_obj_ref(s).unwrap();
                let x = f.borrow().unwrap();
                x.inner().is_char()
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
                let f = self.get_identifier_gc_obj_ref(s).unwrap();
                let x = f.borrow().unwrap();
                x.inner().is_number()
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
                let f = self.get_identifier_gc_obj_ref(s).unwrap();
                let x = f.borrow().unwrap();
                x.inner().is_string()
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
                let f = self.get_identifier_gc_obj_ref(s).unwrap();
                let x = f.borrow().unwrap();
                x.inner().is_symbol()
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
            Ponga::CFunc(args, _) => format!("Compound function with args {:?}", args),
            Ponga::Sexpr(_) => format!("S-expression"),
            Ponga::Identifier(s) => format!("Identifier {}", s),
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
    println!("{:?}", parsed);
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
