use crate::types::*;
use crate::gc::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Env {
    pub map: HashMap<String, Ponga>,
    pub outer: Option<Id>,
}

impl Env {
    pub fn new(outer: Option<Id>) -> Self {
        Env {
            map: HashMap::new(),
            outer,
        }
    }

    pub fn from_map(map: HashMap<String, Ponga>) -> Self {
        Env {
            map,
            outer: None,
        }
    }

    pub fn copy(&self, gc: &Gc<Env>) -> Self {
        self.copy_rec(gc, HashMap::new())
    }

    fn merge(mut a: HashMap<String, Ponga>, b: HashMap<String, Ponga>) -> HashMap<String, Ponga> {
        for (k, v) in b {
            a.entry(k).or_insert(v);
        }
        a
    }

    fn copy_rec(&self, gc: &Gc<Env>, curr: HashMap<String, Ponga>) -> Self {
        let merged = Self::merge(curr, self.map.clone());
        match self.outer {
            Some(id) => {
                let outer_obj = gc.get(id).unwrap();
                let outer = outer_obj.borrow().unwrap();
                let outer_map = outer.map.clone();
                let curr = Self::merge(merged, outer_map);
                outer.copy_rec(gc, curr)
            }
            None => Env::from_map(merged),
        }
    }

    pub fn integrate_hashmap(self, gc: &mut Gc<Env>, map: HashMap<String, Ponga>) -> Self {
        Env {
            map,
            outer: Some(gc.add(self)),
        }
    }

    pub fn insert_furthest<'a>(&'a mut self, gc: &'a Gc<Env>, s: String,
                               val: Ponga) {
        match self.outer {
            Some(id) => {
                let outer_obj = gc.get(id).unwrap();
                let mut outer = outer_obj.borrow_mut().unwrap();
                outer.insert_furthest(gc, s, val);
            }
            None => {
                self.map.insert(s, val);
            }
        }
    }

    pub fn get<'a>(&'a self, gc: &'a Gc<Env>,
                       identifier: &str) -> RunRes<&'a Ponga> {
        match self.map.get(identifier) {
            Some(ponga) => Ok(ponga),
            None => {
                if let Some(outer) = self.outer {
                    gc.get(outer)?.get(gc, identifier)
                } else {
                    Err(RuntimeErr::ReferenceError(identifier.to_string()))
                }
            }
        }
    }

    pub fn set<'a>(&'a mut self, gc: &'a Gc<Env>,
                   identifier: &str, val: Ponga) -> RunRes<()> {
        match self.map.get_mut(identifier) {
            Some(ponga) => {
                *ponga = val;
                Ok(())
            }
            None => {
                if let Some(outer) = self.outer {
                    let outer_env = gc.get(outer)?;
                    outer_env.borrow_mut().unwrap().set(gc, identifier, val)
                } else {
                    Err(RuntimeErr::ReferenceError(identifier.to_string()))
                }
            }
        }
    }
}

impl Trace<Ponga> for Env {
    fn trace(&self, gc: &Gc<Ponga>) {
        for (_, v) in self.map.iter() {
            v.trace(gc);
        }
        match self.outer {
            Some(id) => {
                let outer = gc.get(id).unwrap();
                let outer_borrowed = outer.borrow().unwrap();
                outer_borrowed.trace(gc);
            }
            None => {}
        }
    }
}

impl Trace<Env> for Env {
    fn trace(&self, gc: &Gc<Env>) {
        match self.outer {
            Some(id) => {
                let outer = gc.get(id).unwrap();
                outer.trace(gc);
            }
            None => {}
        }
    }
}
