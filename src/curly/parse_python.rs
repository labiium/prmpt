//! Implements functionality for parsing Python code using Tree-sitter in order
//! to extract function/class signatures and docstrings.

use serde_json::Value as JsonValue;
use std::fs;
use tree_sitter::{Node, Parser};
use tree_sitter_python;

/// Sets up a Tree-sitter parser for Python and extracts function/class signatures
/// along with docstrings from the provided file contents.
///
/// This is used in "docs-only" modes, or to produce more descriptive prompts.
pub fn extract_python_signatures(contents: &str) -> String {
    let mut parser = Parser::new();
    let language = tree_sitter_python::language();
    parser
        .set_language(language)
        .expect("Error loading Python grammar");

    let tree = parser.parse(contents, None).unwrap();
    let root_node = tree.root_node();

    let mut signatures = String::new();

    // Before processing other nodes, check for module-level docstring
    let module_docstring = extract_module_docstring(root_node, contents);
    if !module_docstring.is_empty() {
        signatures.push_str(&module_docstring);
        signatures.push('\n');
    }

    // Process definitions starting from the root node
    let mut cursor = root_node.walk();
    for child in root_node.children(&mut cursor) {
        let child_signatures = extract_definitions(child, contents, 0);
        if !child_signatures.is_empty() {
            signatures.push_str(&child_signatures);
            signatures.push('\n');
        }
    }

    signatures
}

/// Attempts to extract a module-level docstring from a Python file.
fn extract_module_docstring(root_node: Node, source_code: &str) -> String {
    let mut cursor = root_node.walk();
    for child in root_node.children(&mut cursor) {
        if child.kind() == "expression_statement" {
            let mut expr_cursor = child.walk();
            for expr_child in child.named_children(&mut expr_cursor) {
                if expr_child.kind() == "string" {
                    let docstring_text = expr_child.utf8_text(source_code.as_bytes()).unwrap();
                    let (stripped_docstring, quote_type) = strip_quotes(docstring_text);
                    let indented_docstring = indent_docstring(stripped_docstring, "", quote_type);
                    return indented_docstring;
                }
            }
        } else if !child.kind().starts_with("comment") && !child.kind().starts_with("newline") {
            // If we encounter anything else, no module docstring is present
            break;
        }
    }
    String::new()
}

/// Recursively extracts function and class signatures from a given AST node.
fn extract_definitions(node: Node, source_code: &str, indent_level: usize) -> String {
    let mut output = String::new();

    if node.kind() == "function_definition" || node.kind() == "class_definition" {
        let signature = extract_signature_and_docstring(node, source_code, indent_level);
        output.push_str(&signature);
        output.push('\n');
    }

    // If this is a class or function, process its body to find nested definitions
    if node.kind() == "class_definition" || node.kind() == "function_definition" {
        // Find the block node (the body)
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "block" {
                // Process the statements inside the block
                let mut block_cursor = child.walk();
                for stmt in child.children(&mut block_cursor) {
                    let stmt_output = extract_definitions(stmt, source_code, indent_level + 1);
                    if !stmt_output.is_empty() {
                        output.push_str(&stmt_output);
                        output.push('\n');
                    }
                }
            }
        }
    }

    output
}

/// Extracts the signature (including decorators) and docstring from a function or class node.
fn extract_signature_and_docstring(node: Node, source_code: &str, indent_level: usize) -> String {
    let mut signature = String::new();
    let indent = "    ".repeat(indent_level);

    if node.kind() == "function_definition" || node.kind() == "class_definition" {
        signature.push_str(&indent);

        let mut cursor = node.walk();
        let children = node.children(&mut cursor);
        let mut found_signature = false;
        for child in children {
            match child.kind() {
                "decorator" => {
                    let decorator_text = child.utf8_text(source_code.as_bytes()).unwrap();
                    signature.push_str(&indent);
                    signature.push_str(decorator_text);
                    signature.push('\n');
                }
                "def" | "class" => {
                    let text = child.utf8_text(source_code.as_bytes()).unwrap();
                    signature.push_str(text);
                    signature.push(' ');
                }
                "identifier" | "parameters" | ":" => {
                    let text = child.utf8_text(source_code.as_bytes()).unwrap();
                    signature.push_str(text);
                    if child.kind() == ":" {
                        signature.push('\n');
                    }
                    found_signature = true;
                }
                "block" => {
                    // After the signature, look for the docstring
                    if found_signature {
                        let docstring = extract_docstring(child, source_code, indent_level + 1);
                        if !docstring.is_empty() {
                            signature.push_str(&docstring);
                        }
                        break; // We only need the first block
                    }
                }
                _ => {}
            }
        }
    }

    signature
}

/// Attempts to extract a docstring from a code block (e.g., the first statement in a function).
fn extract_docstring(block_node: Node, source_code: &str, indent_level: usize) -> String {
    let mut cursor = block_node.walk();
    let mut children = block_node.named_children(&mut cursor);

    let indent = "    ".repeat(indent_level);

    if let Some(first_child) = children.next() {
        if first_child.kind() == "expression_statement" {
            let mut expr_cursor = first_child.walk();
            for string_node in first_child.named_children(&mut expr_cursor) {
                if string_node.kind() == "string" {
                    // This is the docstring
                    let docstring_text = string_node.utf8_text(source_code.as_bytes()).unwrap();
                    // Strip the quotes and get the quote type
                    let (stripped_docstring, quote_type) = strip_quotes(docstring_text);
                    // Indent the docstring with the correct indentation and re-add quotes
                    let indented_docstring =
                        indent_docstring(stripped_docstring, &indent, quote_type);
                    return indented_docstring;
                }
            }
        }
    }
    String::new()
}

/// Strips quotes around a string literal and returns both the stripped text and the type of quotes used.
fn strip_quotes(s: &str) -> (&str, &str) {
    let s = s.trim();
    if (s.starts_with("\"\"\"") && s.ends_with("\"\"\""))
        || (s.starts_with("'''") && s.ends_with("'''"))
    {
        (&s[3..s.len() - 3], &s[..3]) // Return the inner content and the quote type
    } else if (s.starts_with("\"") && s.ends_with("\"")) || (s.starts_with("'") && s.ends_with("'"))
    {
        (&s[1..s.len() - 1], &s[..1])
    } else {
        (s, "")
    }
}

/// Re-indents a docstring by removing common leading whitespace and re-adding quotes if necessary.
fn indent_docstring(docstring: &str, indent: &str, quote_type: &str) -> String {
    let dedented_docstring = dedent(docstring);

    // Indent each line with the desired indentation
    let indented_docstring = dedented_docstring
        .lines()
        .map(|line| format!("{}{}", indent, line))
        .collect::<Vec<String>>()
        .join("\n");

    // Re-add the quotes with proper indentation
    if quote_type.is_empty() {
        indented_docstring
    } else {
        // For single-line docstrings, keep it in one line
        if !dedented_docstring.contains('\n') {
            format!(
                "{}{}{}{}",
                indent,
                quote_type,
                dedented_docstring.trim(),
                quote_type
            )
        } else {
            // For multi-line docstrings, place quotes on separate lines
            format!(
                "{}{}\n{}\n{}{}",
                indent, quote_type, indented_docstring, indent, quote_type
            )
        }
    }
}

/// Dedents a string by removing leading whitespace from each line and
/// optionally removing empty lines at the start.
fn dedent(s: &str) -> String {
    let lines: Vec<&str> = s.lines().collect();
    // remove first index if it is empty or whitespace-only
    let lines = if lines.first().map_or(true, |line| line.trim().is_empty()) {
        &lines[1..]
    } else {
        &lines
    };

    // remove all whitespace from the beginning of each line
    let string = lines
        .iter()
        .map(|line| line.trim_start())
        .collect::<Vec<&str>>()
        .join("\n");

    string
}

/// Attempts to read an .ipynb file from disk and parse it as JSON.
/// This helper function is used inside the main code to process Jupyter notebooks if needed.
pub fn maybe_read_notebook(file_path: &str) -> Option<JsonValue> {
    if let Ok(notebook_contents) = fs::read_to_string(file_path) {
        if let Ok(notebook_json) = serde_json::from_str::<JsonValue>(&notebook_contents) {
            return Some(notebook_json);
        }
    }
    None
}
