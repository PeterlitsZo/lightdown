use std::fs;
use std::path::Path;

const RENDER_CASES: &[&str] = &[
    "0001-renders-document-as-html-fragment",
    "0002-escapes-text-and-attribute-values",
    "0003-renders-inline-and-table-elements",
];

#[test]
fn renders_fixture_cases() {
    for case in RENDER_CASES {
        let case_path = testcase_path(case);
        let input = fs::read_to_string(case_path.join("input")).expect("input fixture is readable");
        let expected = read_output_fixture(&case_path);

        let actual = lightdown_html::render(&input).expect("fixture renders");

        assert_eq!(actual, expected.trim_end_matches('\n'), "case {case}");
    }
}

#[test]
fn renders_documents_via_the_bytecode_path() {
    let document = lightdown_ir::execute_document(
        &lightdown_ir::compile_module(
            &lightdown_ir::parse(
                r#"(doc {:meta {:version "0.1.0"}} (p (text "Rendered through bytecode.")))"#,
            )
            .expect("module parses"),
        )
        .expect("module compiles"),
    )
    .expect("program executes");

    let html = lightdown_html::render_document(&document).expect("document renders");

    assert_eq!(html, "<p>Rendered through bytecode.</p>");
}

fn testcase_path(case: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("testcases")
        .join(case)
}

fn read_output_fixture(case_path: &Path) -> String {
    let output = fs::read_to_string(case_path.join("output")).expect("output fixture is readable");
    let mut expected = String::new();

    for line in output.trim_end_matches('\n').lines() {
        if line.len() < 6 {
            panic!("invalid output fixture line: {line}");
        }

        let (_, content) = line
            .split_once(" | ")
            .unwrap_or_else(|| panic!("invalid output fixture line: {line}"));

        if line.as_bytes()[0].is_ascii_digit() && !expected.is_empty() {
            expected.push('\n');
        }
        expected.push_str(content);
    }

    expected
}
