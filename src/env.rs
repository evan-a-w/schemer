use crate::types::*;
use gc_rs::gc::*;
use gc_rs::gc_ref::*;
use std::collections::HashMap;
use itertools::Itertools;

#[derive(Debug)]
pub struct Env {
    pub map: HashMap<String, Ponga>,
    pub outer: Option<GcRefMut<Env>>,
}

impl Env {
    pub fn new(outer: Option<GcRefMut<Env>>) -> Self {
        Env {
            map: HashMap::new(),
            outer,
        }
    }

    pub fn pop(self) -> GcRefMut<Env> {
        self.outer.unwrap()
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
            Some(ref outer_obj) => {
                let outer_map = outer_obj.map.clone();
                let curr = Self::merge(merged, outer_map);
                outer_obj.copy_rec(gc, curr)
            }
            None => Env::from_map(merged),
        }
    }

    pub fn integrate_hashmap(self, gc: &mut Gc<Env>, map: HashMap<String, Ponga>) -> Self {
        Env {
            map,
            outer: {
                let id = gc.add(self);
                gc.get_mut(id)
            },
        }
    }

    pub fn to_ref_mut(self, gc: &mut Gc<Env>) -> GcRefMut<Env> {
        let id = gc.add(self);
        gc.get_mut(id).unwrap()
    }

    pub fn add_env_furthest_(&mut self, env: GcRefMut<Env>, level: usize) -> usize {
        match &mut self.outer {
            Some(outer) => outer.add_env_furthest_(env, level + 1),
            None => {
                self.outer = Some(env);
                level
            }
        }
    }

    pub fn add_env_furthest(&mut self, env: GcRefMut<Env>) -> usize {
        self.add_env_furthest_(env, 0)
    }

    pub fn remove_env_at_level_(&mut self, level: usize, curr: usize) -> Option<GcRefMut<Env>> {
        if curr == level {
            Some(std::mem::replace(&mut self.outer, None)?)
        } else {
            match &mut self.outer {
                Some(outer) => outer.remove_env_at_level_(level, curr + 1),
                None => None,
            }
        }
    }

    pub fn remove_env_at_level(&mut self, level: usize) -> Option<GcRefMut<Env>> {
        self.remove_env_at_level_(level, 0)
    }

    pub fn insert_furthest(&mut self, s: String, val: Ponga) {
        match &mut self.outer {
            Some(outer_obj) => {
                outer_obj.insert_furthest(s, val);
            }
            None => {
                self.map.insert(s, val);
            }
        }
    }
    
    pub fn get(&self, identifier: &str) -> Option<&Ponga> {
        match self.map.get(identifier) {
            Some(val) => Some(val),
            None => match &self.outer {
                Some(outer_obj) => outer_obj.get(identifier),
                None => None,
            },
        }
    }

    pub fn get_mut(&mut self, identifier: &str) -> Option<&mut Ponga> {
        match self.map.get_mut(identifier) {
            Some(val) => Some(val),
            None => match &mut self.outer {
                Some(outer_obj) => outer_obj.get_mut(identifier),
                None => None,
            },
        }
    }
}

impl Trace<Ponga> for Env {
    fn trace(&self, gc: &Gc<Ponga>) {
        for (_, v) in self.map.iter() {
            v.trace(gc);
        }
        match &self.outer {
            Some(outer_obj) => {
                outer_obj.as_ref().trace(gc);
            }
            None => {}
        }
    }
}

impl Trace<Env> for Env {
    fn trace(&self, gc: &Gc<Env>) {
        for (_, v) in self.map.iter() {
            v.trace(gc);
        }
        match &self.outer {
            Some(outer_obj) => {
                outer_obj.trace(gc);
            }
            None => {}
        }
    }
}

impl std::fmt::Display for Env {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.clone()
                            .map
                            .iter()
                            .map(|(k, v)| format!("{}: {}, ", k, v))
                            .format(" "))
    }
}
