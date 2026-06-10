# Let And Lambda Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Scheme-like `lambda` and `let` support to Lightdown IR, including multi-expression bodies, lexical scope, and user-defined function calls.

**Architecture:** Parse `lambda` into a dedicated AST node with parameter names and a non-empty body sequence. Parse `let` as syntax sugar that lowers immediately into a lambda call. Extend compilation to emit separate bytecode functions plus closure creation, and resolve symbols against locals, captured values, then builtins. Extend the VM with closures and lexical capture so author-facing embedded IR can abstract repeated table structure without new surface syntax beyond `let` and `lambda`.

**Tech Stack:** Rust, Lightdown IR parser/compiler/VM, existing Rust integration tests.

---

### Task 1: Parse New Forms

**Files:**
- Modify: `crates/lightdown-ir/src/ast.rs`
- Modify: `crates/lightdown-ir/src/parser.rs`
- Test: `crates/lightdown-ir/tests/parser.rs`

- [ ] Add AST nodes for `lambda` plus multi-expression bodies.
- [ ] Parse `(lambda (x y) expr1 expr2)` into the new AST shape.
- [ ] Parse `(let ((name value) ...) body...)` by lowering it into a lambda call during parsing.
- [ ] Extend parser coverage to assert the new AST shape and `let` lowering.

### Task 2: Compile Closures And Lexical Scope

**Files:**
- Modify: `crates/lightdown-ir/src/bytecode.rs`
- Modify: `crates/lightdown-ir/src/compile.rs`
- Test: `crates/lightdown-ir/tests/bytecode.rs`

- [ ] Add bytecode support for user-defined closures and captured values.
- [ ] Track lexical scopes during compilation so symbols resolve to locals, captures, or builtins.
- [ ] Compile each `lambda` into its own function object with a body that returns the last expression.
- [ ] Extend bytecode tests to cover closure creation and local symbol loading.

### Task 3: Execute User Functions

**Files:**
- Modify: `crates/lightdown-ir/src/runtime.rs`
- Modify: `crates/lightdown-ir/src/builtins.rs`
- Modify: `crates/lightdown-ir/src/vm.rs`
- Test: `crates/lightdown-ir/tests/eval.rs`

- [ ] Add a closure runtime value that stores a function id plus captured environment values.
- [ ] Update the VM call path to invoke either builtins or user closures.
- [ ] Ensure multi-expression lambda bodies return the last expression and still interoperate with existing document builtins.
- [ ] Extend evaluation tests with table-oriented `let` and `lambda` examples.

### Task 4: Verify End-To-End Author Syntax

**Files:**
- Modify: `crates/lightdown/tests/parse.rs`
- Modify: `crates/lightdown/tests/render.rs`

- [ ] Add author-pipeline coverage for embedded IR that uses `let` and `lambda`.
- [ ] Run targeted parser, bytecode, eval, parse, and render tests to verify the full pipeline.
