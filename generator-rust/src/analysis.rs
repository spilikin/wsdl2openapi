use std::collections::{HashMap, HashSet};

use indexmap::{IndexMap, IndexSet};

use crate::ir::{FieldType, ItemKind, Occurs, Operation, Program, XmlNode};
use crate::naming::unraw;

pub const SOAP_ENV_PREFIX: &str = "SOAP-ENV";
pub const XML_NAMESPACE: &str = "http://www.w3.org/XML/1998/namespace";

/// One prefix per namespace URI, used for every element and attribute
/// written by the generated serializers. quick-xml's serde layer is not
/// namespace-aware, so names are serialized pre-prefixed and the declarations
/// are written once on the SOAP envelope; parsing matches by local name and
/// accepts whatever prefixes the peer chose.
#[derive(Debug, Clone, Default)]
pub struct Namespaces {
    prefixes: IndexMap<String, String>,
}

impl Namespaces {
    /// Prefixes are derived from the Rust module that declares a namespace's
    /// types (`signatureservice74`, `xmldsig`), so wire logs stay readable and
    /// prefixes do not shift when unrelated schemas are added.
    pub fn build(program: &Program) -> Self {
        let mut candidates: IndexMap<String, String> = IndexMap::new();
        for item in program.items.values() {
            if let Some(ns) = &item.namespace {
                let segment = item
                    .module
                    .last()
                    .map(|s| unraw(s).trim_matches('_'))
                    .unwrap_or_default();
                candidates
                    .entry(ns.clone())
                    .or_insert_with(|| segment.to_owned());
            }
        }
        for ns in used_namespaces(program) {
            candidates.entry(ns).or_default();
        }

        let mut taken: HashSet<String> = [SOAP_ENV_PREFIX, "xml", "xmlns"].map(String::from).into();
        let mut prefixes = IndexMap::new();
        for (ns, candidate) in candidates {
            if ns == XML_NAMESPACE {
                continue;
            }
            let prefix = pick_prefix(&mut taken, &candidate);
            prefixes.insert(ns, prefix);
        }
        Self { prefixes }
    }

    /// The prefix for `ns`; `xml` for the XML namespace, which is predeclared.
    pub fn prefix(&self, ns: &str) -> &str {
        if ns == XML_NAMESPACE {
            return "xml";
        }
        self.prefixes
            .get(ns)
            .map(String::as_str)
            .unwrap_or_else(|| panic!("namespace {ns} was not collected"))
    }

    /// `(prefix, uri)` declarations an operation's messages need, ordered by prefix.
    pub fn declarations(&self, program: &Program, op: &Operation) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = operation_namespaces(program, op)
            .into_iter()
            .filter(|ns| ns != XML_NAMESPACE)
            .map(|ns| (self.prefix(&ns).to_owned(), ns))
            .collect();
        out.sort();
        out
    }
}

fn pick_prefix(taken: &mut HashSet<String>, candidate: &str) -> String {
    let base = match candidate {
        "" => "ns".to_owned(),
        // Names starting with "xml" are reserved by the Namespaces spec.
        c if c.to_ascii_lowercase().starts_with("xml") => format!("ns-{c}"),
        c if c.starts_with(|ch: char| ch.is_ascii_digit()) => format!("ns{c}"),
        c => c.to_owned(),
    };
    let prefix = if base == "ns" {
        (1..).map(|n| format!("ns{n}")).find(|p| !taken.contains(p))
    } else if taken.contains(&base) {
        (2..)
            .map(|n| format!("{base}_{n}"))
            .find(|p| !taken.contains(p))
    } else {
        Some(base)
    }
    .expect("an unused prefix exists");
    taken.insert(prefix.clone());
    prefix
}

fn used_namespaces(program: &Program) -> IndexSet<String> {
    let mut out = IndexSet::new();
    for item in program.items.values() {
        if let ItemKind::Struct(fields) = &item.kind {
            for field in fields {
                if let XmlNode::Element(q) | XmlNode::Attribute(q) = &field.node {
                    out.extend(q.namespace.clone());
                }
            }
        }
    }
    for port in &program.ports {
        for op in &port.operations {
            for payload in [&op.input, &op.output].into_iter().chain(&op.faults) {
                out.extend(payload.element.namespace.clone());
            }
        }
    }
    out
}

/// Every namespace an element or attribute reachable from the operation's
/// input, output or fault payloads can be written in.
fn operation_namespaces(program: &Program, op: &Operation) -> IndexSet<String> {
    let mut out = IndexSet::new();
    let mut visited = HashSet::new();
    let mut stack = Vec::new();
    for payload in [&op.input, &op.output].into_iter().chain(&op.faults) {
        out.extend(payload.element.namespace.clone());
        stack.push(payload.key.as_str());
    }
    while let Some(key) = stack.pop() {
        let (key, _) = program.resolve(key);
        if !visited.insert(key) {
            continue;
        }
        for field in program.fields_of(key) {
            if let XmlNode::Element(q) | XmlNode::Attribute(q) = &field.node {
                out.extend(q.namespace.clone());
            }
            if let FieldType::Named(target) = &field.ty {
                stack.push(target);
            }
        }
    }
    out
}

/// Boxes every non-`Vec` field whose type lies in the same strongly connected
/// component as its owner, which makes all recursive types sized. Minimising
/// the boxed set is NP-hard; boxing every intra-component edge is simple and
/// deterministic.
pub fn box_recursive_fields(program: &mut Program) {
    let keys: Vec<&str> = program
        .items
        .iter()
        .filter(|(_, item)| matches!(item.kind, ItemKind::Struct(_)))
        .map(|(key, _)| key.as_str())
        .collect();
    let index: HashMap<&str, usize> = keys.iter().enumerate().map(|(i, k)| (*k, i)).collect();
    let edges: Vec<Vec<usize>> = keys
        .iter()
        .map(|key| {
            program
                .fields_of(key)
                .iter()
                .filter(|f| f.occurs != Occurs::Many)
                .filter_map(|f| match &f.ty {
                    FieldType::Named(target) => index.get(program.resolve(target).0).copied(),
                    _ => None,
                })
                .collect()
        })
        .collect();
    let component = tarjan(&edges);

    let mut boxed: Vec<(String, usize)> = Vec::new();
    for (i, key) in keys.iter().enumerate() {
        for (f, field) in program.fields_of(key).iter().enumerate() {
            if field.occurs == Occurs::Many {
                continue;
            }
            if let FieldType::Named(target) = &field.ty
                && let Some(&j) = index.get(program.resolve(target).0)
                && component[i] == component[j]
            {
                boxed.push(((*key).to_owned(), f));
            }
        }
    }
    for (key, f) in boxed {
        if let ItemKind::Struct(fields) = &mut program.items[&key].kind {
            fields[f].boxed = true;
        }
    }
}

/// Tarjan's strongly connected components; returns a component id per node.
fn tarjan(edges: &[Vec<usize>]) -> Vec<usize> {
    struct State<'a> {
        edges: &'a [Vec<usize>],
        index: Vec<Option<usize>>,
        low: Vec<usize>,
        on_stack: Vec<bool>,
        stack: Vec<usize>,
        component: Vec<usize>,
        next_index: usize,
        next_component: usize,
    }

    // Recursive; schema graphs are shallow enough (hundreds of nodes) for the call stack.
    fn visit(s: &mut State, v: usize) {
        s.index[v] = Some(s.next_index);
        s.low[v] = s.next_index;
        s.next_index += 1;
        s.stack.push(v);
        s.on_stack[v] = true;
        for &w in &s.edges[v] {
            match s.index[w] {
                None => {
                    visit(s, w);
                    s.low[v] = s.low[v].min(s.low[w]);
                }
                Some(iw) if s.on_stack[w] => s.low[v] = s.low[v].min(iw),
                Some(_) => {}
            }
        }
        if Some(s.low[v]) == s.index[v] {
            loop {
                let w = s.stack.pop().expect("v is on the stack");
                s.on_stack[w] = false;
                s.component[w] = s.next_component;
                if w == v {
                    break;
                }
            }
            s.next_component += 1;
        }
    }

    let n = edges.len();
    let mut state = State {
        edges,
        index: vec![None; n],
        low: vec![0; n],
        on_stack: vec![false; n],
        stack: Vec::new(),
        component: vec![0; n],
        next_index: 0,
        next_component: 0,
    };
    for v in 0..n {
        if state.index[v].is_none() {
            visit(&mut state, v);
        }
    }
    state.component
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tarjan_groups_cycles() {
        // 0 -> 1 -> 2 -> 0, 3 -> 3, 4 alone
        let c = tarjan(&[vec![1], vec![2], vec![0], vec![3], vec![]]);
        assert_eq!(c[0], c[1]);
        assert_eq!(c[1], c[2]);
        assert_ne!(c[0], c[3]);
        assert_ne!(c[3], c[4]);
    }

    #[test]
    fn prefixes_are_unique_and_valid() {
        let mut taken: HashSet<String> = [SOAP_ENV_PREFIX.to_owned()].into();
        assert_eq!(pick_prefix(&mut taken, "error20"), "error20");
        assert_eq!(pick_prefix(&mut taken, "error20"), "error20_2");
        assert_eq!(pick_prefix(&mut taken, "xmldsig"), "ns-xmldsig");
        assert_eq!(pick_prefix(&mut taken, ""), "ns1");
        assert_eq!(pick_prefix(&mut taken, ""), "ns2");
    }
}
