/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * ADN — Ark Data Notation
 * A human-readable data serialization format for the Ark language.
 *
 * ADN is to Ark what EDN is to Clojure, but with content-addressed integrity.
 * It supports: integers, strings, booleans, nil, vectors, maps, keywords,
 * persistent collections (#pvec, #pmap), and tagged literals.
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 */

use crate::persistent::{PMap, PVec, format_value_adn};
use crate::runtime::Value;
use std::collections::HashMap;

// =============================================================================
// ADN Serialization (Value → ADN String)
// =============================================================================

/// Serialize an Ark Value to ADN (Ark Data Notation) string.
///
/// ADN format:
///   42            → integer
///   "hello"       → string
///   true/false    → boolean
///   nil           → unit/null
///   [1 2 3]       → list
///   {:a 1, :b 2}  → struct/map
///   #pvec[1 2 3]  → persistent vector
///   #pmap{:a 1}   → persistent map
///   #<fn>         → function reference
///   #buf[N bytes] → buffer
pub fn to_adn(value: &Value) -> String {
    format_value_adn(value)
}

/// Serialize an Ark Value to pretty-printed ADN with indentation.
pub fn to_adn_pretty(value: &Value) -> String {
    to_adn_indented(value, 0)
}

fn to_adn_indented(value: &Value, indent: usize) -> String {
    let pad = "  ".repeat(indent);
    match value {
        Value::Integer(i) => i.to_string(),
        Value::String(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
        Value::Boolean(b) => b.to_string(),
        Value::Unit => "nil".to_string(),
        Value::List(l) if l.is_empty() => "[]".to_string(),
        Value::List(l) if l.len() <= 5 && !has_nested(l) => {
            let items: Vec<String> = l.iter().map(|v| to_adn_indented(v, 0)).collect();
            format!("[{}]", items.join(" "))
        }
        Value::List(l) => {
            let mut out = String::from("[\n");
            for v in l {
                out.push_str(&format!("{}  {}\n", pad, to_adn_indented(v, indent + 1)));
            }
            out.push_str(&format!("{}]", pad));
            out
        }
        Value::Struct(m) if m.is_empty() => "{}".to_string(),
        Value::Struct(m) if m.len() <= 3 && !has_nested_map(m) => {
            let entries: Vec<String> = m
                .iter()
                .map(|(k, v)| format!(":{} {}", k, to_adn_indented(v, 0)))
                .collect();
            format!("{{{}}}", entries.join(", "))
        }
        Value::Struct(m) => {
            let mut out = String::from("{\n");
            for (k, v) in m {
                out.push_str(&format!(
                    "{}  :{} {}\n",
                    pad,
                    k,
                    to_adn_indented(v, indent + 1)
                ));
            }
            out.push_str(&format!("{}}}", pad));
            out
        }
        Value::PVec(pv) => {
            if pv.len() <= 5 {
                format!("{}", pv)
            } else {
                let mut out = String::from("#pvec[\n");
                for v in pv.iter() {
                    out.push_str(&format!("{}  {}\n", pad, to_adn_indented(v, indent + 1)));
                }
                out.push_str(&format!("{}]", pad));
                out
            }
        }
        Value::PMap(pm) => {
            if pm.len() <= 3 {
                format!("{}", pm)
            } else {
                let mut out = String::from("#pmap{\n");
                for (k, v) in pm.iter() {
                    out.push_str(&format!(
                        "{}  :{} {}\n",
                        pad,
                        k,
                        to_adn_indented(v, indent + 1)
                    ));
                }
                out.push_str(&format!("{}}}", pad));
                out
            }
        }
        other => format_value_adn(other),
    }
}

fn has_nested(l: &[Value]) -> bool {
    l.iter().any(|v| {
        matches!(
            v,
            Value::List(_) | Value::Struct(_) | Value::PVec(_) | Value::PMap(_)
        )
    })
}

fn has_nested_map(m: &HashMap<String, Value>) -> bool {
    m.values().any(|v| {
        matches!(
            v,
            Value::List(_) | Value::Struct(_) | Value::PVec(_) | Value::PMap(_)
        )
    })
}

// =============================================================================
// ADN Parsing (ADN String → Value)
// =============================================================================

/// Parse an ADN string into an Ark Value.
pub fn from_adn(input: &str) -> Result<Value, AdnError> {
    let mut parser = AdnParser::new(input);
    let value = parser.parse_value()?;
    parser.skip_whitespace();
    if parser.pos < parser.input.len() {
        return Err(AdnError::TrailingInput(parser.pos));
    }
    Ok(value)
}

#[derive(Debug)]
pub enum AdnError {
    UnexpectedEof,
    UnexpectedChar(char, usize),
    InvalidNumber(String),
    UnclosedString,
    UnclosedCollection(char),
    TrailingInput(usize),
    InvalidTaggedLiteral(String),
}

impl std::fmt::Display for AdnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdnError::UnexpectedEof => write!(f, "Unexpected end of input"),
            AdnError::UnexpectedChar(c, pos) => {
                write!(f, "Unexpected character '{}' at position {}", c, pos)
            }
            AdnError::InvalidNumber(s) => write!(f, "Invalid number: {}", s),
            AdnError::UnclosedString => write!(f, "Unclosed string literal"),
            AdnError::UnclosedCollection(c) => write!(f, "Unclosed collection, expected '{}'", c),
            AdnError::TrailingInput(pos) => write!(f, "Trailing input at position {}", pos),
            AdnError::InvalidTaggedLiteral(s) => write!(f, "Invalid tagged literal: {}", s),
        }
    }
}

impl std::error::Error for AdnError {}

struct AdnParser {
    input: Vec<char>,
    pos: usize,
}

impl AdnParser {
    fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.input.get(self.pos).copied();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() || c == ',' {
                self.advance();
            } else if c == ';' {
                // Skip line comments
                while let Some(c) = self.advance() {
                    if c == '\n' {
                        break;
                    }
                }
            } else {
                break;
            }
        }
    }

    fn parse_value(&mut self) -> Result<Value, AdnError> {
        self.skip_whitespace();
        match self.peek() {
            None => Err(AdnError::UnexpectedEof),
            Some('"') => self.parse_string(),
            Some('[') => self.parse_list(),
            Some('{') => self.parse_struct(),
            Some('#') => self.parse_tagged(),
            Some(':') => self.parse_keyword(),
            Some(c) if c == '-' || c.is_ascii_digit() => self.parse_number(),
            Some(c) if c.is_alphabetic() => self.parse_symbol(),
            Some(c) => Err(AdnError::UnexpectedChar(c, self.pos)),
        }
    }

    fn parse_string(&mut self) -> Result<Value, AdnError> {
        self.advance(); // consume opening "
        let mut s = String::new();
        loop {
            match self.advance() {
                None => return Err(AdnError::UnclosedString),
                Some('"') => return Ok(Value::String(s)),
                Some('\\') => match self.advance() {
                    Some('n') => s.push('\n'),
                    Some('t') => s.push('\t'),
                    Some('\\') => s.push('\\'),
                    Some('"') => s.push('"'),
                    Some(c) => {
                        s.push('\\');
                        s.push(c);
                    }
                    None => return Err(AdnError::UnclosedString),
                },
                Some(c) => s.push(c),
            }
        }
    }

    fn parse_number(&mut self) -> Result<Value, AdnError> {
        let mut s = String::new();
        if self.peek() == Some('-') {
            s.push('-');
            self.advance();
        }
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }
        // Float support: consume decimal point and fractional digits
        if self.peek() == Some('.') {
            s.push('.');
            self.advance();
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    s.push(c);
                    self.advance();
                } else {
                    break;
                }
            }
            // Parse as f64 and truncate to i64 (consistent with runtime Value::Integer)
            return s
                .parse::<f64>()
                .map(|f| Value::Integer(f as i64))
                .map_err(|_| AdnError::InvalidNumber(s));
        }
        s.parse::<i64>()
            .map(Value::Integer)
            .map_err(|_| AdnError::InvalidNumber(s))
    }

    fn parse_symbol(&mut self) -> Result<Value, AdnError> {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }
        match s.as_str() {
            "true" => Ok(Value::Boolean(true)),
            "false" => Ok(Value::Boolean(false)),
            "nil" => Ok(Value::Unit),
            _ => Ok(Value::String(s)), // Treat unknown symbols as strings
        }
    }

    fn parse_keyword(&mut self) -> Result<Value, AdnError> {
        self.advance(); // consume :
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }
        Ok(Value::String(s))
    }

    fn parse_list(&mut self) -> Result<Value, AdnError> {
        self.advance(); // consume [
        let mut items = Vec::new();
        loop {
            self.skip_whitespace();
            match self.peek() {
                None => return Err(AdnError::UnclosedCollection(']')),
                Some(']') => {
                    self.advance();
                    return Ok(Value::List(items));
                }
                _ => {
                    items.push(self.parse_value()?);
                }
            }
        }
    }

    fn parse_struct(&mut self) -> Result<Value, AdnError> {
        self.advance(); // consume {
        let mut map = HashMap::new();
        loop {
            self.skip_whitespace();
            match self.peek() {
                None => return Err(AdnError::UnclosedCollection('}')),
                Some('}') => {
                    self.advance();
                    return Ok(Value::Struct(map));
                }
                Some(':') => {
                    // Parse :key value pair
                    self.advance(); // consume :
                    let mut key = String::new();
                    while let Some(c) = self.peek() {
                        if c.is_alphanumeric() || c == '_' || c == '-' {
                            key.push(c);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    self.skip_whitespace();
                    let val = self.parse_value()?;
                    map.insert(key, val);
                }
                Some(c) => return Err(AdnError::UnexpectedChar(c, self.pos)),
            }
        }
    }

    fn parse_tagged(&mut self) -> Result<Value, AdnError> {
        self.advance(); // consume #
        let mut tag = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                tag.push(c);
                self.advance();
            } else {
                break;
            }
        }
        match tag.as_str() {
            "pvec" => {
                // Expect [...]
                self.skip_whitespace();
                match self.peek() {
                    Some('[') => {
                        self.advance();
                        let mut items = Vec::new();
                        loop {
                            self.skip_whitespace();
                            match self.peek() {
                                None => return Err(AdnError::UnclosedCollection(']')),
                                Some(']') => {
                                    self.advance();
                                    return Ok(Value::PVec(PVec::from_vec(items)));
                                }
                                _ => {
                                    items.push(self.parse_value()?);
                                }
                            }
                        }
                    }
                    _ => Err(AdnError::InvalidTaggedLiteral(
                        "Expected [ after #pvec".to_string(),
                    )),
                }
            }
            "pmap" => {
                // Expect {...}
                self.skip_whitespace();
                match self.peek() {
                    Some('{') => {
                        self.advance();
                        let mut entries = Vec::new();
                        loop {
                            self.skip_whitespace();
                            match self.peek() {
                                None => return Err(AdnError::UnclosedCollection('}')),
                                Some('}') => {
                                    self.advance();
                                    return Ok(Value::PMap(PMap::from_entries(entries)));
                                }
                                Some(':') => {
                                    self.advance();
                                    let mut key = String::new();
                                    while let Some(c) = self.peek() {
                                        if c.is_alphanumeric() || c == '_' || c == '-' {
                                            key.push(c);
                                            self.advance();
                                        } else {
                                            break;
                                        }
                                    }
                                    self.skip_whitespace();
                                    let val = self.parse_value()?;
                                    entries.push((key, val));
                                }
                                Some(c) => return Err(AdnError::UnexpectedChar(c, self.pos)),
                            }
                        }
                    }
                    _ => Err(AdnError::InvalidTaggedLiteral(
                        "Expected { after #pmap".to_string(),
                    )),
                }
            }
            _ => Err(AdnError::InvalidTaggedLiteral(tag)),
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_integer() {
        let v = Value::Integer(42);
        let adn = to_adn(&v);
        assert_eq!(adn, "42");
        let parsed = from_adn(&adn).unwrap();
        assert_eq!(parsed, v);
    }

    #[test]
    fn test_roundtrip_string() {
        let v = Value::String("hello world".to_string());
        let adn = to_adn(&v);
        assert_eq!(adn, "\"hello world\"");
        let parsed = from_adn(&adn).unwrap();
        assert_eq!(parsed, v);
    }

    #[test]
    fn test_roundtrip_boolean() {
        assert_eq!(from_adn("true").unwrap(), Value::Boolean(true));
        assert_eq!(from_adn("false").unwrap(), Value::Boolean(false));
    }

    #[test]
    fn test_roundtrip_nil() {
        assert_eq!(from_adn("nil").unwrap(), Value::Unit);
    }

    #[test]
    fn test_roundtrip_list() {
        let v = Value::List(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ]);
        let adn = to_adn(&v);
        assert_eq!(adn, "[1 2 3]");
        let parsed = from_adn(&adn).unwrap();
        assert_eq!(parsed, v);
    }

    #[test]
    fn test_roundtrip_pvec() {
        let v = Value::PVec(PVec::from_vec(vec![Value::Integer(10), Value::Integer(20)]));
        let adn = to_adn(&v);
        assert!(adn.starts_with("#pvec["));
        let parsed = from_adn(&adn).unwrap();
        assert_eq!(parsed, v);
    }

    #[test]
    fn test_roundtrip_pmap() {
        let v = Value::PMap(PMap::from_entries(vec![(
            "name".to_string(),
            Value::String("Ark".to_string()),
        )]));
        let adn = to_adn(&v);
        assert!(adn.contains("#pmap{"));
        let parsed = from_adn(&adn).unwrap();
        assert_eq!(parsed, v);
    }

    #[test]
    fn test_parse_struct() {
        let result = from_adn("{:a 1, :b 2}").unwrap();
        if let Value::Struct(m) = result {
            assert_eq!(m.get("a"), Some(&Value::Integer(1)));
            assert_eq!(m.get("b"), Some(&Value::Integer(2)));
        } else {
            panic!("Expected Struct");
        }
    }

    #[test]
    fn test_nested_structures() {
        let adn = "[1 [2 3] [4 5]]";
        let parsed = from_adn(adn).unwrap();
        if let Value::List(l) = &parsed {
            assert_eq!(l.len(), 3);
            assert_eq!(l[0], Value::Integer(1));
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn test_negative_number() {
        assert_eq!(from_adn("-42").unwrap(), Value::Integer(-42));
    }

    #[test]
    fn test_pretty_print() {
        let v = Value::List(vec![
            Value::Integer(1),
            Value::List(vec![Value::Integer(2), Value::Integer(3)]),
        ]);
        let pretty = to_adn_pretty(&v);
        assert!(pretty.contains('\n'));
    }

    #[test]
    fn test_comments_ignored() {
        let result = from_adn("; this is a comment\n42").unwrap();
        assert_eq!(result, Value::Integer(42));
    }

    #[test]
    fn test_error_unclosed_string() {
        assert!(from_adn("\"hello").is_err());
    }

    #[test]
    fn test_error_unclosed_list() {
        assert!(from_adn("[1 2 3").is_err());
    }
}
