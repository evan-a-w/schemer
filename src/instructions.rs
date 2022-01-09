use crate::types::*;

#[derive(Clone, Debug)]
pub enum Instruction {
    PopEnv(Option<usize>),
    PushEnv(Vec<String>),
    PopStack,
    Define(String),
    Set(String),
    CollectArray(usize),
    CollectObject(Vec<String>),
    CollectList(usize),
    Call(usize),
    Eval(Ponga),
}

impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Instruction::Eval(ponga) => write!(f, "eval {}", ponga),
            Instruction::Call(n) => write!(f, "call {}", n),
            Instruction::PopEnv(_) => write!(f, "pop_env"),
            Instruction::PushEnv(_) => write!(f, "push_env"),
            Instruction::Define(name) => write!(f, "define {}", name),
            Instruction::Set(name) => write!(f, "set {}", name),
            Instruction::CollectArray(n) => write!(f, "collect array {}", n),
            Instruction::CollectObject(names) => write!(f, "collect object {}", names.join(", ")),
            Instruction::CollectList(n) => write!(f, "collect list {}", n),
            Instruction::PopStack => write!(f, "pop stack"),
        }
    }
}
