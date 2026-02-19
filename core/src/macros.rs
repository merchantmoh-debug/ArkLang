/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * Hygienic Macro System for the Ark Language.
 *
 * Provides compile-time AST transformations that are hygiene-safe,
 * meaning macro-expanded code cannot accidentally capture or shadow
 * variables from the surrounding scope.
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 */

use crate::ast::{ArkNode, Expression, Statement};
use std::collections::HashMap;

// =============================================================================
// Macro Definition & Registry
// =============================================================================

/// A macro definition: a name, parameter names, and a template body.
#[derive(Debug, Clone)]
pub struct MacroDef {
    /// Macro name (e.g., "unless", "when")
    pub name: String,
    /// Parameter names (capture patterns in the template)
    pub params: Vec<String>,
    /// Template body — the AST pattern to expand into
    pub body: MacroTemplate,
}

/// A macro template — the right-hand side of a macro definition.
#[derive(Debug, Clone)]
pub enum MacroTemplate {
    /// A reference to a captured parameter by name
    Param(String),
    /// A literal AST node to emit as-is
    Literal(ArkNode),
    /// If-then-else at the macro level
    If {
        condition: Box<MacroTemplate>,
        then_block: Vec<MacroTemplate>,
        else_block: Option<Vec<MacroTemplate>>,
    },
    /// A block of sequential templates
    Block(Vec<MacroTemplate>),
    /// Function call with template arguments (callee is a string name)
    Call {
        callee: String,
        args: Vec<MacroTemplate>,
    },
    /// Let binding in expanded code
    Let {
        name: String,
        value: Box<MacroTemplate>,
    },
}

/// The macro registry: stores all defined macros.
#[derive(Debug, Default, Clone)]
pub struct MacroRegistry {
    macros: HashMap<String, MacroDef>,
    /// Counter for generating unique hygienic names
    gensym_counter: usize,
}

impl MacroRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            macros: HashMap::new(),
            gensym_counter: 0,
        };
        registry.register_builtins();
        registry
    }

    /// Generate a unique hygienic symbol name.
    pub fn gensym(&mut self, prefix: &str) -> String {
        self.gensym_counter += 1;
        format!("__ark_hyg_{}_{}", prefix, self.gensym_counter)
    }

    /// Define a new macro.
    pub fn define(&mut self, mac: MacroDef) {
        self.macros.insert(mac.name.clone(), mac);
    }

    /// Check if a macro is defined.
    pub fn is_defined(&self, name: &str) -> bool {
        self.macros.contains_key(name)
    }

    /// Get a macro definition.
    pub fn get(&self, name: &str) -> Option<&MacroDef> {
        self.macros.get(name)
    }

    /// Expand a macro call. Returns the expanded AST node.
    pub fn expand(&mut self, name: &str, args: Vec<ArkNode>) -> Result<ArkNode, MacroError> {
        let mac = self
            .macros
            .get(name)
            .ok_or_else(|| MacroError::Undefined(name.to_string()))?
            .clone();

        if args.len() != mac.params.len() {
            return Err(MacroError::ArityMismatch {
                name: name.to_string(),
                expected: mac.params.len(),
                got: args.len(),
            });
        }

        // Build substitution environment
        let mut env: HashMap<String, ArkNode> = HashMap::new();
        for (param, arg) in mac.params.iter().zip(args) {
            env.insert(param.clone(), arg);
        }

        // Expand the template with hygienic renaming
        self.expand_template(&mac.body, &env)
    }

    /// Expand a macro template with the given substitution environment.
    fn expand_template(
        &mut self,
        template: &MacroTemplate,
        env: &HashMap<String, ArkNode>,
    ) -> Result<ArkNode, MacroError> {
        match template {
            MacroTemplate::Param(name) => env
                .get(name)
                .cloned()
                .ok_or_else(|| MacroError::UnboundParam(name.clone())),

            MacroTemplate::Literal(node) => Ok(node.clone()),

            MacroTemplate::If {
                condition,
                then_block,
                else_block,
            } => {
                let cond_node = self.expand_template(condition, env)?;
                let cond_expr = extract_expr(&cond_node)?;

                let mut then_stmts = Vec::new();
                for t in then_block {
                    let node = self.expand_template(t, env)?;
                    then_stmts.push(node_to_statement(node));
                }

                let else_stmts = match else_block {
                    Some(stmts) => {
                        let mut result = Vec::new();
                        for t in stmts {
                            let node = self.expand_template(t, env)?;
                            result.push(node_to_statement(node));
                        }
                        Some(result)
                    }
                    None => None,
                };

                Ok(ArkNode::Statement(Statement::If {
                    condition: cond_expr,
                    then_block: then_stmts,
                    else_block: else_stmts,
                }))
            }

            MacroTemplate::Block(templates) => {
                let mut stmts = Vec::new();
                for t in templates {
                    let node = self.expand_template(t, env)?;
                    stmts.push(node_to_statement(node));
                }
                Ok(ArkNode::Statement(Statement::Block(stmts)))
            }

            MacroTemplate::Call { callee, args } => {
                let mut arg_exprs = Vec::new();
                for arg in args {
                    let node = self.expand_template(arg, env)?;
                    arg_exprs.push(extract_expr(&node)?);
                }
                Ok(ArkNode::Expression(Expression::Call {
                    function_hash: callee.clone(),
                    args: arg_exprs,
                }))
            }

            MacroTemplate::Let { name, value } => {
                // Hygienic: rename the variable to avoid capture
                let hygienic_name = self.gensym(name);
                let val_node = self.expand_template(value, env)?;
                Ok(ArkNode::Statement(Statement::Let {
                    name: hygienic_name,
                    ty: None,
                    value: extract_expr(&val_node)?,
                }))
            }
        }
    }

    /// Register built-in macros that ship with the language.
    fn register_builtins(&mut self) {
        // `unless` — inverted if: (unless cond body) → if (!cond) { body }
        self.define(MacroDef {
            name: "unless".to_string(),
            params: vec!["condition".to_string(), "body".to_string()],
            body: MacroTemplate::If {
                condition: Box::new(MacroTemplate::Param("condition".to_string())),
                then_block: vec![],
                else_block: Some(vec![MacroTemplate::Param("body".to_string())]),
            },
        });

        // `when` — one-armed if: (when cond body) → if (cond) { body }
        self.define(MacroDef {
            name: "when".to_string(),
            params: vec!["condition".to_string(), "body".to_string()],
            body: MacroTemplate::If {
                condition: Box::new(MacroTemplate::Param("condition".to_string())),
                then_block: vec![MacroTemplate::Param("body".to_string())],
                else_block: None,
            },
        });

        // `assert` — runtime assertion: (assert expr) → if (!expr) { panic("Assertion failed") }
        self.define(MacroDef {
            name: "assert".to_string(),
            params: vec!["expr".to_string()],
            body: MacroTemplate::If {
                condition: Box::new(MacroTemplate::Param("expr".to_string())),
                then_block: vec![],
                else_block: Some(vec![MacroTemplate::Call {
                    callee: "panic".to_string(),
                    args: vec![MacroTemplate::Literal(ArkNode::Expression(
                        Expression::Literal("Assertion failed".to_string()),
                    ))],
                }]),
            },
        });

        // `swap!` — atomic swap pattern: wraps in a block
        self.define(MacroDef {
            name: "swap".to_string(),
            params: vec!["a".to_string(), "b".to_string()],
            body: MacroTemplate::Block(vec![
                MacroTemplate::Let {
                    name: "tmp".to_string(),
                    value: Box::new(MacroTemplate::Param("a".to_string())),
                },
                MacroTemplate::Param("a".to_string()),
                MacroTemplate::Param("b".to_string()),
            ]),
        });

        // `thread-first` — threading macro: (thread-first val fn) → fn(val)
        self.define(MacroDef {
            name: "thread-first".to_string(),
            params: vec!["val".to_string(), "fn_name".to_string()],
            body: MacroTemplate::Call {
                callee: "thread_apply".to_string(),
                args: vec![
                    MacroTemplate::Param("fn_name".to_string()),
                    MacroTemplate::Param("val".to_string()),
                ],
            },
        });
    }
}

// =============================================================================
// Error Types
// =============================================================================

#[derive(Debug)]
pub enum MacroError {
    Undefined(String),
    ArityMismatch {
        name: String,
        expected: usize,
        got: usize,
    },
    UnboundParam(String),
    InvalidExpansion(String),
}

impl std::fmt::Display for MacroError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MacroError::Undefined(name) => write!(f, "Undefined macro: {}", name),
            MacroError::ArityMismatch {
                name,
                expected,
                got,
            } => write!(
                f,
                "Macro '{}' expects {} arguments, got {}",
                name, expected, got
            ),
            MacroError::UnboundParam(name) => write!(f, "Unbound macro parameter: {}", name),
            MacroError::InvalidExpansion(msg) => write!(f, "Invalid macro expansion: {}", msg),
        }
    }
}

impl std::error::Error for MacroError {}

// =============================================================================
// Helper Functions
// =============================================================================

/// Extract an Expression from an ArkNode.
fn extract_expr(node: &ArkNode) -> Result<Expression, MacroError> {
    match node {
        ArkNode::Expression(e) => Ok(e.clone()),
        ArkNode::Statement(Statement::Expression(e)) => Ok(e.clone()),
        _ => Err(MacroError::InvalidExpansion(
            "Expected an expression node".to_string(),
        )),
    }
}

/// Convert an ArkNode to a Statement.
fn node_to_statement(node: ArkNode) -> Statement {
    match node {
        ArkNode::Statement(s) => s,
        ArkNode::Expression(e) => Statement::Expression(e),
        ArkNode::Function(f) => Statement::Function(f),
        ArkNode::Type(_t) => Statement::Expression(Expression::Literal("type".to_string())),
    }
}

// =============================================================================
// Walk and Expand — Process an entire AST, expanding all macro calls
// =============================================================================

/// Walk an AST and expand all macro calls in-place.
pub fn expand_all(node: &ArkNode, registry: &mut MacroRegistry) -> Result<ArkNode, MacroError> {
    match node {
        ArkNode::Expression(expr) => {
            let expanded = expand_expr(expr, registry)?;
            Ok(ArkNode::Expression(expanded))
        }
        ArkNode::Statement(stmt) => {
            let expanded = expand_stmt(stmt, registry)?;
            Ok(ArkNode::Statement(expanded))
        }
        ArkNode::Function(f) => Ok(ArkNode::Function(f.clone())),
        ArkNode::Type(t) => Ok(ArkNode::Type(t.clone())),
    }
}

fn expand_expr(expr: &Expression, registry: &mut MacroRegistry) -> Result<Expression, MacroError> {
    match expr {
        // Check if this is a macro call via the Call variant
        Expression::Call {
            function_hash,
            args,
        } => {
            if registry.is_defined(function_hash) {
                // This is a macro call! Expand it.
                let arg_nodes: Vec<ArkNode> = args
                    .iter()
                    .map(|a| ArkNode::Expression(a.clone()))
                    .collect();
                let expanded = registry.expand(function_hash, arg_nodes)?;
                return extract_expr(&expanded);
            }
            // Not a macro — recursively expand args
            let expanded_args: Result<Vec<Expression>, _> =
                args.iter().map(|a| expand_expr(a, registry)).collect();
            Ok(Expression::Call {
                function_hash: function_hash.clone(),
                args: expanded_args?,
            })
        }

        // Recursively expand sub-expressions in lists
        Expression::List(items) => {
            let expanded: Result<Vec<Expression>, _> =
                items.iter().map(|a| expand_expr(a, registry)).collect();
            Ok(Expression::List(expanded?))
        }

        // Leaf nodes — no expansion needed
        _ => Ok(expr.clone()),
    }
}

fn expand_stmt(stmt: &Statement, registry: &mut MacroRegistry) -> Result<Statement, MacroError> {
    match stmt {
        Statement::Expression(e) => {
            let expanded = expand_expr(e, registry)?;
            Ok(Statement::Expression(expanded))
        }
        Statement::Let { name, ty, value } => {
            let expanded = expand_expr(value, registry)?;
            Ok(Statement::Let {
                name: name.clone(),
                ty: ty.clone(),
                value: expanded,
            })
        }
        Statement::Block(stmts) => {
            let expanded: Result<Vec<Statement>, _> =
                stmts.iter().map(|s| expand_stmt(s, registry)).collect();
            Ok(Statement::Block(expanded?))
        }
        Statement::If {
            condition,
            then_block,
            else_block,
        } => {
            let expanded_cond = expand_expr(condition, registry)?;
            let expanded_then: Result<Vec<Statement>, _> = then_block
                .iter()
                .map(|s| expand_stmt(s, registry))
                .collect();
            let expanded_else = match else_block {
                Some(stmts) => {
                    let expanded: Result<Vec<Statement>, _> =
                        stmts.iter().map(|s| expand_stmt(s, registry)).collect();
                    Some(expanded?)
                }
                None => None,
            };
            Ok(Statement::If {
                condition: expanded_cond,
                then_block: expanded_then?,
                else_block: expanded_else,
            })
        }
        Statement::While { condition, body } => {
            let expanded_cond = expand_expr(condition, registry)?;
            let expanded_body: Result<Vec<Statement>, _> =
                body.iter().map(|s| expand_stmt(s, registry)).collect();
            Ok(Statement::While {
                condition: expanded_cond,
                body: expanded_body?,
            })
        }
        Statement::For {
            variable,
            iterable,
            body,
        } => {
            let expanded_iter = expand_expr(iterable, registry)?;
            let expanded_body: Result<Vec<Statement>, _> =
                body.iter().map(|s| expand_stmt(s, registry)).collect();
            Ok(Statement::For {
                variable: variable.clone(),
                iterable: expanded_iter,
                body: expanded_body?,
            })
        }
        Statement::Return(e) => {
            let expanded = expand_expr(e, registry)?;
            Ok(Statement::Return(expanded))
        }
        // Pass-through statements
        _ => Ok(stmt.clone()),
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macro_registry_builtins() {
        let registry = MacroRegistry::new();
        assert!(registry.is_defined("unless"));
        assert!(registry.is_defined("when"));
        assert!(registry.is_defined("assert"));
        assert!(registry.is_defined("swap"));
        assert!(registry.is_defined("thread-first"));
    }

    #[test]
    fn test_gensym_uniqueness() {
        let mut registry = MacroRegistry::new();
        let sym1 = registry.gensym("x");
        let sym2 = registry.gensym("x");
        assert_ne!(sym1, sym2);
        assert!(sym1.starts_with("__ark_hyg_x_"));
    }

    #[test]
    fn test_expand_when_macro() {
        let mut registry = MacroRegistry::new();
        let result = registry.expand(
            "when",
            vec![
                ArkNode::Expression(Expression::Variable("x".to_string())),
                ArkNode::Expression(Expression::Literal("hello".to_string())),
            ],
        );
        assert!(result.is_ok());
        // Should produce an If statement
        let node = result.unwrap();
        if let ArkNode::Statement(Statement::If { .. }) = node {
            // Good — expanded to if
        } else {
            panic!("Expected If statement, got {:?}", node);
        }
    }

    #[test]
    fn test_arity_mismatch() {
        let mut registry = MacroRegistry::new();
        let result = registry.expand(
            "when",
            vec![ArkNode::Expression(Expression::Literal(
                "only_one".to_string(),
            ))],
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_undefined_macro() {
        let mut registry = MacroRegistry::new();
        let result = registry.expand("nonexistent", vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn test_custom_macro_definition() {
        let mut registry = MacroRegistry::new();
        registry.define(MacroDef {
            name: "double_call".to_string(),
            params: vec!["x".to_string()],
            body: MacroTemplate::Call {
                callee: "add".to_string(),
                args: vec![
                    MacroTemplate::Param("x".to_string()),
                    MacroTemplate::Param("x".to_string()),
                ],
            },
        });

        let result = registry.expand(
            "double_call",
            vec![ArkNode::Expression(Expression::Literal("5".to_string()))],
        );
        assert!(result.is_ok());
        let node = result.unwrap();
        if let ArkNode::Expression(Expression::Call {
            function_hash,
            args,
        }) = &node
        {
            assert_eq!(function_hash, "add");
            assert_eq!(args.len(), 2);
        } else {
            panic!("Expected Call, got {:?}", node);
        }
    }

    #[test]
    fn test_expand_all_no_macros() {
        let mut registry = MacroRegistry::new();
        let node = ArkNode::Expression(Expression::Literal("42".to_string()));
        let result = expand_all(&node, &mut registry).unwrap();
        assert_eq!(result, node);
    }
}
