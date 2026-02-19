/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * Persistent Data Structures for the Ark Language.
 * Immutable-by-default collections with structural sharing.
 *
 * Inspired by Clojure's persistent vectors and maps, implemented
 * in Rust with Arc-based structural sharing for zero-copy immutability.
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 */

use crate::runtime::Value;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

// =============================================================================
// PVec — Persistent Vector (32-way Branching Trie)
// =============================================================================
//
// A persistent (immutable) vector with O(1) amortized append and O(log32 n)
// lookup/update. Modeled after Clojure's PersistentVector:
//
//   - A 32-way branching trie stores elements in leaf arrays of up to 32
//   - A separate "tail" buffer collects recent appends for O(1) amortized conj
//   - On conj, when the tail is full (32 elements), it is pushed into the trie
//   - Structural sharing via Arc: mutations copy only the root-to-leaf path
//
// Branching factor 32 ⇒ depth ≤ 7 for 32^7 ≈ 34 billion elements.
// Practical depth for most programs: 1–3 levels.

const BITS: usize = 5;
const WIDTH: usize = 1 << BITS; // 32
const MASK: usize = WIDTH - 1; // 0x1F

/// Internal trie node: either a branch (children are nodes) or a leaf (children are values).
#[derive(Clone)]
enum TrieNode {
    Branch(Vec<Arc<TrieNode>>),
    Leaf(Vec<Value>),
}

impl TrieNode {
    fn empty_branch() -> Self {
        TrieNode::Branch(Vec::new())
    }
}

/// A persistent (immutable) vector with structural sharing via a 32-way trie.
#[derive(Clone)]
pub struct PVec {
    /// Number of elements in the trie (excludes tail)
    trie_len: usize,
    /// Depth/shift of the trie (0 = leaf level, 5 = one level, 10 = two levels, etc.)
    shift: usize,
    /// Root of the trie (stores elements 0..trie_len)
    root: Arc<TrieNode>,
    /// Tail buffer for recent appends (up to 32 elements)
    tail: Arc<Vec<Value>>,
    /// Total number of elements (trie_len + tail.len())
    len: usize,
}

impl fmt::Debug for PVec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PVec[")?;
        for (i, v) in self.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{:?}", v)?;
        }
        write!(f, "]")
    }
}

impl PartialEq for PVec {
    fn eq(&self, other: &Self) -> bool {
        if self.len != other.len {
            return false;
        }
        // Fast path: identical trie root + tail = definitely equal
        if Arc::ptr_eq(&self.root, &other.root) && Arc::ptr_eq(&self.tail, &other.tail) {
            return true;
        }
        // Slow path: element-wise
        self.iter().zip(other.iter()).all(|(a, b)| a == b)
    }
}

impl PVec {
    /// Create an empty persistent vector.
    pub fn new() -> Self {
        Self {
            trie_len: 0,
            shift: BITS,
            root: Arc::new(TrieNode::empty_branch()),
            tail: Arc::new(Vec::new()),
            len: 0,
        }
    }

    /// Create a persistent vector from existing values.
    pub fn from_vec(v: Vec<Value>) -> Self {
        let mut pv = Self::new();
        for val in v {
            pv = pv.conj(val);
        }
        pv
    }

    /// Number of elements.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the vector is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Index of the first element in the tail.
    fn tail_offset(&self) -> usize {
        if self.len < WIDTH {
            0
        } else {
            ((self.len - 1) >> BITS) << BITS
        }
    }

    /// Find the leaf array containing `index`.
    fn array_for(&self, index: usize) -> &[Value] {
        if index >= self.tail_offset() {
            // Element is in the tail
            &self.tail
        } else {
            // Walk the trie
            let mut node = &*self.root;
            let mut level = self.shift;
            loop {
                match node {
                    TrieNode::Branch(children) => {
                        let child_idx = (index >> level) & MASK;
                        node = &children[child_idx];
                        if level == BITS {
                            // Next level is leaf
                            break;
                        }
                        level -= BITS;
                    }
                    TrieNode::Leaf(values) => return values.as_slice(),
                }
            }
            match node {
                TrieNode::Leaf(values) => values.as_slice(),
                _ => unreachable!("Expected leaf at bottom of trie"),
            }
        }
    }

    /// Get element at index. Returns None if out of bounds.
    pub fn get(&self, index: usize) -> Option<&Value> {
        if index >= self.len {
            return None;
        }
        let arr = self.array_for(index);
        Some(&arr[index & MASK])
    }

    /// Return a NEW persistent vector with the element appended.
    pub fn conj(&self, value: Value) -> Self {
        // Case 1: Room in tail
        if self.len - self.trie_len < WIDTH {
            let mut new_tail = (*self.tail).clone();
            new_tail.push(value);
            return Self {
                trie_len: self.trie_len,
                shift: self.shift,
                root: Arc::clone(&self.root),
                tail: Arc::new(new_tail),
                len: self.len + 1,
            };
        }
        // Case 2: Tail is full — push it into the trie
        let tail_node = Arc::new(TrieNode::Leaf((*self.tail).clone()));
        let (new_root, new_shift) = if (self.trie_len >> BITS) > (1 << self.shift) {
            // Need to grow the trie depth
            let new_root = TrieNode::Branch(vec![
                Arc::clone(&self.root),
                Self::new_path(self.shift, tail_node),
            ]);
            (Arc::new(new_root), self.shift + BITS)
        } else {
            let new_root = Self::push_tail(self.shift, &self.root, tail_node, self.trie_len);
            (Arc::new(new_root), self.shift)
        };

        Self {
            trie_len: self.trie_len + WIDTH,
            shift: new_shift,
            root: new_root,
            tail: Arc::new(vec![value]),
            len: self.len + 1,
        }
    }

    /// Recursively push a full tail node into the trie.
    fn push_tail(
        level: usize,
        parent: &TrieNode,
        tail_node: Arc<TrieNode>,
        trie_len: usize,
    ) -> TrieNode {
        let sub_idx = ((trie_len - 1) >> level) & MASK;
        match parent {
            TrieNode::Branch(children) => {
                let mut new_children = children.clone();
                if level == BITS {
                    // Insert at leaf level
                    if sub_idx < new_children.len() {
                        new_children[sub_idx] = tail_node;
                    } else {
                        new_children.push(tail_node);
                    }
                } else {
                    // Recurse deeper
                    if sub_idx < new_children.len() {
                        let child = Self::push_tail(
                            level - BITS,
                            &new_children[sub_idx],
                            tail_node,
                            trie_len,
                        );
                        new_children[sub_idx] = Arc::new(child);
                    } else {
                        new_children.push(Self::new_path(level - BITS, tail_node));
                    }
                }
                TrieNode::Branch(new_children)
            }
            _ => unreachable!("push_tail: expected Branch node"),
        }
    }

    /// Create a new path from root to the tail node at the given depth.
    fn new_path(level: usize, node: Arc<TrieNode>) -> Arc<TrieNode> {
        if level == 0 {
            node
        } else {
            Arc::new(TrieNode::Branch(vec![Self::new_path(level - BITS, node)]))
        }
    }

    /// Return a NEW persistent vector with the element at `index` replaced.
    pub fn assoc(&self, index: usize, value: Value) -> Option<Self> {
        if index >= self.len {
            return None;
        }
        if index >= self.tail_offset() {
            // Update in tail
            let mut new_tail = (*self.tail).clone();
            new_tail[index & MASK] = value;
            Some(Self {
                trie_len: self.trie_len,
                shift: self.shift,
                root: Arc::clone(&self.root),
                tail: Arc::new(new_tail),
                len: self.len,
            })
        } else {
            // Update in trie — path copy
            let new_root = Self::do_assoc(self.shift, &self.root, index, value);
            Some(Self {
                trie_len: self.trie_len,
                shift: self.shift,
                root: Arc::new(new_root),
                tail: Arc::clone(&self.tail),
                len: self.len,
            })
        }
    }

    /// Recursively set a value in the trie, producing a new path.
    fn do_assoc(level: usize, node: &TrieNode, index: usize, value: Value) -> TrieNode {
        if level == 0 {
            match node {
                TrieNode::Leaf(values) => {
                    let mut new_values = values.clone();
                    new_values[index & MASK] = value;
                    TrieNode::Leaf(new_values)
                }
                _ => unreachable!("Expected leaf at level 0"),
            }
        } else {
            match node {
                TrieNode::Branch(children) => {
                    let sub_idx = (index >> level) & MASK;
                    let mut new_children = children.clone();
                    let child = Self::do_assoc(level - BITS, &children[sub_idx], index, value);
                    new_children[sub_idx] = Arc::new(child);
                    TrieNode::Branch(new_children)
                }
                _ => unreachable!("Expected branch at level > 0"),
            }
        }
    }

    /// Return a NEW persistent vector without the last element.
    pub fn pop(&self) -> Option<(Self, Value)> {
        if self.is_empty() {
            return None;
        }
        let last = self.get(self.len - 1).unwrap().clone();

        if self.tail.len() > 1 {
            // Just shrink the tail
            let mut new_tail = (*self.tail).clone();
            new_tail.pop();
            Some((
                Self {
                    trie_len: self.trie_len,
                    shift: self.shift,
                    root: Arc::clone(&self.root),
                    tail: Arc::new(new_tail),
                    len: self.len - 1,
                },
                last,
            ))
        } else if self.trie_len == 0 {
            // Popping the only element
            Some((Self::new(), last))
        } else {
            // Tail becomes the rightmost leaf of the trie; pop that leaf out
            let new_tail = self.array_for(self.trie_len - 1).to_vec();
            let (new_root, new_shift) = self.pop_tail();
            Some((
                Self {
                    trie_len: self.trie_len - WIDTH,
                    shift: new_shift,
                    root: new_root,
                    tail: Arc::new(new_tail),
                    len: self.len - 1,
                },
                last,
            ))
        }
    }

    /// Remove the rightmost leaf from the trie, returning (new_root, new_shift).
    fn pop_tail(&self) -> (Arc<TrieNode>, usize) {
        let new_root = Self::do_pop_tail(self.shift, &self.root, self.trie_len);
        match new_root {
            Some(root) => {
                // Check if we can reduce depth
                if self.shift > BITS {
                    if let TrieNode::Branch(ref children) = *root {
                        if children.len() == 1 {
                            return (Arc::clone(&children[0]), self.shift - BITS);
                        }
                    }
                }
                (root, self.shift)
            }
            None => (Arc::new(TrieNode::empty_branch()), self.shift),
        }
    }

    fn do_pop_tail(level: usize, node: &TrieNode, trie_len: usize) -> Option<Arc<TrieNode>> {
        let sub_idx = ((trie_len - 1) >> level) & MASK;
        if level > BITS {
            match node {
                TrieNode::Branch(children) => {
                    let new_child = Self::do_pop_tail(level - BITS, &children[sub_idx], trie_len);
                    if new_child.is_none() && sub_idx == 0 {
                        None
                    } else {
                        let mut new_children = children.clone();
                        match new_child {
                            Some(child) => new_children[sub_idx] = child,
                            None => {
                                new_children.pop();
                            }
                        }
                        Some(Arc::new(TrieNode::Branch(new_children)))
                    }
                }
                _ => None,
            }
        } else {
            // At the leaf level
            if sub_idx == 0 {
                None
            } else {
                match node {
                    TrieNode::Branch(children) => {
                        let mut new_children = children.clone();
                        new_children.pop();
                        Some(Arc::new(TrieNode::Branch(new_children)))
                    }
                    _ => None,
                }
            }
        }
    }

    /// Return a NEW persistent vector with elements from another PVec appended.
    pub fn concat(&self, other: &PVec) -> Self {
        let mut result = self.clone();
        for v in other.iter() {
            result = result.conj(v.clone());
        }
        result
    }

    /// Iterate over elements.
    pub fn iter(&self) -> PVecIter<'_> {
        PVecIter {
            vec: self,
            index: 0,
        }
    }

    /// Convert to a Vec (snapshot).
    pub fn to_vec(&self) -> Vec<Value> {
        self.iter().cloned().collect()
    }

    /// Map a function over elements, producing a new PVec.
    pub fn map<F>(&self, f: F) -> Self
    where
        F: Fn(&Value) -> Value,
    {
        let new_data: Vec<Value> = self.iter().map(f).collect();
        Self::from_vec(new_data)
    }

    /// Filter elements, producing a new PVec.
    pub fn filter<F>(&self, f: F) -> Self
    where
        F: Fn(&Value) -> bool,
    {
        let new_data: Vec<Value> = self.iter().filter(|v| f(v)).cloned().collect();
        Self::from_vec(new_data)
    }

    /// Return a subvector (slice) as a new PVec.
    pub fn subvec(&self, start: usize, end: usize) -> Option<Self> {
        if start > end || end > self.len {
            return None;
        }
        let new_data: Vec<Value> = (start..end).map(|i| self.get(i).unwrap().clone()).collect();
        Some(Self::from_vec(new_data))
    }
}

/// Iterator for PVec.
pub struct PVecIter<'a> {
    vec: &'a PVec,
    index: usize,
}

impl<'a> Iterator for PVecIter<'a> {
    type Item = &'a Value;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.vec.len {
            let val = self.vec.get(self.index);
            self.index += 1;
            val
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.vec.len - self.index;
        (remaining, Some(remaining))
    }
}

impl Default for PVec {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// PMap — Persistent Map
// =============================================================================
//
// A persistent (immutable) association map. Uses BTreeMap internally for
// deterministic ordering (important for content addressing / MAST hashing).
// Structural sharing is achieved via Arc: "modifications" clone and produce
// a new PMap while the original remains intact.

/// A persistent (immutable) map with structural sharing.
#[derive(Clone)]
pub struct PMap {
    data: Arc<BTreeMap<String, Value>>,
}

impl fmt::Debug for PMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PMap{{")?;
        for (i, (k, v)) in self.data.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}: {:?}", k, v)?;
        }
        write!(f, "}}")
    }
}

impl PartialEq for PMap {
    fn eq(&self, other: &Self) -> bool {
        // Fast path: same Arc
        if Arc::ptr_eq(&self.data, &other.data) {
            return true;
        }
        // Slow path: entry-wise comparison
        self.data == other.data
    }
}

impl PMap {
    /// Create an empty persistent map.
    pub fn new() -> Self {
        Self {
            data: Arc::new(BTreeMap::new()),
        }
    }

    /// Create a PMap from a list of key-value pairs.
    pub fn from_entries(entries: Vec<(String, Value)>) -> Self {
        let mut map = BTreeMap::new();
        for (k, v) in entries {
            map.insert(k, v);
        }
        Self {
            data: Arc::new(map),
        }
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Whether the map is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get value by key.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.data.get(key)
    }

    /// Check if key exists.
    pub fn contains_key(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    /// Return a NEW persistent map with the key-value pair added/updated.
    /// The original map is unchanged.
    pub fn assoc(&self, key: String, value: Value) -> Self {
        let mut new_data = (*self.data).clone();
        new_data.insert(key, value);
        Self {
            data: Arc::new(new_data),
        }
    }

    /// Return a NEW persistent map without the specified key.
    pub fn dissoc(&self, key: &str) -> Self {
        let mut new_data = (*self.data).clone();
        new_data.remove(key);
        Self {
            data: Arc::new(new_data),
        }
    }

    /// Merge another PMap into this one, producing a new PMap.
    /// Keys from `other` take precedence on conflict.
    pub fn merge(&self, other: &PMap) -> Self {
        let mut new_data = (*self.data).clone();
        for (k, v) in other.data.iter() {
            new_data.insert(k.clone(), v.clone());
        }
        Self {
            data: Arc::new(new_data),
        }
    }

    /// Get all keys as a vector.
    pub fn keys(&self) -> Vec<String> {
        self.data.keys().cloned().collect()
    }

    /// Get all values as a vector.
    pub fn values(&self) -> Vec<Value> {
        self.data.values().cloned().collect()
    }

    /// Get all entries as (key, value) pairs.
    pub fn entries(&self) -> Vec<(String, Value)> {
        self.data
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Iterate over key-value pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Value)> {
        self.data.iter()
    }

    /// Select specific keys into a new PMap.
    pub fn select_keys(&self, keys: &[String]) -> Self {
        let mut new_data = BTreeMap::new();
        for key in keys {
            if let Some(v) = self.data.get(key) {
                new_data.insert(key.clone(), v.clone());
            }
        }
        Self {
            data: Arc::new(new_data),
        }
    }
}

impl Default for PMap {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Display implementations (for Ark REPL/debugger)
// =============================================================================

impl fmt::Display for PVec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#pvec[")?;
        for (i, v) in self.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "{}", format_value_adn(v))?;
        }
        write!(f, "]")
    }
}

impl fmt::Display for PMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#pmap{{")?;
        for (i, (k, v)) in self.data.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, ":{} {}", k, format_value_adn(v))?;
        }
        write!(f, "}}")
    }
}

/// Format a Value in ADN (Ark Data Notation) syntax.
pub fn format_value_adn(v: &Value) -> String {
    match v {
        Value::Integer(i) => i.to_string(),
        Value::String(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
        Value::Boolean(b) => b.to_string(),
        Value::Unit => "nil".to_string(),
        Value::List(l) => {
            let items: Vec<String> = l.iter().map(format_value_adn).collect();
            format!("[{}]", items.join(" "))
        }
        Value::Struct(m) => {
            let entries: Vec<String> = m
                .iter()
                .map(|(k, v)| format!(":{} {}", k, format_value_adn(v)))
                .collect();
            format!("{{{}}}", entries.join(", "))
        }
        Value::PVec(pv) => format!("{}", pv),
        Value::PMap(pm) => format!("{}", pm),
        Value::Function(_) => "#<fn>".to_string(),
        Value::NativeFunction(_) => "#<native-fn>".to_string(),
        Value::Buffer(b) => format!("#buf[{} bytes]", b.len()),
        Value::LinearObject { typename, id, .. } => format!("#<linear:{} {}>", typename, id),
        Value::Return(v) => format_value_adn(v),
        Value::EnumValue {
            enum_name,
            variant,
            fields,
        } => {
            if fields.is_empty() {
                format!("{}::{}", enum_name, variant)
            } else {
                let field_strs: Vec<String> = fields.iter().map(format_value_adn).collect();
                format!("{}::{}({})", enum_name, variant, field_strs.join(", "))
            }
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- PVec Tests ---

    #[test]
    fn test_pvec_empty() {
        let v = PVec::new();
        assert_eq!(v.len(), 0);
        assert!(v.is_empty());
    }

    #[test]
    fn test_pvec_conj_immutability() {
        let v1 = PVec::new();
        let v2 = v1.conj(Value::Integer(1));
        let v3 = v2.conj(Value::Integer(2));

        // Original unchanged
        assert_eq!(v1.len(), 0);
        assert_eq!(v2.len(), 1);
        assert_eq!(v3.len(), 2);

        // Values correct
        assert_eq!(v2.get(0), Some(&Value::Integer(1)));
        assert_eq!(v3.get(0), Some(&Value::Integer(1)));
        assert_eq!(v3.get(1), Some(&Value::Integer(2)));
    }

    #[test]
    fn test_pvec_assoc() {
        let v1 = PVec::from_vec(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ]);
        let v2 = v1.assoc(1, Value::Integer(99)).unwrap();

        // Original unchanged
        assert_eq!(v1.get(1), Some(&Value::Integer(2)));
        // New version updated
        assert_eq!(v2.get(1), Some(&Value::Integer(99)));
    }

    #[test]
    fn test_pvec_pop() {
        let v1 = PVec::from_vec(vec![Value::Integer(1), Value::Integer(2)]);
        let (v2, last) = v1.pop().unwrap();

        assert_eq!(last, Value::Integer(2));
        assert_eq!(v2.len(), 1);
        assert_eq!(v1.len(), 2); // original unchanged
    }

    #[test]
    fn test_pvec_concat() {
        let v1 = PVec::from_vec(vec![Value::Integer(1), Value::Integer(2)]);
        let v2 = PVec::from_vec(vec![Value::Integer(3), Value::Integer(4)]);
        let v3 = v1.concat(&v2);

        assert_eq!(v3.len(), 4);
        assert_eq!(v1.len(), 2); // unchanged
        assert_eq!(v2.len(), 2); // unchanged
    }

    #[test]
    fn test_pvec_subvec() {
        let v1 = PVec::from_vec(vec![
            Value::Integer(0),
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
            Value::Integer(4),
        ]);
        let sub = v1.subvec(1, 4).unwrap();
        assert_eq!(sub.len(), 3);
        assert_eq!(sub.get(0), Some(&Value::Integer(1)));
        assert_eq!(sub.get(2), Some(&Value::Integer(3)));
    }

    #[test]
    fn test_pvec_equality() {
        let v1 = PVec::from_vec(vec![Value::Integer(1), Value::Integer(2)]);
        let v2 = PVec::from_vec(vec![Value::Integer(1), Value::Integer(2)]);
        let v3 = PVec::from_vec(vec![Value::Integer(1), Value::Integer(3)]);

        assert_eq!(v1, v2);
        assert_ne!(v1, v3);
    }

    // --- PMap Tests ---

    #[test]
    fn test_pmap_empty() {
        let m = PMap::new();
        assert_eq!(m.len(), 0);
        assert!(m.is_empty());
    }

    #[test]
    fn test_pmap_assoc_immutability() {
        let m1 = PMap::new();
        let m2 = m1.assoc("a".to_string(), Value::Integer(1));
        let m3 = m2.assoc("b".to_string(), Value::Integer(2));

        // m1 unchanged
        assert_eq!(m1.len(), 0);
        assert_eq!(m2.len(), 1);
        assert_eq!(m3.len(), 2);

        assert_eq!(m2.get("a"), Some(&Value::Integer(1)));
        assert_eq!(m3.get("a"), Some(&Value::Integer(1)));
        assert_eq!(m3.get("b"), Some(&Value::Integer(2)));
    }

    #[test]
    fn test_pmap_dissoc() {
        let m1 = PMap::from_entries(vec![
            ("a".to_string(), Value::Integer(1)),
            ("b".to_string(), Value::Integer(2)),
        ]);
        let m2 = m1.dissoc("a");

        assert_eq!(m1.len(), 2); // unchanged
        assert_eq!(m2.len(), 1);
        assert_eq!(m2.get("a"), None);
        assert_eq!(m2.get("b"), Some(&Value::Integer(2)));
    }

    #[test]
    fn test_pmap_merge() {
        let m1 = PMap::from_entries(vec![
            ("a".to_string(), Value::Integer(1)),
            ("b".to_string(), Value::Integer(2)),
        ]);
        let m2 = PMap::from_entries(vec![
            ("b".to_string(), Value::Integer(99)),
            ("c".to_string(), Value::Integer(3)),
        ]);
        let m3 = m1.merge(&m2);

        assert_eq!(m3.len(), 3);
        assert_eq!(m3.get("a"), Some(&Value::Integer(1)));
        assert_eq!(m3.get("b"), Some(&Value::Integer(99))); // m2 wins
        assert_eq!(m3.get("c"), Some(&Value::Integer(3)));
    }

    #[test]
    fn test_pmap_keys_values() {
        let m = PMap::from_entries(vec![
            ("x".to_string(), Value::Integer(10)),
            ("y".to_string(), Value::Integer(20)),
        ]);
        let keys = m.keys();
        assert!(keys.contains(&"x".to_string()));
        assert!(keys.contains(&"y".to_string()));
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn test_pmap_equality() {
        let m1 = PMap::from_entries(vec![
            ("a".to_string(), Value::Integer(1)),
            ("b".to_string(), Value::Integer(2)),
        ]);
        let m2 = PMap::from_entries(vec![
            ("b".to_string(), Value::Integer(2)),
            ("a".to_string(), Value::Integer(1)),
        ]);
        assert_eq!(m1, m2); // order-independent equality
    }

    #[test]
    fn test_pmap_select_keys() {
        let m1 = PMap::from_entries(vec![
            ("a".to_string(), Value::Integer(1)),
            ("b".to_string(), Value::Integer(2)),
            ("c".to_string(), Value::Integer(3)),
        ]);
        let m2 = m1.select_keys(&["a".to_string(), "c".to_string()]);
        assert_eq!(m2.len(), 2);
        assert_eq!(m2.get("b"), None);
    }

    // --- ADN Format Test ---

    #[test]
    fn test_adn_format_pvec() {
        let v = PVec::from_vec(vec![Value::Integer(1), Value::String("hello".to_string())]);
        let s = format!("{}", v);
        assert_eq!(s, "#pvec[1 \"hello\"]");
    }

    #[test]
    fn test_adn_format_pmap() {
        let m = PMap::from_entries(vec![("name".to_string(), Value::String("Ark".to_string()))]);
        let s = format!("{}", m);
        assert!(s.contains(":name"));
        assert!(s.contains("\"Ark\""));
    }

    #[test]
    fn test_format_value_adn() {
        assert_eq!(format_value_adn(&Value::Integer(42)), "42");
        assert_eq!(format_value_adn(&Value::Boolean(true)), "true");
        assert_eq!(format_value_adn(&Value::Unit), "nil");
        assert_eq!(format_value_adn(&Value::String("hi".to_string())), "\"hi\"");
    }
}
