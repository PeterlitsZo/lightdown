use std::collections::{BTreeMap, BTreeSet};

use crate::Span;
use crate::ast::{Expr, ExprKind, Module};
use crate::builtins::resolve_builtin;
use crate::bytecode::{Constant, ConstantId, Function, FunctionId, Instruction, Opcode, Program};

pub fn compile_module(module: &Module) -> Result<Program, CompileError> {
    Compiler::new(module).compile()
}

pub struct Compiler<'a> {
    module: &'a Module,
    program: Program,
    constants: BTreeMap<Constant, ConstantId>,
}

impl<'a> Compiler<'a> {
    pub fn new(module: &'a Module) -> Self {
        Self {
            module,
            program: Program {
                version: module.metadata.version.clone(),
                metadata_span: module.metadata.span,
                constants: Vec::new(),
                functions: vec![Function {
                    name: "module".into(),
                    arity: 0,
                    locals: 0,
                    captures: 0,
                    instructions: Vec::new(),
                }],
                entry: FunctionId(0),
            },
            constants: BTreeMap::new(),
        }
    }

    pub fn compile(mut self) -> Result<Program, CompileError> {
        let body = self.module.body.clone();
        let mut entry = FunctionCompiler::new("module".into(), Vec::new(), Vec::new());
        entry.compile_expr(&mut self, &body)?;
        entry.emit(Opcode::Return, self.module.span);
        self.program.functions[0] = entry.finish()?;
        Ok(self.program)
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

    fn reserve_function_slot(&mut self) -> Result<FunctionId, CompileError> {
        let next = u16::try_from(self.program.functions.len()).map_err(|_| {
            CompileError::IndexOverflow {
                kind: "functions",
                value: self.program.functions.len(),
            }
        })?;
        let id = FunctionId(next);
        self.program.functions.push(Function {
            name: format!("lambda#{next}"),
            arity: 0,
            locals: 0,
            captures: 0,
            instructions: Vec::new(),
        });
        Ok(id)
    }
}

struct FunctionCompiler {
    name: String,
    locals: Vec<String>,
    captures: Vec<String>,
    instructions: Vec<Instruction>,
}

impl FunctionCompiler {
    fn new(name: String, locals: Vec<String>, captures: Vec<String>) -> Self {
        Self {
            name,
            locals,
            captures,
            instructions: Vec::new(),
        }
    }

    fn compile_expr(
        &mut self,
        compiler: &mut Compiler<'_>,
        expr: &Expr,
    ) -> Result<(), CompileError> {
        match &expr.kind {
            ExprKind::String(text) => {
                let id = compiler.intern_constant(Constant::String(text.clone()))?;
                self.emit(Opcode::PushConst { id }, expr.span);
            }
            ExprKind::Bool(value) => {
                let id = compiler.intern_constant(Constant::Bool(*value))?;
                self.emit(Opcode::PushConst { id }, expr.span);
            }
            ExprKind::Symbol(name) => match self.resolve_name(name) {
                NameResolution::Local(slot) => self.emit(Opcode::LoadLocal { slot }, expr.span),
                NameResolution::Capture(slot) => self.emit(Opcode::LoadCapture { slot }, expr.span),
                NameResolution::Builtin => {
                    let id = compiler.intern_constant(Constant::String(name.clone()))?;
                    self.emit(Opcode::LoadBuiltin { name: id }, expr.span);
                }
            },
            ExprKind::Call { callee, args } => {
                self.compile_expr(compiler, callee)?;
                for arg in args {
                    self.compile_expr(compiler, arg)?;
                }
                let argc = u16::try_from(args.len()).map_err(|_| CompileError::IndexOverflow {
                    kind: "call arguments",
                    value: args.len(),
                })?;
                self.emit(Opcode::Call { argc }, expr.span);
            }
            ExprKind::Lambda { params, body } => {
                let free_names =
                    collect_free_names_from_body(body, params.iter().cloned().collect());
                let captures = free_names
                    .into_iter()
                    .filter(|name| {
                        matches!(
                            self.resolve_name(name),
                            NameResolution::Local(_) | NameResolution::Capture(_)
                        )
                    })
                    .collect::<Vec<_>>();

                let function_id = compiler.reserve_function_slot()?;
                let function_name = format!("lambda#{}", function_id.0);
                let mut lambda =
                    FunctionCompiler::new(function_name, params.clone(), captures.clone());
                lambda.compile_body(compiler, body)?;
                compiler.program.functions[usize::from(function_id.0)] = lambda.finish()?;

                for capture in &captures {
                    match self.resolve_name(capture) {
                        NameResolution::Local(slot) => {
                            self.emit(Opcode::LoadLocal { slot }, expr.span)
                        }
                        NameResolution::Capture(slot) => {
                            self.emit(Opcode::LoadCapture { slot }, expr.span)
                        }
                        NameResolution::Builtin => {
                            return Err(CompileError::UnsupportedConstruct {
                                detail: "builtin cannot be captured",
                            });
                        }
                    }
                }

                let capture_count =
                    u16::try_from(captures.len()).map_err(|_| CompileError::IndexOverflow {
                        kind: "closure captures",
                        value: captures.len(),
                    })?;
                self.emit(
                    Opcode::MakeClosure {
                        function: function_id,
                        captures: capture_count,
                    },
                    expr.span,
                );
            }
        }
        Ok(())
    }

    fn compile_body(
        &mut self,
        compiler: &mut Compiler<'_>,
        body: &[Expr],
    ) -> Result<(), CompileError> {
        for (index, expr) in body.iter().enumerate() {
            self.compile_expr(compiler, expr)?;
            if index + 1 != body.len() {
                self.emit(Opcode::Pop, expr.span);
            }
        }
        let span = body.last().map_or(compiler.module.span, |expr| expr.span);
        self.emit(Opcode::Return, span);
        Ok(())
    }

    fn resolve_name(&self, name: &str) -> NameResolution {
        if let Some(slot) = self.locals.iter().position(|candidate| candidate == name) {
            return NameResolution::Local(slot as u16);
        }
        if let Some(slot) = self.captures.iter().position(|candidate| candidate == name) {
            return NameResolution::Capture(slot as u16);
        }
        let _ = resolve_builtin(name);
        NameResolution::Builtin
    }

    fn emit(&mut self, opcode: Opcode, span: Span) {
        self.instructions.push(Instruction { opcode, span });
    }

    fn finish(self) -> Result<Function, CompileError> {
        let arity = u16::try_from(self.locals.len()).map_err(|_| CompileError::IndexOverflow {
            kind: "function arity",
            value: self.locals.len(),
        })?;
        let captures =
            u16::try_from(self.captures.len()).map_err(|_| CompileError::IndexOverflow {
                kind: "function captures",
                value: self.captures.len(),
            })?;
        Ok(Function {
            name: self.name,
            arity,
            locals: arity,
            captures,
            instructions: self.instructions,
        })
    }
}

enum NameResolution {
    Local(u16),
    Capture(u16),
    Builtin,
}

fn collect_free_names_from_body(body: &[Expr], bound: BTreeSet<String>) -> BTreeSet<String> {
    let mut free = BTreeSet::new();
    for expr in body {
        collect_free_names(expr, &bound, &mut free);
    }
    free
}

fn collect_free_names(expr: &Expr, bound: &BTreeSet<String>, free: &mut BTreeSet<String>) {
    match &expr.kind {
        ExprKind::String(_) | ExprKind::Bool(_) => {}
        ExprKind::Symbol(name) => {
            if !bound.contains(name) {
                free.insert(name.clone());
            }
        }
        ExprKind::Call { callee, args } => {
            collect_free_names(callee, bound, free);
            for arg in args {
                collect_free_names(arg, bound, free);
            }
        }
        ExprKind::Lambda { params, body } => {
            let mut child_bound = bound.clone();
            child_bound.extend(params.iter().cloned());
            for expr in body {
                collect_free_names(expr, &child_bound, free);
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompileError {
    IndexOverflow { kind: &'static str, value: usize },
    UnsupportedConstruct { detail: &'static str },
}
