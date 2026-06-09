pub mod ast;
mod builtins;
pub mod bytecode;
pub mod compile;
pub mod document;
pub mod lexer;
pub mod parser;
pub mod runtime;
pub mod vm;

pub use ast::{Expr, ExprKind, Module, ModuleMetadata, Node};
pub use bytecode::{Constant, ConstantId, Function, FunctionId, Instruction, Opcode, Program};
pub use compile::{CompileError, Compiler, compile_module};
pub use document::{
    Block, BlockKind, Document, DocumentMetadata, Inline, InlineKind, TableCell, TableCellKind,
    TableChild, TableChildKind, TableRow, TableRowKind,
};
pub use lexer::{LexError, LexErrorKind, Lexer, Position, Span, Token, TokenKind};
pub use parser::{ParseError, ParseErrorKind, Parser, parse, parse_expr_fragment};
pub use runtime::{
    BlockValue, DocumentValue, InlineValue, MetadataValue, NodeValue, TableCellValue,
    TableChildValue, TableRowValue, Value,
};
pub use vm::{Vm, VmError, execute_document, execute_program};
