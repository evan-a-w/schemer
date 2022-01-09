use crate::types::*;
use std::collections::HashMap;
use itertools::Itertools;
use gc_rs::{Gc, Trace};

#[derive(Debug, Trace, Clone, PartialEq)]
pub struct Env {
    pub map: HashMap<String, Ponga>,
    pub outer: Option<Gc<Env>>,
    pub in_use: bool,
}

impl Env {
    pub fn new(outer: Option<Gc<Env>>) -> Self {
        Env {
            map: HashMap::new(),
            outer,
            in_use: true,
        }
    }

    pub fn pop(self) -> Gc<Env> {
        let res = self.outer.unwrap();
        res.root();
        res
    }

    pub fn from_map(map: HashMap<String, Ponga>) -> Self {
        Env {
            map,
            outer: None,
            in_use: true,
        }
    }

    pub fn copy(&self) -> HashMap<String, Ponga> {
        self.copy_rec(HashMap::new())
    }

    fn merge(mut a: HashMap<String, Ponga>, b: HashMap<String, Ponga>) -> HashMap<String, Ponga> {
        for (k, v) in b {
            a.entry(k).or_insert(v);
        }
        a
    }

    fn copy_rec(&self, curr: HashMap<String, Ponga>) -> HashMap<String, Ponga> {
        match self.outer {
            Some(ref outer_obj) => {
                let merged = Self::merge(curr, self.map.clone());
                let outer_map = outer_obj.map.clone();
                let curr = Self::merge(merged, outer_map);
                outer_obj.copy_rec(curr)
            }
            None => curr,
        }
    }

    pub fn integrate_hashmap(me: Gc<Env>, map: HashMap<String, Ponga>) -> Self {
        Env {
            map,
            outer: Some(me),
            in_use: true,
        }
    }

    pub fn add_env_furthest_(&mut self, env: Gc<Env>, level: usize) -> usize {
        match &mut self.outer {
            Some(outer) => {
                let mut mut_ref = outer.borrow_mut().unwrap();
                mut_ref.add_env_furthest_(env, level + 1)
            }
            None => {
                self.outer = Some(env);
                level
            }
        }
    }

    pub fn add_env_furthest(&mut self, env: Gc<Env>) -> usize {
        self.add_env_furthest_(env, 0)
    }

    pub fn remove_env_at_level_(&mut self, level: usize, curr: usize) -> Option<Gc<Env>> {
        if curr == level {
            self.outer.take()
        } else {
            match &mut self.outer {
                Some(outer) => {
                    let mut mut_ref = outer.borrow_mut().unwrap();
                    mut_ref.remove_env_at_level_(level, curr + 1)
                }
                None => None,
            }
        }
    }

    pub fn remove_env_at_level(&mut self, level: usize) -> Option<Gc<Env>> {
        self.remove_env_at_level_(level, 0)
    }

    pub fn insert_furthest(&mut self, s: String, val: Ponga) {
        match &mut self.outer {
            Some(outer_obj) => {
                let mut mut_ref = outer_obj.borrow_mut().unwrap();
                mut_ref.insert_furthest(s, val);
            }
            None => {
                self.map.insert(s, val);
            }
        }
    }

    pub fn find_in(obj: Gc<Env>, identifier: &str) -> Option<Gc<Env>> {
        match obj.get(identifier) {
            Some(_) => Some(obj),
            None => match &obj.outer {
                Some(outer) => Self::find_in(outer.clone(), identifier),
                None => None,
            },
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

    pub fn set(obj: Gc<Env>, identifier: &str, pong: Ponga) -> Option<()> {
        match Self::find_in(obj, identifier) {
            Some(env) => {
                let mut mut_ref = env.borrow_mut().unwrap();
                mut_ref.map.insert(identifier.to_string(), pong);
                Some(())
            }
            None => None,
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
