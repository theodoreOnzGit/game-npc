// SPDX-License-Identifier: AGPL-3.0-only
// Translated from goapai `heap_test.go` (commit 2e86c5e, Apache-2.0,
// Copyright 2024 Agoune Ramine).

use super::*;
use alloc::vec;

fn heap_of(costs: &[f32]) -> (NodeHeap, Vec<Node>) {
    let mut nodes: Vec<Node> = costs.iter().map(|c| Node::with_cost(*c)).collect();
    let mut h = NodeHeap::default();
    for i in 0..nodes.len() {
        h.push_raw(&mut nodes, i);
    }
    (h, nodes)
}

#[test]
fn node_heap_less() {
    let cases = [
        ("first node has lower cost", [5.0, 10.0], true),
        ("first node has higher cost", [15.0, 10.0], false),
        ("equal costs", [10.0, 10.0], false),
    ];
    for (name, costs, want) in cases {
        let (h, nodes) = heap_of(&costs);
        assert_eq!(h.less(&nodes, 0, 1), want, "{name}");
    }
}

#[test]
fn node_heap_swap() {
    let cases: [(&str, &[f32], usize, usize); 2] = [
        ("swap first and second", &[5.0, 10.0], 0, 1),
        ("swap first and third", &[5.0, 10.0, 15.0], 0, 2),
    ];
    for (name, costs, i, j) in cases {
        let (mut h, mut nodes) = heap_of(costs);
        let (orig_i, orig_j) = (h.items[i], h.items[j]);
        h.swap(&mut nodes, i, j);
        assert_eq!(h.items[i], orig_j, "{name}");
        assert_eq!(h.items[j], orig_i, "{name}");
        assert_eq!(nodes[h.items[i]].heap_index, Some(i), "{name}");
        assert_eq!(nodes[h.items[j]].heap_index, Some(j), "{name}");
    }
}

#[test]
fn node_heap_push() {
    let cases: [(&str, &[f32], usize, usize); 2] = [
        ("push to empty heap", &[], 1, 0),
        ("push to non-empty heap", &[5.0, 10.0], 3, 2),
    ];
    for (name, costs, want_len, want_index) in cases {
        let (mut h, mut nodes) = heap_of(costs);
        nodes.push(Node::with_cost(15.0));
        let n = nodes.len() - 1;
        h.push_raw(&mut nodes, n);
        assert_eq!(h.len(), want_len, "{name}");
        assert_eq!(nodes[n].heap_index, Some(want_index), "{name}");
        assert_eq!(*h.items.last().unwrap(), n, "{name}");
    }
}

#[test]
fn node_heap_pop() {
    let cases: [(&str, &[f32], usize); 2] = [
        ("pop from heap with one element", &[5.0], 0),
        (
            "pop from heap with multiple elements",
            &[5.0, 10.0, 15.0],
            2,
        ),
    ];
    for (name, costs, want_len) in cases {
        let (mut h, mut nodes) = heap_of(costs);
        let last = *h.items.last().unwrap();
        let popped = h.pop_raw(&mut nodes);
        assert_eq!(h.len(), want_len, "{name}");
        assert_eq!(nodes[popped].heap_index, None, "{name}");
        assert_eq!(popped, last, "{name}");
    }
}

// ---- Tests added in the translation (not in upstream) ----

/// Equal-cost nodes pop in the order Go's container/heap produces. Reference
/// order recorded from Go 1.24.7 pushing these costs in this order (arena
/// index = push index).
#[test]
fn equal_cost_pop_order_matches_go_container_heap() {
    let costs = [3.0, 1.0, 1.0, 2.0, 1.0, 3.0, 2.0, 1.0];
    let mut nodes: Vec<Node> = costs.iter().map(|c| Node::with_cost(*c)).collect();
    let mut h = NodeHeap::default();
    for i in 0..nodes.len() {
        h.push(&mut nodes, i);
    }
    let mut order = vec![];
    while h.len() > 0 {
        order.push(h.pop(&mut nodes));
    }
    assert_eq!(order, GO_POP_ORDER);
}

const GO_POP_ORDER: [usize; 8] = [1, 4, 7, 2, 3, 6, 5, 0];

#[test]
fn fix_after_decrease_moves_node_up() {
    let mut nodes: Vec<Node> = [5.0, 6.0, 7.0, 8.0]
        .iter()
        .map(|c| Node::with_cost(*c))
        .collect();
    let mut h = NodeHeap::default();
    for i in 0..nodes.len() {
        h.push(&mut nodes, i);
    }
    nodes[3].total_cost = 1.0;
    let i = nodes[3].heap_index.unwrap();
    h.fix(&mut nodes, i);
    assert_eq!(h.pop(&mut nodes), 3);
}
