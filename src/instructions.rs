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
    PopStack,
}

impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Instruction::Eval(ponga) => write!(f, "eval {}", ponga),
            Instruction::Call(n) => write!(f, "call {}", n),
            Instruction::Pop(name, n) => write!(f, "pop {} {}", name, n.unwrap_or(0)),
            Instruction::Push(name) => write!(f, "push {}", name),
            Instruction::Define(name) => write!(f, "define {}", name),
            Instruction::Set(name) => write!(f, "set {}", name),
            Instruction::CollectArray(n) => write!(f, "collect array {}", n),
            Instruction::CollectObject(names) => write!(f, "collect object {}", names.join(", ")),
            Instruction::CollectList(n) => write!(f, "collect list {}", n),
            Instruction::PopStack => write!(f, "pop stack"),
        }
    }
}
