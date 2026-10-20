// SPDX-License-Identifier: AGPL-3.0-only

//! Differential fixture generator: runs random preference scenarios through
//! the real `soar` binary and records what it did.
//!
//! ```bash
//! SOAR_BIN=/path/to/soar \
//!   cargo run --release -p soar-decide --example gen_differential --features std
//! ```
//!
//! For each scenario it writes a `.soar` file with:
//! - a fixed `srand` seed, exploration policy, ε, temperature and
//!   numeric-indifferent mode;
//! - one production that proposes 1–5 operators with random preferences of
//!   every type.
//!
//! It then runs, in order: the proposal phase, `preferences S1 operator`,
//! the decision phase, and `print S2`. From Soar's output it records:
//! - each preference list in Soar's internal order;
//! - the read-only candidates and selection probabilities;
//! - the decision: the selected operator, or the impasse type, attribute
//!   and items.
//!
//! The fixture goes to `tests/fixtures/differential.txt`.
//!
//! Tooling only, not shipped code: it shells out to an external binary, so a
//! test cannot reach it. Its output is what `tests/differential.rs` checks.

use std::fmt::Write as _;
use std::process::Command;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    fn n(&mut self, k: u64) -> u64 {
        self.next() % k
    }
    fn chance(&mut self, pct: u64) -> bool {
        self.n(100) < pct
    }
}

const POLICIES: [&str; 5] = ["boltzmann", "epsilon-greedy", "first", "last", "softmax"];
const EPSILONS: [&str; 5] = ["0", "0.1", "0.3", "0.5", "1"];
const TEMPERATURES: [&str; 4] = ["0.25", "0.5", "1", "25"];

/// Section headers of Soar's `preferences` output, mapped to
/// `PreferenceType` numbering.
const SECTIONS: [(&str, u8); 11] = [
    ("acceptables:", 0),
    ("requires:", 1),
    ("rejects:", 2),
    ("prohibits:", 3),
    ("unary indifferents:", 5),
    ("bests:", 7),
    ("worsts:", 8),
    ("binary indifferents:", 9),
    ("betters:", 11),
    ("worses:", 12),
    ("numeric indifferents:", 13),
];

fn number(r: &mut Rng) -> String {
    // Multiples of 1/8 in [-1, 2]: exact in binary and in Soar's %f output.
    let k = r.n(25) as i64 - 8;
    format!("{}", k as f64 / 8.0)
}

fn scenario(id: usize, r: &mut Rng) -> (String, String) {
    let seed = r.next() as u32;
    let policy = POLICIES[r.n(5) as usize];
    let eps = EPSILONS[r.n(5) as usize];
    let temp = TEMPERATURES[r.n(4) as usize];
    let avg = r.chance(30);
    let k = 1 + r.n(5) as usize;
    // Mostly indifferent scenarios exercise the exploration policies.
    let indifferent = r.chance(55);

    let mut rhs = String::new();
    for i in 1..=k {
        // Grounded via ^cand so Soar accepts operators with no preference.
        let _ = write!(rhs, " (<s> ^cand <o{i}>) (<o{i}> ^name o{i})");
    }
    let op = |r: &mut Rng| 1 + r.n(k as u64) as usize;
    for i in 1..=k {
        if r.chance(85) {
            let _ = write!(rhs, " (<s> ^operator <o{i}> +)");
        }
        if indifferent && r.chance(85) {
            let _ = write!(rhs, " (<s> ^operator <o{i}> = {})", number(r));
        }
    }
    // Dominance mode: several better/worse pairs, for conflict impasses and
    // the order-dependent conflict marking.
    if !indifferent && r.chance(40) {
        for _ in 0..2 + r.n(4) {
            let (a, b) = (op(r), op(r));
            let rel = if r.chance(60) { ">" } else { "<" };
            let _ = write!(rhs, " (<s> ^operator <o{a}> {rel} <o{b}>)");
        }
    }
    let extra = r.n(if indifferent { 4 } else { 8 });
    for _ in 0..extra {
        let (a, b) = (op(r), op(r));
        let pref = match r.n(100) {
            0..=4 => format!("<o{a}> !"),
            5..=12 => format!("<o{a}> -"),
            13..=18 => format!("<o{a}> ~"),
            19..=27 => format!("<o{a}> >"),
            28..=36 => format!("<o{a}> <"),
            37..=47 => format!("<o{a}> ="),
            48..=59 => format!("<o{a}> > <o{b}>"),
            60..=69 => format!("<o{a}> < <o{b}>"),
            70..=81 => format!("<o{a}> = <o{b}>"),
            _ => format!("<o{a}> = {}", number(r)),
        };
        let _ = write!(rhs, " (<s> ^operator {pref})");
    }

    let soar = format!(
        "srand {seed}\n\
         decide indifferent-selection --{policy}\n\
         decide indifferent-selection --epsilon {eps}\n\
         decide indifferent-selection --temperature {temp}\n\
         decide numeric-indifferent-mode --{mode}\n\
         sp {{all (state <s> ^superstate nil) -->{rhs}}}\n\
         run -p 2\n\
         echo ===PREFS\n\
         preferences S1 operator\n\
         echo ===DECIDE\n\
         run -p 1\n\
         echo ===S1\n\
         print S1\n\
         echo ===S2\n\
         print S2\n\
         echo ===END\n",
        mode = if avg { "avg" } else { "sum" },
    );
    let header = format!(
        "S {id} {seed} {policy} {eps} {temp} {}\n",
        if avg { "avg" } else { "sum" }
    );
    (soar, header)
}

fn oid(tok: &str) -> Option<u32> {
    tok.strip_prefix('O')?.parse().ok()
}

fn parse(out: &str) -> String {
    let mut rec = String::new();
    let prefs = out
        .split("===PREFS")
        .nth(1)
        .and_then(|s| s.split("===DECIDE").next())
        .unwrap_or("");
    let s1 = out
        .split("===S1")
        .nth(1)
        .and_then(|s| s.split("===S2").next())
        .unwrap_or("");
    let s2 = out
        .split("===S2")
        .nth(1)
        .and_then(|s| s.split("===END").next())
        .unwrap_or("");

    let mut section: Option<u8> = None;
    let mut in_probs = false;
    let mut cands = Vec::new();
    for line in prefs.lines() {
        let t = line.trim();
        if let Some((_, code)) = SECTIONS.iter().find(|(h, _)| *h == t) {
            section = Some(*code);
            in_probs = false;
            continue;
        }
        if t == "selection probabilities:" {
            section = None;
            in_probs = true;
            continue;
        }
        let toks: Vec<&str> = t.split_whitespace().collect();
        let Some(v) = toks.first().and_then(|x| oid(x)) else {
            continue;
        };
        if in_probs {
            let pct = toks
                .last()
                .unwrap()
                .trim_start_matches('(')
                .trim_end_matches("%)");
            cands.push(format!("{v} {pct}"));
            continue;
        }
        let Some(code) = section else { continue };
        let referent = match code {
            9 | 11 | 12 => {
                let r = toks
                    .iter()
                    .skip(2)
                    .find_map(|x| oid(x))
                    .expect("binary referent");
                format!("O{r}")
            }
            13 => {
                let i = toks.iter().position(|x| *x == "=").expect("numeric =");
                let x: f64 = toks[i + 1].parse().expect("number");
                format!("N{:016x}", x.to_bits())
            }
            _ => "-".into(),
        };
        let _ = writeln!(rec, "P {code} {v} {referent}");
    }
    if cands.is_empty() {
        rec.push_str("C -\n");
    } else {
        let _ = writeln!(rec, "C {}", cands.join(" "));
    }

    // Outcome from working memory: an impasse substate S2, or an operator
    // WME on S1 (`^operator Ox` not followed by `+`).
    let s2_toks: Vec<&str> = s2.split_whitespace().collect();
    let attr_of = |name: &str| {
        s2_toks
            .iter()
            .position(|t| *t == name)
            .map(|i| s2_toks[i + 1].trim_end_matches(')'))
    };
    if let Some(kind) = attr_of("^impasse") {
        let attr = attr_of("^attribute").expect("impasse attribute");
        let items: Vec<String> = s2_toks
            .windows(2)
            .filter(|w| w[0] == "^item")
            .filter_map(|w| oid(w[1].trim_end_matches(')')).map(|v| v.to_string()))
            .collect();
        let _ = writeln!(rec, "D imp {kind} {attr} {}", items.join(" "));
    } else {
        let t: Vec<&str> = s1.split_whitespace().collect();
        let sel = t
            .windows(3)
            .find(|w| w[0] == "^operator" && w[2] != "+")
            .or_else(|| {
                t.windows(2)
                    .find(|w| w[0] == "^operator")
                    .filter(|w| !w[1].ends_with('+'))
            })
            .and_then(|w| oid(w[1].trim_end_matches(')')))
            .expect("selected operator on S1");
        let _ = writeln!(rec, "D sel {sel}");
    }
    rec
}

fn main() {
    let soar = std::env::var("SOAR_BIN").expect("set SOAR_BIN to the soar executable");
    let count: usize = std::env::var("SCENARIOS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2000);
    let dir = std::env::temp_dir().join("soar-decide-differential");
    std::fs::create_dir_all(&dir).unwrap();
    let mut fixture = String::new();
    let _ = writeln!(
        fixture,
        "# soar-decide differential fixture; generated by examples/gen_differential.rs"
    );
    let _ = writeln!(
        fixture,
        "# against Soar commit 3fc337a1ab2d04b9e5183335c1496d7dcdc57c1c (kernel 9.6.5)"
    );
    let mut r = Rng(0x50a2);
    for id in 0..count {
        let (text, header) = scenario(id, &mut r);
        let path = dir.join(format!("s{id}.soar"));
        std::fs::write(&path, &text).unwrap();
        let out = Command::new(&soar)
            .arg("-s")
            .arg(&path)
            .stdin(std::process::Stdio::null())
            .output()
            .expect("run soar");
        let out = String::from_utf8_lossy(&out.stdout);
        fixture.push_str(&header);
        fixture.push_str(&parse(&out));
    }
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/differential.txt"
    );
    std::fs::write(path, fixture).unwrap();
    println!("wrote {count} scenarios to {path}");
}
