use lightdown_ir::{
    Constant, FunctionId, Instruction, Opcode, Program, VmError, compile_module, execute_program,
    parse,
};

#[test]
fn compiles_minimal_module_into_a_single_entry_function() {
    let module = parse(r#"(doc {:meta {:version "0.1.0"}})"#).expect("module parses");

    let program = compile_module(&module).expect("module compiles");

    assert_eq!(program.version, "0.1.0");
    assert_eq!(program.metadata_span, module.metadata.span);
    assert_eq!(program.entry, FunctionId(0));
    assert_eq!(program.functions.len(), 1);
    assert_eq!(program.functions[0].arity, 0);
    assert_eq!(program.functions[0].locals, 0);
    assert_eq!(program.functions[0].captures, 0);
    assert_eq!(program.constants, vec![Constant::String("doc".into())]);
    assert_eq!(program.functions[0].instructions.len(), 3);
    assert_eq!(
        program.functions[0].instructions[0],
        Instruction {
            opcode: Opcode::LoadBuiltin {
                name: lightdown_ir::ConstantId(0),
            },
            span: call_callee_span(&module),
        }
    );
    assert_eq!(
        program.functions[0].instructions[1],
        Instruction {
            opcode: Opcode::Call { argc: 0 },
            span: module.body.span,
        }
    );
    assert_eq!(
        program.functions[0].instructions[2],
        Instruction {
            opcode: Opcode::Return,
            span: module.span,
        }
    );
}

#[test]
fn pools_repeated_string_and_builtin_name_constants() {
    let module = parse(indoc::indoc! {r#"
        (doc {:meta {:version "0.1.0"}}
          (p (text "same") (text "same"))
          (codeblock "same")
          (p (code "same")))
    "#})
    .expect("module parses");

    let program = compile_module(&module).expect("module compiles");

    let same_count = program
        .constants
        .iter()
        .filter(|constant| matches!(constant, Constant::String(text) if text == "same"))
        .count();
    let text_count = program
        .constants
        .iter()
        .filter(|constant| matches!(constant, Constant::String(text) if text == "text"))
        .count();

    assert_eq!(same_count, 1);
    assert_eq!(text_count, 1);
}

#[test]
fn compiles_list_map_and_apply_with_only_generic_call_opcodes() {
    let module = parse(indoc::indoc! {r#"
        (doc {:meta {:version "0.1.0"}}
          (table
            (thead
              (apply tr (map th (list (text "Foo") (text "Bar")))))))
    "#})
    .expect("module parses");

    let program = compile_module(&module).expect("module compiles");
    let opcodes = program.functions[0]
        .instructions
        .iter()
        .map(|instruction| &instruction.opcode)
        .collect::<Vec<_>>();

    assert!(opcodes.iter().all(|opcode| {
        matches!(
            opcode,
            Opcode::PushConst { .. }
                | Opcode::LoadBuiltin { .. }
                | Opcode::Call { .. }
                | Opcode::Return
        )
    }));
    assert!(
        opcodes
            .iter()
            .any(|opcode| matches!(opcode, Opcode::Call { argc: 2 }))
    );
}

#[test]
fn compiles_lambda_closures_with_local_and_capture_loads() {
    let module = parse(indoc::indoc! {r#"
        (doc {:meta {:version "0.1.0"}}
          ((lambda (header)
             ((lambda (wrap)
                (wrap header))
              (lambda (value)
                (th value))))
           (text "Foo")))
    "#})
    .expect("module parses");

    let program = compile_module(&module).expect("module compiles");

    assert_eq!(program.functions.len(), 4);
    assert!(
        program.functions[0]
            .instructions
            .iter()
            .any(|instruction| matches!(instruction.opcode, Opcode::MakeClosure { .. }))
    );
    assert!(
        program.functions[1..]
            .iter()
            .flat_map(|function| function.instructions.iter())
            .any(|instruction| matches!(instruction.opcode, Opcode::LoadCapture { .. }))
    );
    assert!(
        program.functions[1..]
            .iter()
            .flat_map(|function| function.instructions.iter())
            .any(|instruction| matches!(instruction.opcode, Opcode::LoadLocal { .. }))
    );
}

#[test]
fn reports_runtime_errors_for_unknown_builtin_and_non_callable_values() {
    let span = parse(r#"(doc {:meta {:version "0.1.0"}})"#)
        .expect("module parses")
        .span;

    let unknown_builtin = execute_program(&Program {
        version: "0.1.0".into(),
        metadata_span: span,
        constants: vec![Constant::String("unknown".into())],
        functions: vec![lightdown_ir::Function {
            name: "entry".into(),
            arity: 0,
            locals: 0,
            captures: 0,
            instructions: vec![
                Instruction {
                    opcode: Opcode::LoadBuiltin {
                        name: lightdown_ir::ConstantId(0),
                    },
                    span,
                },
                Instruction {
                    opcode: Opcode::Return,
                    span,
                },
            ],
        }],
        entry: FunctionId(0),
    })
    .expect_err("unknown builtin");
    assert!(matches!(
        unknown_builtin,
        VmError::UnknownBuiltin { name, .. } if name == "unknown"
    ));

    let calling_non_callable = execute_program(&Program {
        version: "0.1.0".into(),
        metadata_span: span,
        constants: vec![Constant::String("x".into())],
        functions: vec![lightdown_ir::Function {
            name: "entry".into(),
            arity: 0,
            locals: 0,
            captures: 0,
            instructions: vec![
                Instruction {
                    opcode: Opcode::PushConst {
                        id: lightdown_ir::ConstantId(0),
                    },
                    span,
                },
                Instruction {
                    opcode: Opcode::Call { argc: 0 },
                    span,
                },
            ],
        }],
        entry: FunctionId(0),
    })
    .expect_err("calling non-callable value");
    assert!(matches!(
        calling_non_callable,
        VmError::NonCallableValue {
            found: "string",
            ..
        }
    ));
}

fn call_callee_span(module: &lightdown_ir::Module) -> lightdown_ir::Span {
    let lightdown_ir::ExprKind::Call { callee, .. } = &module.body.kind else {
        panic!("expected call");
    };
    callee.span
}
