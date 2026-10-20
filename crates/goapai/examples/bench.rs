// SPDX-License-Identifier: AGPL-3.0-only
// Translated from goapai `benchmark/goapai_test.go` (commit 2e86c5e,
// Apache-2.0, Copyright 2024 Agoune Ramine).

//! Same scenario as upstream's `BenchmarkGoapAI`: time per `get_plan` and
//! bytes allocated per call. Run with
//! `cargo run --release -p goapai --example bench --features std`.
//!
//! Timing and allocation harness only; a test is impractical for a
//! wall-clock measurement.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use goapai::*;

struct Counting;
static BYTES: AtomicU64 = AtomicU64::new(0);
static ALLOCS: AtomicU64 = AtomicU64::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

const ATTRIBUTE_1: StateKey = 0;
const ATTRIBUTE_2: StateKey = 1;
const ATTRIBUTE_3: StateKey = 2;

fn main() {
    use Arithmetic::*;
    use Operator::*;
    let mut actions = Actions::new();
    actions.add_action(
        "action1",
        1.0,
        true,
        vec![Condition::new(ATTRIBUTE_1, Upper, 0_i64).into()],
        vec![
            Effect::new(ATTRIBUTE_1, Subtract, 50_i64).into(),
            Effect::new(ATTRIBUTE_2, Subtract, 5_i64).into(),
        ],
    );
    actions.add_action(
        "action2",
        1.0,
        true,
        vec![Condition::new(ATTRIBUTE_3, Lower, 50_i64).into()],
        vec![
            Effect::new(ATTRIBUTE_3, Add, 20_i64).into(),
            Effect::new(ATTRIBUTE_2, Add, 10_i64).into(),
            Effect::new(ATTRIBUTE_1, Add, 5_i64).into(),
        ],
    );
    actions.add_action(
        "action3",
        1.0,
        true,
        vec![Condition::new(ATTRIBUTE_3, Upper, 30_i64).into()],
        vec![Effect::new(ATTRIBUTE_3, Subtract, 30_i64).into()],
    );
    let goals = Goals::new()
        .with(
            "goal1",
            GoalInterface::new(
                vec![Condition::new(ATTRIBUTE_2, Upper, 80_i64).into()],
                |_| 1.0,
            ),
        )
        .with(
            "goal2",
            GoalInterface::new(
                vec![Condition::new(ATTRIBUTE_2, Equal, 100_i64).into()],
                |_| 0.0,
            ),
        );
    let mut agent = create_agent(goals, actions);
    set_state(&mut agent, ATTRIBUTE_1, 80_i64);
    set_state(&mut agent, ATTRIBUTE_2, 0_i64);
    set_state(&mut agent, ATTRIBUTE_3, 0_i64);

    let (goal, plan) = get_plan(&agent, 15);
    let names: Vec<&str> = plan.steps().iter().map(|a| a.get_name()).collect();
    println!("goal={goal} plan={names:?} cost={}", plan.get_total_cost());

    let iterations = 2000;
    let b0 = BYTES.load(Ordering::Relaxed);
    let a0 = ALLOCS.load(Ordering::Relaxed);
    let t0 = Instant::now();
    for _ in 0..iterations {
        std::hint::black_box(get_plan(std::hint::black_box(&agent), 15));
    }
    let dt = t0.elapsed();
    let bytes = BYTES.load(Ordering::Relaxed) - b0;
    let allocs = ALLOCS.load(Ordering::Relaxed) - a0;
    println!(
        "{} ns/op  {} B/op  {} allocs/op",
        dt.as_nanos() / iterations as u128,
        bytes / iterations,
        allocs / iterations
    );
}
