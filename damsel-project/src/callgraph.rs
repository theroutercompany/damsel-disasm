//! Call-graph queries over stored facts (plan feature 10, slice 1):
//! the semantic query engine's first surface — source→sink reachability
//! over call edges, with path reconstruction.
//!
//! Graph derivation is fact-driven and format-neutral: `call|to=function:…`
//! xref facts on `instruction:0x…` objects become edges from the function
//! whose *bound contains* the instruction to the target. Call targets
//! without recovered bounds still appear as nodes (unnamed) so paths
//! through thunks/stubs survive. Paths are BFS-shortest; results carry
//! provenance by construction — every edge is a stored fact.

use crate::StoreError;
use crate::facts::AnalysisFact;
use crate::store::ProjectStore;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Directed call graph derived from a project's facts.
#[derive(Debug, Clone, Default)]
pub struct CallGraph {
    /// Caller → callees.
    edges: BTreeMap<u64, BTreeSet<u64>>,
    /// Every node seen (callers, callees, isolated functions).
    nodes: BTreeSet<u64>,
}

/// Parse `0x…` keys (hex by contract — fact object keys are hex).
fn parse_address(key: &str) -> Option<u64> {
    u64::from_str_radix(key.trim_start_matches("0x"), 16).ok()
}

impl CallGraph {
    /// Derive the graph from facts (bounds + call xrefs).
    pub fn from_facts(facts: &[AnalysisFact]) -> Self {
        // Sorted bounds for containing-function lookup.
        let mut bounds: Vec<(u64, u64)> = Vec::new();
        for fact in facts {
            if fact.kind.as_str() != "function_bound" {
                continue;
            }
            let Some(start) = fact.payload.split('-').next().and_then(parse_address) else {
                continue;
            };
            let Some(end) = fact
                .payload
                .split('-')
                .nth(1)
                .map(|rest| rest.split('|').next().unwrap_or(""))
                .and_then(parse_address)
            else {
                continue;
            };
            if end > start {
                bounds.push((start, end));
            }
        }
        bounds.sort_unstable();

        let containing = |address: u64| -> Option<u64> {
            let index = bounds.partition_point(|(start, _)| *start <= address);
            if index == 0 {
                return None;
            }
            let (start, end) = bounds[index - 1];
            (address < end).then_some(start)
        };

        let mut graph = CallGraph::default();
        for (start, end) in &bounds {
            graph.nodes.insert(*start);
            let _ = end;
        }
        for fact in facts {
            if fact.kind.as_str() != "xref" {
                continue;
            }
            let Some(target) = fact
                .payload
                .strip_prefix("call|to=function:")
                .and_then(parse_address)
            else {
                continue;
            };
            let Some(object) = &fact.object_id else {
                continue;
            };
            let Some(instruction) = parse_address(object.key()) else {
                continue;
            };
            let Some(caller) = containing(instruction) else {
                continue;
            };
            graph.nodes.insert(caller);
            graph.nodes.insert(target);
            graph.edges.entry(caller).or_default().insert(target);
        }
        graph
    }

    /// From an open project store.
    pub fn from_store(store: &impl ProjectStore) -> Result<Self, StoreError> {
        Ok(Self::from_facts(&store.facts()?))
    }

    /// All nodes (functions with bounds plus every call target).
    pub fn nodes(&self) -> &BTreeSet<u64> {
        &self.nodes
    }

    /// Direct callees of `function`.
    pub fn callees(&self, function: u64) -> Option<&BTreeSet<u64>> {
        self.edges.get(&function)
    }

    /// Direct callers of `function` (derived index on demand).
    pub fn callers(&self, function: u64) -> Vec<u64> {
        self.edges
            .iter()
            .filter(|(_, callees)| callees.contains(&function))
            .map(|(caller, _)| *caller)
            .collect()
    }

    /// Edge count.
    pub fn edge_count(&self) -> usize {
        self.edges.values().map(|set| set.len()).sum()
    }

    /// Shortest call path `from → to` (BFS). `None` when unreachable.
    /// A trivial path `[from]` is returned when `from == to`.
    pub fn path(&self, from: u64, to: u64) -> Option<Vec<u64>> {
        if from == to {
            return self.nodes.contains(&from).then(|| vec![from]);
        }
        let mut parent: BTreeMap<u64, u64> = BTreeMap::new();
        let mut visited: BTreeSet<u64> = BTreeSet::from([from]);
        let mut queue = VecDeque::from([from]);
        while let Some(current) = queue.pop_front() {
            for callee in self.edges.get(&current).into_iter().flatten() {
                if !visited.insert(*callee) {
                    continue;
                }
                parent.insert(*callee, current);
                if *callee == to {
                    let mut path = vec![to];
                    let mut walk = to;
                    while let Some(step) = parent.get(&walk) {
                        path.push(*step);
                        walk = *step;
                    }
                    path.reverse();
                    return Some(path);
                }
                queue.push_back(*callee);
            }
        }
        None
    }

    /// Every function reachable from `from` (excluding `from` unless
    /// cyclic), in address order.
    pub fn reachable(&self, from: u64) -> BTreeSet<u64> {
        let mut visited = BTreeSet::from([from]);
        let mut queue = VecDeque::from([from]);
        while let Some(current) = queue.pop_front() {
            for callee in self.edges.get(&current).into_iter().flatten() {
                if visited.insert(*callee) {
                    queue.push_back(*callee);
                }
            }
        }
        visited.remove(&from);
        visited
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facts::{Confidence, FactKind, FactProvenance};

    fn fact(kind: &str, object: &str, payload: &str) -> AnalysisFact {
        AnalysisFact {
            source_id: 1,
            object_id: Some(crate::ObjectId::parse(object).unwrap()),
            kind: FactKind::new(kind),
            payload: payload.to_string(),
            confidence: Confidence::Strong,
            provenance: FactProvenance::analyzer("test", "0"),
        }
    }

    fn chain_facts() -> Vec<AnalysisFact> {
        // a(0x1000-0x2000) -> b(0x2000-0x3000) -> c(0x3000-0x4000) -> a
        vec![
            fact(
                "function_bound",
                "function:0x1000",
                "0x1000-0x2000|stop=return",
            ),
            fact(
                "function_bound",
                "function:0x2000",
                "0x2000-0x3000|stop=return",
            ),
            fact(
                "function_bound",
                "function:0x3000",
                "0x3000-0x4000|stop=return",
            ),
            fact("xref", "instruction:0x1004", "call|to=function:0x2000"),
            fact("xref", "instruction:0x2008", "call|to=function:0x9000"),
            fact("xref", "instruction:0x2004", "call|to=function:0x3000"),
            fact("xref", "instruction:0x3004", "call|to=function:0x1000"),
            // Non-call xrefs and out-of-function calls are ignored.
            fact("xref", "instruction:0x100c", "data|to=data:0x5000"),
        ]
    }

    #[test]
    fn derives_edges_with_containment_and_unbounded_targets() {
        let graph = CallGraph::from_facts(&chain_facts());
        // a→b, b→{c, 0x9000}, c→a (the 0x9000 call site sits in b).
        assert_eq!(graph.edge_count(), 4);
        assert!(graph.callees(0x1000).unwrap().contains(&0x2000));
        assert!(graph.callees(0x2000).unwrap().contains(&0x9000));
        assert!(graph.nodes.contains(&0x9000), "unbounded target is a node");
        assert_eq!(graph.callers(0x1000), vec![0x3000], "cycle back-edge");
    }

    #[test]
    fn shortest_paths_and_cycles() {
        let graph = CallGraph::from_facts(&chain_facts());
        assert_eq!(
            graph.path(0x1000, 0x3000),
            Some(vec![0x1000, 0x2000, 0x3000])
        );
        assert_eq!(
            graph.path(0x2000, 0x1000),
            Some(vec![0x2000, 0x3000, 0x1000])
        );
        assert_eq!(
            graph.path(0x1000, 0x9000),
            Some(vec![0x1000, 0x2000, 0x9000]),
            "paths run through unbounded targets"
        );
        assert_eq!(graph.path(0x1000, 0x1000), Some(vec![0x1000]));
        assert_eq!(graph.path(0x9000, 0x1000), None, "sink cannot reach");
        assert!(graph.reachable(0x1000).contains(&0x9000));
        assert!(!graph.reachable(0x9000).contains(&0x1000));
    }

    #[test]
    fn empty_facts_yield_empty_graph() {
        let graph = CallGraph::from_facts(&[]);
        assert!(graph.nodes().is_empty());
        assert_eq!(graph.path(1, 2), None);
    }
}
