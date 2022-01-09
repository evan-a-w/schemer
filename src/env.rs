use crate::types::*;
use std::collections::HashMap;
use itertools::Itertools;
use gc_rs::{Gc, Trace};

pub type PongMap = HashMap<String, Ponga>;

#[derive(Debug, Trace, Clone, PartialEq)]
pub struct MapUse {
    pub map: Gc<PongMap>,
    pub used: bool,
}

#[derive(Debug, Trace, Clone, PartialEq)]
pub struct Env {
    maps: Vec<Gc<PongMap>>,
}

impl Env {
    pub fn new() -> Self {
        Env {
            maps: vec![Gc::new(PongMap::new())],
        }
    }

    pub fn push(&mut self, map: Gc<PongMap>) {
        self.maps.push(map);
    }

    pub fn copy(&self) -> PongMap {
        self.maps.iter().fold(PongMap::new(), |mut acc, map| {
            for (k, v) in map.iter() {
                acc.insert(k.clone(), v.clone());
            }
            acc
        })
    }

    pub fn pop(&mut self) -> Gc<PongMap> {
        self.maps.pop().unwrap()
    }

    pub fn get(&self, identifier: &str) -> Option<&Ponga> {
        for m in self.maps.iter().rev() {
            if let Some(ponga) = m.get(identifier) {
                return Some(ponga);
            }
        }
        None
    }

    pub fn insert_furthest(&self, identifier: String, pong: Ponga) {
        self.maps[0].borrow_mut().unwrap().insert(identifier, pong);
    }

    pub fn set(&self, identifier: &str, pong: Ponga) -> Option<()> {
        for m in self.maps.iter().rev() {
            if let Some(ponga) = m.borrow_mut().unwrap().get_mut(identifier) {
                *ponga = pong;
                return Some(());
            }
        }
        None
    }
}

impl std::fmt::Display for Env {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.copy().iter()
                                   .map(|(k, v)| format!("{}: {}", k, v))
                                   .format(" "))
    }
}
