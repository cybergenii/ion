use anyhow::Result;
use std::collections::{HashMap, HashSet, VecDeque};

/// A node in the dependency graph after resolution
#[derive(Debug, Clone)]
pub struct ResolvedNode {
    pub name: String,
    pub version: String,
    pub source_uri: String,
    pub cmake_targets: Vec<String>,
    /// Direct dependencies ("name version_req" strings)
    pub direct_deps: Vec<String>,
}

/// Directed dependency graph with cycle detection and topological sort.
pub struct DependencyGraph {
    /// name → (version, direct_dep_names)
    nodes: HashMap<String, (String, Vec<String>)>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, name: String, version: String, deps: Vec<String>) {
        self.nodes.insert(name, (version, deps));
    }

    /// Kahn's algorithm for topological sort.
    /// Returns node names in dependency-first order (leaves first).
    /// Returns an error if a cycle is detected.
    pub fn topological_sort(&self) -> Result<Vec<String>> {
        // Build adjacency list and in-degree map
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();

        for (name, (_, deps)) in &self.nodes {
            in_degree.entry(name.as_str()).or_insert(0);
            for dep in deps {
                if self.nodes.contains_key(dep.as_str()) {
                    *in_degree.entry(name.as_str()).or_insert(0) += 1;
                    adj.entry(dep.as_str())
                        .or_default()
                        .push(name.as_str());
                }
            }
        }

        // Nodes with no dependencies go first
        let mut queue: VecDeque<&str> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&n, _)| n)
            .collect();

        let mut sorted = Vec::new();

        while let Some(node) = queue.pop_front() {
            sorted.push(node.to_string());

            if let Some(dependents) = adj.get(node) {
                for &dependent in dependents {
                    let deg = in_degree.get_mut(dependent).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(dependent);
                    }
                }
            }
        }

        if sorted.len() != self.nodes.len() {
            // Cycle detected — identify the participants
            let in_cycle: Vec<&str> = in_degree
                .iter()
                .filter(|(_, &d)| d > 0)
                .map(|(&n, _)| n)
                .collect();
            anyhow::bail!(
                "Circular dependency detected among: [{}]. \
                 Ion cannot resolve cyclic C++ dependencies.",
                in_cycle.join(", ")
            );
        }

        Ok(sorted)
    }

    /// Find all packages that are transitively required by `root`
    pub fn transitive_deps(&self, root: &str) -> HashSet<String> {
        let mut visited = HashSet::new();
        let mut stack = vec![root.to_string()];

        while let Some(current) = stack.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());

            if let Some((_, deps)) = self.nodes.get(&current) {
                for dep in deps {
                    if !visited.contains(dep) {
                        stack.push(dep.clone());
                    }
                }
            }
        }

        visited.remove(root);
        visited
    }

    /// Check if removing `package` would leave any other package without its dependency
    pub fn check_removal(&self, package: &str) -> Vec<String> {
        let mut affected = Vec::new();
        for (name, (_, deps)) in &self.nodes {
            if name != package && deps.contains(&package.to_string()) {
                affected.push(name.clone());
            }
        }
        affected
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topo_sort_simple() {
        let mut g = DependencyGraph::new();
        g.add_node("C".into(), "1.0".into(), vec![]);
        g.add_node("B".into(), "1.0".into(), vec!["C".into()]);
        g.add_node("A".into(), "1.0".into(), vec!["B".into(), "C".into()]);

        let sorted = g.topological_sort().unwrap();
        // C must come before B and A, B must come before A
        let pos = |n: &str| sorted.iter().position(|x| x == n).unwrap();
        assert!(pos("C") < pos("B"));
        assert!(pos("B") < pos("A"));
    }

    #[test]
    fn test_cycle_detection() {
        let mut g = DependencyGraph::new();
        g.add_node("A".into(), "1.0".into(), vec!["B".into()]);
        g.add_node("B".into(), "1.0".into(), vec!["A".into()]);

        let result = g.topological_sort();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Circular dependency"));
    }

    #[test]
    fn test_transitive_deps() {
        let mut g = DependencyGraph::new();
        g.add_node("C".into(), "1.0".into(), vec![]);
        g.add_node("B".into(), "1.0".into(), vec!["C".into()]);
        g.add_node("A".into(), "1.0".into(), vec!["B".into()]);

        let deps = g.transitive_deps("A");
        assert!(deps.contains("B"));
        assert!(deps.contains("C"));
        assert!(!deps.contains("A"));
    }

    #[test]
    fn test_check_removal() {
        let mut g = DependencyGraph::new();
        g.add_node("fmt".into(), "10.0".into(), vec![]);
        g.add_node("spdlog".into(), "1.0".into(), vec!["fmt".into()]);

        let affected = g.check_removal("fmt");
        assert!(affected.contains(&"spdlog".to_string()));
    }
}
