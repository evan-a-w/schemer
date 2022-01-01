use crate::types::*;

#[derive(Clone, Debug)]
pub enum Instruction {
    Eval,
    Call(usize),
    Pop(String),
    Push(String),
    Define(String),
    Set(String),
    CollectArray(usize),
    CollectObject(Vec<String>),
    CollectList(usize),
    CollectSexpr(usize),
}