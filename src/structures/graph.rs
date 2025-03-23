/*
This module defines the `Graph` structure and its associated methods.

The `Graph` structure represents the entire network of nodes in the SAT solver.
It manages node creation, deletion, and connections while respecting Rust's
ownership and borrowing rules.
*/

use std::collections::HashMap;
use crate::structures::node::{Node, Message};

pub struct Graph {
    nodes: HashMap<u32, Node>,
    next_node_id: u32,
}

impl Graph {
    pub fn new() -> Self {
        Graph {
            nodes: HashMap::new(),
            next_node_id: 0,
        }
    }

    pub fn create_node(&mut self) -> u32 {
        let id = self.next_node_id;
        self.nodes.insert(id, Node::new());
        self.next_node_id += 1;
        id
    }

    pub fn remove_node(&mut self, id: u32) {
        // First remove this node from all other nodes' neighbor lists
        for node in self.nodes.values_mut() {
            node.remove_neighbor(id);
        }
        // Then remove the node itself
        self.nodes.remove(&id);
    }

    pub fn add_edge(&mut self, from_id: u32, to_id: u32) {
        if let Some(from_node) = self.nodes.get_mut(&from_id) {
            if self.nodes.contains_key(&to_id) {
                from_node.add_neighbor(to_id);
            } else {
                panic!("Target node {} does not exist!", to_id);
            }
        } else {
            panic!("Source node {} does not exist!", from_id);
        }
    }

    pub fn remove_edge(&mut self, from_id: u32, to_id: u32) {
        if let Some(from_node) = self.nodes.get_mut(&from_id) {
            from_node.remove_neighbor(to_id);
        }
    }

    pub fn send_message(&mut self, from_id: u32, to_id: u32, message: Message) {
        if let Some(to_node) = self.nodes.get_mut(&to_id) {
            to_node.set_neighbor_message(from_id, message);
        } else {
            panic!("Target node {} does not exist!", to_id);
        }
    }

    pub fn clear_message(&mut self, from_id: u32, to_id: u32) {
        if let Some(to_node) = self.nodes.get_mut(&to_id) {
            to_node.clear_neighbor_message(from_id);
        }
    }

    pub fn get_message(&self, from_id: u32, to_id: u32) -> Option<&Message> {
        self.nodes
            .get(&to_id)
            .and_then(|node| node.get_neighbor_message(from_id))
    }

    pub fn get_node(&self, id: u32) -> Option<&Node> {
        self.nodes.get(&id)
    }

    pub fn get_node_mut(&mut self, id: u32) -> Option<&mut Node> {
        self.nodes.get_mut(&id)
    }

    pub fn get_node_ids(&self) -> Vec<u32> {
        self.nodes.keys().copied().collect()
    }

    pub fn get_neighbors(&self, id: u32) -> Vec<u32> {
        self.nodes
            .get(&id)
            .map(|node| node.get_neighbor_ids())
            .unwrap_or_default()
    }

    pub fn has_available_neighbor(&self, id: u32) -> bool {
        self.nodes
            .get(&id)
            .map(|node| node.has_available_neighbor())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_creation() {
        let mut graph = Graph::new();
        let node1 = graph.create_node();
        let node2 = graph.create_node();
        
        assert_eq!(graph.get_node_ids(), vec![0, 1]);
        assert!(graph.get_node(node1).is_some());
        assert!(graph.get_node(node2).is_some());
    }

    #[test]
    fn test_edge_management() {
        let mut graph = Graph::new();
        let node1 = graph.create_node();
        let node2 = graph.create_node();
        
        graph.add_edge(node1, node2);
        assert_eq!(graph.get_neighbors(node1), vec![node2]);
        
        graph.remove_edge(node1, node2);
        assert!(graph.get_neighbors(node1).is_empty());
    }

    #[test]
    fn test_message_passing() {
        let mut graph = Graph::new();
        let node1 = graph.create_node();
        let node2 = graph.create_node();
        
        graph.add_edge(node1, node2);
        
        let message = Message::Success;
        graph.send_message(node1, node2, message);
        assert!(graph.get_message(node1, node2).is_some());
        
        graph.clear_message(node1, node2);
        assert!(graph.get_message(node1, node2).is_none());
    }
} 