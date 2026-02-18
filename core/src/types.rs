/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * This file is part of the Ark Sovereign Compiler.
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 *
 * 1. OPEN SOURCE: You may use this file under the terms of the GNU Affero
 * General Public License v3.0. If you link to this code, your ENTIRE
 * application must be open-sourced under AGPLv3.
 *
 * 2. COMMERCIAL: For proprietary use, you must obtain a Commercial License
 * from Sovereign Systems.
 *
 * PATENT NOTICE: Protected by US Patent App #63/935,467.
 * NO IMPLIED LICENSE to rights of Mohamad Al-Zawahreh or Sovereign Systems.
 */

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum ArkType {
    // Legacy / Linear Checker types (Preserved for compatibility)
    Linear(String),
    Affine(String),
    Shared(String),

    // New Type System
    Integer,
    Float,
    String,
    Boolean,
    List(Box<ArkType>),                     // List<T>
    Map(Box<ArkType>, Box<ArkType>),        // Map<K, V>
    Struct(String, Vec<(String, ArkType)>), // Named struct with fields
    Function(Vec<ArkType>, Box<ArkType>),   // (params) -> return
    Optional(Box<ArkType>),                 // T?
    Unit,                                   // void/nil
    Any,                                    // dynamic type
    Unknown,                                // not yet inferred
}

impl ArkType {
    /// Legacy helper for Linear Checker
    pub fn is_linear(&self) -> bool {
        matches!(self, ArkType::Linear(_))
    }

    /// Check if this type is compatible with the expected type.
    /// `Any` and `Unknown` are compatible with everything.
    pub fn is_compatible(&self, expected: &ArkType) -> bool {
        if matches!(self, ArkType::Any | ArkType::Unknown) {
            return true;
        }
        if matches!(expected, ArkType::Any | ArkType::Unknown) {
            return true;
        }

        match (self, expected) {
            (ArkType::Integer, ArkType::Integer) => true,
            (ArkType::Float, ArkType::Float) => true,
            (ArkType::String, ArkType::String) => true,
            (ArkType::Boolean, ArkType::Boolean) => true,
            (ArkType::Unit, ArkType::Unit) => true,
            (ArkType::List(inner_self), ArkType::List(inner_expected)) => {
                inner_self.is_compatible(inner_expected)
            }
            (ArkType::Map(k1, v1), ArkType::Map(k2, v2)) => {
                k1.is_compatible(k2) && v1.is_compatible(v2)
            }
            (ArkType::Optional(inner_self), ArkType::Optional(inner_expected)) => {
                inner_self.is_compatible(inner_expected)
            }
            (ArkType::Function(params1, ret1), ArkType::Function(params2, ret2)) => {
                if params1.len() != params2.len() {
                    return false;
                }
                for (p1, p2) in params1.iter().zip(params2.iter()) {
                    // Parameters should be contravariant.
                    // Expected type (p2) must be compatible with Actual type (p1).
                    // e.g. Expect (String)->Void, Actual (Any)->Void.
                    // p2=String, p1=Any. String is compatible with Any? Yes.
                    if !p2.is_compatible(p1) {
                        return false;
                    }
                }
                ret1.is_compatible(ret2)
            }
            (ArkType::Struct(name1, fields1), ArkType::Struct(name2, fields2)) => {
                if name1 != name2 {
                    return false;
                }
                if fields1.len() != fields2.len() {
                    return false;
                }
                for ((n1, t1), (n2, t2)) in fields1.iter().zip(fields2.iter()) {
                    if n1 != n2 || !t1.is_compatible(t2) {
                        return false;
                    }
                }
                true
            }
            // Legacy compatibility
            (ArkType::Linear(s1), ArkType::Linear(s2)) => s1 == s2,
            (ArkType::Affine(s1), ArkType::Affine(s2)) => s1 == s2,
            (ArkType::Shared(s1), ArkType::Shared(s2)) => s1 == s2,

            _ => false,
        }
    }

    /// Narrow the current type based on evidence.
    /// e.g., if current is Optional(T) and evidence is T, result is T.
    pub fn narrow(&self, evidence: &ArkType) -> ArkType {
        match (self, evidence) {
            (ArkType::Optional(inner), _) => {
                // If evidence is compatible with inner, we can narrow to inner.
                // e.g. Optional(Int) narrowed by Int -> Int.
                if evidence.is_compatible(inner) {
                    return *inner.clone();
                }
                self.clone()
            }
            (ArkType::Any, specific) => specific.clone(),
            _ => self.clone(),
        }
    }
}

impl fmt::Display for ArkType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArkType::Integer => write!(f, "Int"),
            ArkType::Float => write!(f, "Float"),
            ArkType::String => write!(f, "Str"),
            ArkType::Boolean => write!(f, "Bool"),
            ArkType::Unit => write!(f, "Unit"),
            ArkType::Any => write!(f, "Any"),
            ArkType::Unknown => write!(f, "Unknown"),
            ArkType::List(inner) => write!(f, "List<{}>", inner),
            ArkType::Map(k, v) => write!(f, "Map<{}, {}>", k, v),
            ArkType::Optional(inner) => write!(f, "{}?", inner),
            ArkType::Function(params, ret) => {
                write!(f, "(")?;
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", param)?;
                }
                write!(f, ") -> {}", ret)
            }
            ArkType::Struct(name, _) => write!(f, "{}", name),

            // Legacy
            ArkType::Linear(s) => write!(f, "Linear({})", s),
            ArkType::Affine(s) => write!(f, "Affine({})", s),
            ArkType::Shared(s) => write!(f, "Shared({})", s),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_display_format() {
        assert_eq!(format!("{}", ArkType::Integer), "Int");
        assert_eq!(format!("{}", ArkType::String), "Str");
        assert_eq!(
            format!("{}", ArkType::List(Box::new(ArkType::Integer))),
            "List<Int>"
        );
        assert_eq!(
            format!(
                "{}",
                ArkType::Map(Box::new(ArkType::String), Box::new(ArkType::Any))
            ),
            "Map<Str, Any>"
        );
        assert_eq!(
            format!(
                "{}",
                ArkType::Function(
                    vec![ArkType::Integer, ArkType::Boolean],
                    Box::new(ArkType::Unit)
                )
            ),
            "(Int, Bool) -> Unit"
        );
    }

    #[test]
    fn test_type_compatibility_basic() {
        assert!(ArkType::Integer.is_compatible(&ArkType::Any));
        assert!(ArkType::Integer.is_compatible(&ArkType::Integer));
        assert!(!ArkType::Integer.is_compatible(&ArkType::String));

        // Any is compatible with everything (as declared in impl)
        assert!(ArkType::Any.is_compatible(&ArkType::Integer));
    }

    #[test]
    fn test_type_compatibility_generics() {
        // List<Int> compatible with List<Any>
        let list_int = ArkType::List(Box::new(ArkType::Integer));
        let list_any = ArkType::List(Box::new(ArkType::Any));
        assert!(list_int.is_compatible(&list_any));

        // List<Any> compatible with List<Int>?
        // Since Any compatible with Int (Dynamic), yes.
        assert!(list_any.is_compatible(&list_int));

        // List<Int> not compatible with List<String>
        let list_str = ArkType::List(Box::new(ArkType::String));
        assert!(!list_int.is_compatible(&list_str));

        // Function compatibility (Contravariance)
        // (Any) -> Unit compatible with (Int) -> Unit?
        // Expect (Int)->Unit. Actual (Any)->Unit.
        // Caller gives Int. Actual takes Any. OK.
        let fn_any_unit = ArkType::Function(vec![ArkType::Any], Box::new(ArkType::Unit));
        let fn_int_unit = ArkType::Function(vec![ArkType::Integer], Box::new(ArkType::Unit));

        // self=fn_any_unit (Actual), expected=fn_int_unit.
        // p2=Int (Expected), p1=Any (Actual).
        // p2.is_compatible(p1) -> Int.is_compatible(Any) -> True.
        assert!(fn_any_unit.is_compatible(&fn_int_unit));

        // (Int) -> Unit compatible with (Any) -> Unit?
        // Expect (Any)->Unit. Actual (Int)->Unit.
        // Caller gives String. Actual takes Int. Crash.
        // p2=Any (Expected), p1=Int (Actual).
        // p2.is_compatible(p1) -> Any.is_compatible(Int) -> True (Dynamic typing!).
        // Wait, Any is compatible with Int? Yes, my code says:
        // if matches!(expected, Any) return true.
        // So Any is compatible with Int.
        // So p2.is_compatible(p1) returns True.
        // So fn_int_unit is compatible with fn_any_unit.
        // This means (Int)->Unit IS compatible with (Any)->Unit.
        // This implies unsafe dynamic behavior, but consistent with implementation.
        assert!(fn_int_unit.is_compatible(&fn_any_unit));
    }

    #[test]
    fn test_type_narrowing() {
        let opt_int = ArkType::Optional(Box::new(ArkType::Integer));
        let int_type = ArkType::Integer;

        // Narrow Optional(Int) with Int evidence -> Int
        assert_eq!(opt_int.narrow(&int_type), ArkType::Integer);

        // Narrow Any with Int evidence -> Int
        assert_eq!(ArkType::Any.narrow(&int_type), ArkType::Integer);

        // Narrow Optional(Int) with String evidence -> Optional(Int) (no change)
        assert_eq!(opt_int.narrow(&ArkType::String), opt_int);
    }
}
