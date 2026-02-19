/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * WIT (WebAssembly Interface Types) Generator for Ark.
 *
 * Generates .wit interface definitions from Ark function signatures,
 * enabling Ark WASM modules to participate in the Component Model ecosystem.
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 *
 * PATENT NOTICE: Protected by US Patent App #63/935,467.
 * NO IMPLIED LICENSE to rights of Mohamad Al-Zawahreh or Sovereign Systems.
 */

use crate::ast::{ArkNode, FunctionDef, Statement};
use crate::types::ArkType;
use std::fmt;

// =============================================================================
// Error Types
// =============================================================================

#[derive(Debug, Clone)]
pub struct WitGenError {
    pub message: String,
}

impl fmt::Display for WitGenError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "WIT generation error: {}", self.message)
    }
}

impl std::error::Error for WitGenError {}

// =============================================================================
// WIT Type Mapping
// =============================================================================

/// Maps an ArkType to its WIT type representation.
pub fn ark_type_to_wit(ty: &ArkType) -> String {
    match ty {
        ArkType::Integer => "s64".to_string(),
        ArkType::String => "string".to_string(),
        ArkType::Boolean => "bool".to_string(),
        ArkType::Unit => String::new(), // void / no return
        ArkType::Float => "float64".to_string(),
        ArkType::List(inner) => format!("list<{}>", ark_type_to_wit(inner)),
        ArkType::Map(key, val) => {
            format!(
                "list<tuple<{}, {}>>",
                ark_type_to_wit(key),
                ark_type_to_wit(val)
            )
        }
        ArkType::Function(inputs, output) => {
            let params: Vec<String> = inputs.iter().map(|t| ark_type_to_wit(t)).collect();
            let ret = ark_type_to_wit(output);
            format!("/* func({}) -> {} */", params.join(", "), ret)
        }
        ArkType::Struct(name, _fields) => {
            // Reference to a named record type
            to_wit_ident(name)
        }
        ArkType::Optional(inner) => format!("option<{}>", ark_type_to_wit(inner)),
        ArkType::Any => "/* any */ s64".to_string(),
        ArkType::Unknown => "/* unknown */ s64".to_string(),
        // Legacy linear types map to their inner name
        ArkType::Linear(name) | ArkType::Affine(name) | ArkType::Shared(name) => to_wit_ident(name),
    }
}

/// Convert a name to a valid WIT identifier (kebab-case).
fn to_wit_ident(name: &str) -> String {
    let mut result = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push('-');
            result.push(c.to_lowercase().next().unwrap_or(c));
        } else if c == '_' {
            result.push('-');
        } else {
            result.push(c.to_lowercase().next().unwrap_or(c));
        }
    }
    result
}

// =============================================================================
// WIT Interface Generator
// =============================================================================

/// Represents a WIT interface definition.
#[derive(Debug, Clone)]
pub struct WitInterface {
    /// Package name (e.g., "ark:program")
    pub package: String,
    /// Interface name
    pub name: String,
    /// Exported functions
    pub functions: Vec<WitFunction>,
    /// Record type definitions
    pub records: Vec<WitRecord>,
}

/// A function signature in WIT.
#[derive(Debug, Clone)]
pub struct WitFunction {
    pub name: String,
    pub params: Vec<(String, String)>, // (name, wit_type)
    pub result: Option<String>,        // None = void
}

/// A record type definition in WIT.
#[derive(Debug, Clone)]
pub struct WitRecord {
    pub name: String,
    pub fields: Vec<(String, String)>, // (name, wit_type)
}

impl WitInterface {
    /// Generate a WIT interface definition from an ArkNode AST.
    pub fn from_ast(node: &ArkNode, package_name: &str) -> Result<Self, WitGenError> {
        let mut iface = WitInterface {
            package: package_name.to_string(),
            name: "exports".to_string(),
            functions: Vec::new(),
            records: Vec::new(),
        };

        // Extract exported functions
        let all_func_defs = Self::extract_functions(node);

        // Backward compatibility: if ANY function has #[export], only export those
        let has_any_export = all_func_defs
            .iter()
            .any(|f| f.attributes.iter().any(|a| a == "export"));
        let func_defs: Vec<&FunctionDef> = if has_any_export {
            all_func_defs
                .iter()
                .filter(|f| f.attributes.iter().any(|a| a == "export"))
                .collect()
        } else {
            all_func_defs.iter().collect()
        };

        for func_def in &func_defs {
            // Skip internal/intrinsic functions
            if func_def.name.starts_with("intrinsic_") || func_def.name.starts_with('_') {
                continue;
            }

            let params: Vec<(String, String)> = func_def
                .inputs
                .iter()
                .map(|(name, ty)| (to_wit_ident(name), ark_type_to_wit(ty)))
                .collect();

            let result = match &func_def.output {
                ArkType::Unit => None,
                ty => Some(ark_type_to_wit(ty)),
            };

            iface.functions.push(WitFunction {
                name: to_wit_ident(&func_def.name),
                params,
                result,
            });
        }

        // Extract struct declarations for record types
        Self::extract_records(node, &mut iface.records);

        Ok(iface)
    }

    /// Render the WIT interface to a string.
    pub fn render(&self) -> String {
        let mut out = String::new();

        // Package declaration
        out.push_str(&format!("package {};\n\n", self.package));

        // Interface block
        out.push_str(&format!("interface {} {{\n", self.name));

        // Record types
        for record in &self.records {
            out.push_str(&format!("  record {} {{\n", record.name));
            for (name, ty) in &record.fields {
                out.push_str(&format!("    {}: {},\n", name, ty));
            }
            out.push_str("  }\n\n");
        }

        // Functions
        for func in &self.functions {
            let params: Vec<String> = func
                .params
                .iter()
                .map(|(name, ty)| format!("{}: {}", name, ty))
                .collect();

            if let Some(ref ret) = func.result {
                out.push_str(&format!(
                    "  {}: func({}) -> {};\n",
                    func.name,
                    params.join(", "),
                    ret
                ));
            } else {
                out.push_str(&format!("  {}: func({});\n", func.name, params.join(", ")));
            }
        }

        out.push_str("}\n\n");

        // World definition
        out.push_str(&format!(
            "world {} {{\n  export {};\n}}\n",
            to_wit_ident(&self.package.replace(':', "-")),
            self.name
        ));

        out
    }

    fn extract_functions(node: &ArkNode) -> Vec<FunctionDef> {
        let mut defs = Vec::new();
        match node {
            ArkNode::Function(f) => defs.push(f.clone()),
            ArkNode::Statement(Statement::Function(f)) => defs.push(f.clone()),
            ArkNode::Statement(Statement::Block(stmts)) => {
                for stmt in stmts {
                    if let Statement::Function(f) = stmt {
                        defs.push(f.clone());
                    }
                }
            }
            _ => {}
        }
        defs
    }

    fn extract_records(node: &ArkNode, records: &mut Vec<WitRecord>) {
        match node {
            ArkNode::Statement(Statement::StructDecl(s)) => {
                let fields: Vec<(String, String)> = s
                    .fields
                    .iter()
                    .map(|(name, ty)| (to_wit_ident(name), ark_type_to_wit(ty)))
                    .collect();
                records.push(WitRecord {
                    name: to_wit_ident(&s.name),
                    fields,
                });
            }
            ArkNode::Statement(Statement::Block(stmts)) => {
                for stmt in stmts {
                    if let Statement::StructDecl(s) = stmt {
                        let fields: Vec<(String, String)> = s
                            .fields
                            .iter()
                            .map(|(name, ty)| (to_wit_ident(name), ark_type_to_wit(ty)))
                            .collect();
                        records.push(WitRecord {
                            name: to_wit_ident(&s.name),
                            fields,
                        });
                    }
                }
            }
            _ => {}
        }
    }
}

// =============================================================================
// WASM Binary Validation (via wasmparser)
// =============================================================================

/// Validate a WASM binary using wasmparser.
/// Returns Ok(()) if valid, or a descriptive error.
pub fn validate_wasm(bytes: &[u8]) -> Result<(), WitGenError> {
    let mut validator = wasmparser::Validator::new();
    validator
        .validate_all(bytes)
        .map(|_| ())
        .map_err(|e| WitGenError {
            message: format!("WASM validation failed: {:?}", e),
        })
}

// =============================================================================
// CLI Integration — Generate WIT from .ark
// =============================================================================

/// Generate a WIT interface definition string from an ArkNode AST.
pub fn generate_wit(node: &ArkNode, package_name: &str) -> Result<String, WitGenError> {
    let iface = WitInterface::from_ast(node, package_name)?;
    Ok(iface.render())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Expression, MastNode, StructDecl};

    /// Helper to create a MastNode from an ArkNode (mirrors wasm_codegen tests).
    fn mast(content: ArkNode) -> MastNode {
        MastNode {
            hash: "test_hash".to_string(),
            content,
            span: None,
        }
    }

    #[test]
    fn test_ark_type_to_wit_primitives() {
        assert_eq!(ark_type_to_wit(&ArkType::Integer), "s64");
        assert_eq!(ark_type_to_wit(&ArkType::String), "string");
        assert_eq!(ark_type_to_wit(&ArkType::Boolean), "bool");
        assert_eq!(ark_type_to_wit(&ArkType::Unit), "");
        assert_eq!(ark_type_to_wit(&ArkType::Float), "float64");
    }

    #[test]
    fn test_ark_type_to_wit_list() {
        let list_type = ArkType::List(Box::new(ArkType::Integer));
        assert_eq!(ark_type_to_wit(&list_type), "list<s64>");
    }

    #[test]
    fn test_ark_type_to_wit_optional() {
        let opt = ArkType::Optional(Box::new(ArkType::String));
        assert_eq!(ark_type_to_wit(&opt), "option<string>");
    }

    #[test]
    fn test_ark_type_to_wit_map() {
        let map = ArkType::Map(Box::new(ArkType::String), Box::new(ArkType::Integer));
        assert_eq!(ark_type_to_wit(&map), "list<tuple<string, s64>>");
    }

    #[test]
    fn test_to_wit_ident() {
        assert_eq!(to_wit_ident("MyStruct"), "my-struct");
        assert_eq!(to_wit_ident("get_value"), "get-value");
        assert_eq!(to_wit_ident("simple"), "simple");
    }

    #[test]
    fn test_wit_interface_from_ast() {
        let func = FunctionDef {
            name: "add_numbers".to_string(),
            inputs: vec![
                ("a".to_string(), ArkType::Integer),
                ("b".to_string(), ArkType::Integer),
            ],
            output: ArkType::Integer,
            body: Box::new(mast(ArkNode::Expression(Expression::Integer(0)))),
            attributes: vec![],
        };

        let program = ArkNode::Statement(Statement::Block(vec![Statement::Function(func)]));

        let iface = WitInterface::from_ast(&program, "ark:test").unwrap();
        assert_eq!(iface.functions.len(), 1);
        assert_eq!(iface.functions[0].name, "add-numbers");
        assert_eq!(iface.functions[0].params.len(), 2);
        assert_eq!(iface.functions[0].result, Some("s64".to_string()));
    }

    #[test]
    fn test_wit_render() {
        let iface = WitInterface {
            package: "ark:example".to_string(),
            name: "exports".to_string(),
            functions: vec![
                WitFunction {
                    name: "greet".to_string(),
                    params: vec![("name".to_string(), "string".to_string())],
                    result: Some("string".to_string()),
                },
                WitFunction {
                    name: "compute".to_string(),
                    params: vec![
                        ("x".to_string(), "s64".to_string()),
                        ("y".to_string(), "s64".to_string()),
                    ],
                    result: Some("s64".to_string()),
                },
            ],
            records: vec![WitRecord {
                name: "point".to_string(),
                fields: vec![
                    ("x".to_string(), "s64".to_string()),
                    ("y".to_string(), "s64".to_string()),
                ],
            }],
        };

        let wit = iface.render();
        assert!(wit.contains("package ark:example;"));
        assert!(wit.contains("interface exports {"));
        assert!(wit.contains("greet: func(name: string) -> string;"));
        assert!(wit.contains("compute: func(x: s64, y: s64) -> s64;"));
        assert!(wit.contains("record point {"));
        assert!(wit.contains("x: s64,"));
        assert!(wit.contains("world ark-example {"));
    }

    #[test]
    fn test_wit_with_struct_decl() {
        let struct_decl = StructDecl {
            name: "Vector3".to_string(),
            fields: vec![
                ("x".to_string(), ArkType::Float),
                ("y".to_string(), ArkType::Float),
                ("z".to_string(), ArkType::Float),
            ],
        };

        let program =
            ArkNode::Statement(Statement::Block(vec![Statement::StructDecl(struct_decl)]));

        let iface = WitInterface::from_ast(&program, "ark:math").unwrap();
        assert_eq!(iface.records.len(), 1);
        assert_eq!(iface.records[0].name, "vector3");
        assert_eq!(iface.records[0].fields.len(), 3);
    }

    #[test]
    fn test_validate_wasm_invalid() {
        let bad_bytes = vec![0x00, 0x01, 0x02, 0x03];
        let result = validate_wasm(&bad_bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_wasm_from_codegen() {
        use crate::wasm_codegen::WasmCodegen;

        // Compile a trivial program and validate the binary
        let program = ArkNode::Statement(Statement::Block(vec![Statement::Expression(
            Expression::Integer(42),
        )]));

        let wasm_bytes = WasmCodegen::compile_to_bytes(&program).unwrap();
        let result = validate_wasm(&wasm_bytes);
        assert!(result.is_ok(), "Validation failed: {:?}", result.err());
    }

    #[test]
    fn test_generate_wit_full_pipeline() {
        let func = FunctionDef {
            name: "factorial".to_string(),
            inputs: vec![("n".to_string(), ArkType::Integer)],
            output: ArkType::Integer,
            body: Box::new(mast(ArkNode::Expression(Expression::Integer(1)))),
            attributes: vec![],
        };

        let program = ArkNode::Statement(Statement::Block(vec![Statement::Function(func)]));

        let wit = generate_wit(&program, "ark:math").unwrap();
        assert!(wit.contains("factorial: func(n: s64) -> s64;"));
        assert!(wit.contains("package ark:math;"));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Phase 13: Selective Export Tests
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_wit_selective_export_filters_unexported() {
        // Only the #[export]-marked function should appear in WIT
        let exported_func = FunctionDef {
            name: "public_api".to_string(),
            inputs: vec![("x".to_string(), ArkType::Integer)],
            output: ArkType::Integer,
            body: Box::new(mast(ArkNode::Expression(Expression::Integer(0)))),
            attributes: vec!["export".to_string()],
        };
        let internal_func = FunctionDef {
            name: "internal_helper".to_string(),
            inputs: vec![("y".to_string(), ArkType::Integer)],
            output: ArkType::Integer,
            body: Box::new(mast(ArkNode::Expression(Expression::Integer(0)))),
            attributes: vec![],
        };

        let program = ArkNode::Statement(Statement::Block(vec![
            Statement::Function(exported_func),
            Statement::Function(internal_func),
        ]));

        let iface = WitInterface::from_ast(&program, "ark:api").unwrap();
        assert_eq!(
            iface.functions.len(),
            1,
            "Only #[export] function should be in WIT"
        );
        assert_eq!(iface.functions[0].name, "public-api");
    }

    #[test]
    fn test_wit_backward_compat_exports_all() {
        // When no functions have #[export], all functions should appear
        let func_a = FunctionDef {
            name: "func_a".to_string(),
            inputs: vec![],
            output: ArkType::Integer,
            body: Box::new(mast(ArkNode::Expression(Expression::Integer(0)))),
            attributes: vec![],
        };
        let func_b = FunctionDef {
            name: "func_b".to_string(),
            inputs: vec![],
            output: ArkType::Integer,
            body: Box::new(mast(ArkNode::Expression(Expression::Integer(0)))),
            attributes: vec![],
        };

        let program = ArkNode::Statement(Statement::Block(vec![
            Statement::Function(func_a),
            Statement::Function(func_b),
        ]));

        let iface = WitInterface::from_ast(&program, "ark:legacy").unwrap();
        assert_eq!(
            iface.functions.len(),
            2,
            "Both functions should be exported when no #[export] is used"
        );
    }
}
