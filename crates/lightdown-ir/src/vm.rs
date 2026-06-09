use crate::Span;
use crate::builtins::{call_callable, resolve_builtin};
use crate::bytecode::{Constant, ConstantId, Function, FunctionId, Instruction, Opcode, Program};
use crate::document::Document;
use crate::runtime::{
    CallableValue, DecodeError, MetadataValue, NodeValue, Value,
};

pub fn execute_program(program: &Program) -> Result<Value, VmError> {
    Vm::new(program).execute()
}

pub fn execute_document(program: &Program) -> Result<Document, VmError> {
    let value = execute_program(program)?;
    match value {
        Value::Node(NodeValue::Document(document)) => {
            let span = document.span;
            document
                .try_into_document()
                .map_err(|error| VmError::InvalidRuntimeShape {
                    detail: decode_error_detail(error),
                    span,
                })
        }
        other => Err(VmError::NonDocumentEntryResult {
            found: other.kind_name(),
            span: value_span(&other),
        }),
    }
}

pub struct Vm<'a> {
    program: &'a Program,
    stack: Vec<Value>,
    frames: Vec<CallFrame>,
}

impl<'a> Vm<'a> {
    pub fn new(program: &'a Program) -> Self {
        Self {
            program,
            stack: Vec::new(),
            frames: vec![CallFrame::new(program.entry, 0)],
        }
    }

    pub fn execute(mut self) -> Result<Value, VmError> {
        loop {
            let instruction = self.next_instruction()?;
            match instruction.opcode {
                Opcode::PushConst { id } => {
                    self.stack
                        .push(self.constant_to_value(id, instruction.span)?);
                }
                Opcode::LoadBuiltin { name } => {
                    let name = self.constant_string(name, instruction.span)?;
                    let Some(builtin) = resolve_builtin(&name) else {
                        return Err(VmError::UnknownBuiltin {
                            name,
                            span: instruction.span,
                        });
                    };
                    self.stack.push(Value::Callable(CallableValue::Builtin(builtin)));
                }
                Opcode::LoadLocal { slot } => {
                    let value = self
                        .current_frame()
                        .locals
                        .get(usize::from(slot))
                        .cloned()
                        .ok_or(VmError::InvalidLocalSlot {
                            slot,
                            span: instruction.span,
                        })?;
                    self.stack.push(value);
                }
                Opcode::StoreLocal { slot } => {
                    let value = self.pop_value("StoreLocal", instruction.span)?;
                    let local = self
                        .current_frame_mut()
                        .locals
                        .get_mut(usize::from(slot))
                        .ok_or(VmError::InvalidLocalSlot {
                            slot,
                            span: instruction.span,
                        })?;
                    *local = value;
                }
                Opcode::Call { argc } => {
                    let args = self.pop_many(usize::from(argc), "Call", instruction.span)?;
                    let callee = self.pop_value("Call", instruction.span)?;
                    let Value::Callable(callable) = callee else {
                        return Err(VmError::NonCallableValue {
                            found: callee.kind_name(),
                            builtin: None,
                            span: value_span(&callee).or(Some(instruction.span)),
                        });
                    };
                    let result = call_callable(
                        callable,
                        args,
                        instruction.span,
                        &MetadataValue {
                            version: self.program.version.clone(),
                            span: self.program.metadata_span,
                        },
                    )?;
                    self.stack.push(result);
                }
                Opcode::Return => {
                    let value = self.pop_value("Return", instruction.span)?;
                    self.frames.pop();
                    if self.frames.is_empty() {
                        return Ok(value);
                    }
                    self.stack.push(value);
                }
                Opcode::Jump { target } => {
                    self.jump_to(target, instruction.span)?;
                }
                Opcode::JumpIfFalse { target } => {
                    let condition = self.pop_value("JumpIfFalse", instruction.span)?;
                    match condition {
                        Value::Bool(false) => self.jump_to(target, instruction.span)?,
                        Value::Bool(true) => {}
                        other => {
                            return Err(VmError::TypeMismatch {
                                opcode: "JumpIfFalse",
                                expected: "bool",
                                found: other.kind_name(),
                                span: instruction.span,
                            });
                        }
                    }
                }
                Opcode::Pop => {
                    self.pop_value("Pop", instruction.span)?;
                }
            }
        }
    }

    fn next_instruction(&mut self) -> Result<Instruction, VmError> {
        let frame = self.current_frame();
        let function_id = frame.function;
        let ip = frame.ip;
        let function = self.function(function_id, self.program.metadata_span)?;
        let Some(instruction) = function.instructions.get(ip).cloned() else {
            return Err(VmError::MissingReturnValue {
                function: function_id,
                span: function
                    .instructions
                    .last()
                    .map(|instruction| instruction.span)
                    .unwrap_or(self.program.metadata_span),
            });
        };
        self.current_frame_mut().ip += 1;
        Ok(instruction)
    }

    fn constant_to_value(&self, id: ConstantId, span: Span) -> Result<Value, VmError> {
        match self.program.constants.get(usize::from(id.0)) {
            Some(Constant::String(text)) => Ok(Value::String(text.clone())),
            Some(Constant::Bool(value)) => Ok(Value::Bool(*value)),
            None => Err(VmError::InvalidConstantIndex { index: id, span }),
        }
    }

    fn constant_string(&self, id: ConstantId, span: Span) -> Result<String, VmError> {
        match self.program.constants.get(usize::from(id.0)) {
            Some(Constant::String(text)) => Ok(text.clone()),
            Some(Constant::Bool(_)) => Err(VmError::TypeMismatch {
                opcode: "LoadBuiltin",
                expected: "string",
                found: "bool",
                span,
            }),
            None => Err(VmError::InvalidConstantIndex { index: id, span }),
        }
    }

    fn jump_to(&mut self, target: usize, span: Span) -> Result<(), VmError> {
        let function = self.function(self.current_frame().function, span)?;
        if target >= function.instructions.len() {
            return Err(VmError::InvalidJumpTarget { target, span });
        }
        self.current_frame_mut().ip = target;
        Ok(())
    }

    fn current_frame(&self) -> &CallFrame {
        self.frames
            .last()
            .expect("vm always keeps at least one frame until return")
    }

    fn current_frame_mut(&mut self) -> &mut CallFrame {
        self.frames
            .last_mut()
            .expect("vm always keeps at least one frame until return")
    }

    fn function(&self, id: FunctionId, span: Span) -> Result<&Function, VmError> {
        self.program
            .functions
            .get(usize::from(id.0))
            .ok_or(VmError::InvalidFunctionIndex { index: id, span })
    }

    fn pop_value(&mut self, opcode: &'static str, span: Span) -> Result<Value, VmError> {
        self.stack
            .pop()
            .ok_or(VmError::StackUnderflow { opcode, span })
    }

    fn pop_many(
        &mut self,
        len: usize,
        opcode: &'static str,
        span: Span,
    ) -> Result<Vec<Value>, VmError> {
        if self.stack.len() < len {
            return Err(VmError::StackUnderflow { opcode, span });
        }

        let mut values = Vec::with_capacity(len);
        for _ in 0..len {
            values.push(self.stack.pop().expect("length already checked"));
        }
        values.reverse();
        Ok(values)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CallFrame {
    pub function: FunctionId,
    pub ip: usize,
    pub locals: Vec<Value>,
}

impl CallFrame {
    fn new(function: FunctionId, locals: u16) -> Self {
        Self {
            function,
            ip: 0,
            locals: vec![Value::Unit; usize::from(locals)],
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VmError {
    StackUnderflow {
        opcode: &'static str,
        span: Span,
    },
    InvalidConstantIndex {
        index: crate::bytecode::ConstantId,
        span: Span,
    },
    InvalidFunctionIndex {
        index: FunctionId,
        span: Span,
    },
    InvalidLocalSlot {
        slot: u16,
        span: Span,
    },
    TypeMismatch {
        opcode: &'static str,
        expected: &'static str,
        found: &'static str,
        span: Span,
    },
    InvalidJumpTarget {
        target: usize,
        span: Span,
    },
    MissingReturnValue {
        function: FunctionId,
        span: Span,
    },
    NonDocumentEntryResult {
        found: &'static str,
        span: Option<Span>,
    },
    InvalidRuntimeShape {
        detail: &'static str,
        span: Span,
    },
    UnknownBuiltin {
        name: String,
        span: Span,
    },
    NonCallableValue {
        found: &'static str,
        builtin: Option<&'static str>,
        span: Option<Span>,
    },
    BuiltinArityMismatch {
        builtin: &'static str,
        expected: &'static str,
        actual: usize,
        span: Span,
    },
    BuiltinTypeMismatch {
        builtin: &'static str,
        expected: &'static str,
        found: &'static str,
        span: Span,
    },
}

fn decode_error_detail(error: DecodeError) -> &'static str {
    match error {
        DecodeError::ExpectedBlock => "expected block node",
        DecodeError::ExpectedInline => "expected inline node",
        DecodeError::ExpectedTableChild => "expected table child node",
        DecodeError::ExpectedTableRow => "expected table row node",
        DecodeError::ExpectedTableCell => "expected table cell node",
    }
}

fn value_span(value: &Value) -> Option<Span> {
    match value {
        Value::Node(node) => Some(match node {
            NodeValue::Document(document) => document.span,
            NodeValue::Block(block) => match block {
                crate::runtime::BlockValue::Heading { span, .. }
                | crate::runtime::BlockValue::Paragraph { span, .. }
                | crate::runtime::BlockValue::List { span, .. }
                | crate::runtime::BlockValue::ListItem { span, .. }
                | crate::runtime::BlockValue::BlockQuote { span, .. }
                | crate::runtime::BlockValue::CodeBlock { span, .. }
                | crate::runtime::BlockValue::ThematicBreak { span }
                | crate::runtime::BlockValue::Table { span, .. } => *span,
            },
            NodeValue::Inline(inline) => match inline {
                crate::runtime::InlineValue::Text { span, .. }
                | crate::runtime::InlineValue::Emphasis { span, .. }
                | crate::runtime::InlineValue::Strong { span, .. }
                | crate::runtime::InlineValue::Code { span, .. }
                | crate::runtime::InlineValue::Link { span, .. }
                | crate::runtime::InlineValue::Image { span, .. }
                | crate::runtime::InlineValue::Break { span } => *span,
            },
            NodeValue::TableChild(child) => match child {
                crate::runtime::TableChildValue::Head { span, .. }
                | crate::runtime::TableChildValue::Body { span, .. } => *span,
            },
            NodeValue::TableRow(row) => row.span,
            NodeValue::TableCell(cell) => match cell {
                crate::runtime::TableCellValue::Header { span, .. }
                | crate::runtime::TableCellValue::Data { span, .. } => *span,
            },
        }),
        _ => None,
    }
}
