use crate::types::*;
use std::collections::HashMap;
use itertools::Itertools;
use gc_rs::{Gc, Trace};

#[derive(Debug, Trace, Clone, PartialEq)]
pub struct Env {
    pub map: HashMap<String, Ponga>,
    pub outer: Option<Gc<Env>>,
}

impl Env {
    pub fn new(outer: Option<Gc<Env>>) -> Self {
        Env {
            map: HashMap::new(),
            outer,
        }
    }

    pub fn pop(self) -> Gc<Env> {
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

    pub fn integrate_hashmap(me: Gc<Env>, map: HashMap<String, Ponga>) -> Self {
        Env {
            map,
            outer: Some(me),
        }
    }

    pub fn add_env_furthest_(&mut self, env: Gc<Env>, level: usize) -> usize {
        match &mut self.outer {
            Some(outer) => outer.add_env_furthest_(env, level + 1),
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
                Some(outer) => outer.remove_env_at_level_(level, curr + 1),
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

    pub fn set(&mut self, identifer: &str, pong: Ponga) -> Option<()> {
        match self.map.get_mut(identifer) {
            Some(val) => {
                *val = pong;
                Some(())
            }
            None => match self.outer {
                Some(outer_obj) => outer_obj.borrow_mut().unwrap().set(identifer, pong),
                None => None,
            },
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
