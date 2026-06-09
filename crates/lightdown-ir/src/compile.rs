use std::collections::BTreeMap;

use crate::Span;
use crate::ast::{Expr, ExprKind, Module};
use crate::bytecode::{Constant, ConstantId, Function, FunctionId, Instruction, Opcode, Program};

pub fn compile_module(module: &Module) -> Result<Program, CompileError> {
    Compiler::new(module).compile()
}

pub struct Compiler<'a> {
    module: &'a Module,
    program: Program,
    constants: BTreeMap<Constant, ConstantId>,
    entry_instructions: Vec<Instruction>,
}

impl<'a> Compiler<'a> {
    pub fn new(module: &'a Module) -> Self {
        Self {
            module,
            program: Program {
                version: module.metadata.version.clone(),
                metadata_span: module.metadata.span,
                constants: Vec::new(),
                functions: Vec::new(),
                entry: FunctionId(0),
            },
            constants: BTreeMap::new(),
            entry_instructions: Vec::new(),
        }
    }

    pub fn compile(mut self) -> Result<Program, CompileError> {
        self.compile_expr(&self.module.body)?;
        self.emit(Opcode::Return, self.module.span);
        self.program.entry = FunctionId(0);
        self.program.functions.push(Function {
            name: "module".into(),
            arity: 0,
            locals: 0,
            instructions: self.entry_instructions,
        });
        Ok(self.program)
    }

    fn compile_expr(&mut self, expr: &Expr) -> Result<(), CompileError> {
        match &expr.kind {
            ExprKind::String(text) => {
                let id = self.intern_constant(Constant::String(text.clone()))?;
                self.emit(Opcode::PushConst { id }, expr.span);
            }
            ExprKind::Bool(value) => {
                let id = self.intern_constant(Constant::Bool(*value))?;
                self.emit(Opcode::PushConst { id }, expr.span);
            }
            ExprKind::Symbol(name) => {
                let id = self.intern_constant(Constant::String(name.clone()))?;
                self.emit(Opcode::LoadBuiltin { name: id }, expr.span);
            }
            ExprKind::Call { callee, args } => {
                self.compile_expr(callee)?;
                for arg in args {
                    self.compile_expr(arg)?;
                }
                let argc = u16::try_from(args.len()).map_err(|_| CompileError::IndexOverflow {
                    kind: "call arguments",
                    value: args.len(),
                })?;
                self.emit(Opcode::Call { argc }, expr.span);
            }
        }
        Ok(())
    }

    pub fn intern_constant(&mut self, constant: Constant) -> Result<ConstantId, CompileError> {
        if let Some(id) = self.constants.get(&constant).copied() {
            return Ok(id);
        }

        let next = u16::try_from(self.program.constants.len()).map_err(|_| {
            CompileError::IndexOverflow {
                kind: "constants",
                value: self.program.constants.len(),
            }
        })?;
        let id = ConstantId(next);
        self.program.constants.push(constant.clone());
        self.constants.insert(constant, id);
        Ok(id)
    }

    fn emit(&mut self, opcode: Opcode, span: Span) {
        self.entry_instructions.push(Instruction { opcode, span });
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompileError {
    IndexOverflow { kind: &'static str, value: usize },
    UnsupportedConstruct { detail: &'static str },
}
