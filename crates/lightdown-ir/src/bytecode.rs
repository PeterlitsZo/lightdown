use crate::Span;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Program {
    pub version: String,
    pub metadata_span: Span,
    pub constants: Vec<Constant>,
    pub functions: Vec<Function>,
    pub entry: FunctionId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Function {
    pub name: String,
    pub arity: u16,
    pub locals: u16,
    pub captures: u16,
    pub instructions: Vec<Instruction>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Instruction {
    pub opcode: Opcode,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FunctionId(pub u16);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ConstantId(pub u16);

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Constant {
    String(String),
    Bool(bool),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Opcode {
    PushConst { id: ConstantId },
    LoadBuiltin { name: ConstantId },
    LoadLocal { slot: u16 },
    LoadCapture { slot: u16 },
    StoreLocal { slot: u16 },
    MakeClosure { function: FunctionId, captures: u16 },
    Call { argc: u16 },
    Return,
    Jump { target: usize },
    JumpIfFalse { target: usize },
    Pop,
}
