use crate::types::*;

#[derive(Clone, Debug)]
pub enum Instruction {
    Eval(Ponga),
    Call(usize),
    Pop(String, Option<usize>),
    Push(String),
    Define(String),
    Set(String),
    CollectArray(usize),
    CollectObject(Vec<String>),
    CollectList(usize),
    CollectSexpr(usize),
    PopStack,
    PushStack(Ponga),
}
