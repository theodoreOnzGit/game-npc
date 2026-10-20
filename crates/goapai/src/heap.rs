// SPDX-License-Identifier: AGPL-3.0-only
// Translated from goapai `heap.go` (https://github.com/akmonengine/goapai,
// commit 2e86c5e, Apache-2.0, Copyright 2024 Agoune Ramine) and from the Go
// standard library's `container/heap` algorithm (BSD-3-Clause, Copyright The
// Go Authors), which goapai drives. Modified: see docs/DEVIATIONS.md.

//! Min-heap of search nodes keyed on `total_cost` (`heap.go` +
//! `container/heap`).
//!
//! The sift-up / sift-down sequence is a line-by-line port of Go's
//! `container/heap`, so nodes with equal cost leave the heap in the same order
//! as upstream. That order decides which of several equal-cost plans is
//! returned, so a different heap (e.g. `alloc::collections::BinaryHeap`)
//! would change results.

use alloc::vec::Vec;

use crate::astar::Node;

/// Indices into a node arena, ordered as a binary min-heap.
#[derive(Clone, Debug, Default)]
pub(crate) struct NodeHeap {
    pub(crate) items: Vec<usize>,
}

impl NodeHeap {
    pub(crate) fn len(&self) -> usize {
        self.items.len()
    }

    fn less(&self, nodes: &[Node], i: usize, j: usize) -> bool {
        nodes[self.items[i]].total_cost < nodes[self.items[j]].total_cost
    }

    pub(crate) fn swap(&mut self, nodes: &mut [Node], i: usize, j: usize) {
        self.items.swap(i, j);
        nodes[self.items[i]].heap_index = Some(i);
        nodes[self.items[j]].heap_index = Some(j);
    }

    /// `nodeHeap.Push`: append without sifting.
    pub(crate) fn push_raw(&mut self, nodes: &mut [Node], n: usize) {
        nodes[n].heap_index = Some(self.items.len());
        self.items.push(n);
    }

    /// `nodeHeap.Pop`: remove the last element without sifting.
    pub(crate) fn pop_raw(&mut self, nodes: &mut [Node]) -> usize {
        let n = self.items.pop().expect("pop_raw on empty heap");
        nodes[n].heap_index = None;
        n
    }

    /// `heap.Push`.
    pub(crate) fn push(&mut self, nodes: &mut [Node], n: usize) {
        self.push_raw(nodes, n);
        self.up(nodes, self.len() - 1);
    }

    /// `heap.Pop`.
    pub(crate) fn pop(&mut self, nodes: &mut [Node]) -> usize {
        let n = self.len() - 1;
        self.swap(nodes, 0, n);
        self.down(nodes, 0, n);
        self.pop_raw(nodes)
    }

    /// `heap.Fix`.
    pub(crate) fn fix(&mut self, nodes: &mut [Node], i: usize) {
        if !self.down(nodes, i, self.len()) {
            self.up(nodes, i);
        }
    }

    fn up(&mut self, nodes: &mut [Node], mut j: usize) {
        while j > 0 {
            let i = (j - 1) / 2;
            if !self.less(nodes, j, i) {
                break;
            }
            self.swap(nodes, i, j);
            j = i;
        }
    }

    fn down(&mut self, nodes: &mut [Node], i0: usize, n: usize) -> bool {
        let mut i = i0;
        loop {
            let j1 = 2 * i + 1;
            if j1 >= n {
                break;
            }
            let mut j = j1;
            let j2 = j1 + 1;
            if j2 < n && self.less(nodes, j2, j1) {
                j = j2;
            }
            if !self.less(nodes, j, i) {
                break;
            }
            self.swap(nodes, i, j);
            i = j;
        }
        i > i0
    }
}

#[cfg(test)]
#[path = "heap_test.rs"]
mod heap_test;
