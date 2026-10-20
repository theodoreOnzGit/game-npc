# Critique: Oltramari (2023), cognitive neuro-symbolic systems

**Paper:** A. Oltramari, *Enabling High-Level Machine Reasoning with
Cognitive Neuro-Symbolic Systems*, arXiv:2311.07759 (13 Nov 2023). A
version appeared at the AAAI 2024 Spring Symposium, and a journal version
in *Neurosymbolic Artificial Intelligence* as *A Path Towards High-Level
Reasoning Through Cognitive Neuro-Symbolic Systems*. Reference [13] in
`docs/literature-review.md`.

> **What this critique rests on.** The full text could **not** be read.
> arxiv.org, ojs.aaai.org, ceur-ws.org and neurosymbolic-ai-journal.com
> are all blocked in the session that wrote this (2026-10-20). Only the
> following claims, confirmed in search-engine summaries of those pages,
> are used:
>
> 1. Knowledge-infusion methods alone do not give high-level reasoning,
>    meaning generalising from experience and robustness in novel
>    situations.
> 2. The proposal is to augment cognitive architectures with external
>    neuro-symbolic components, with the cognitive architecture as the
>    orchestrator of symbolic knowledge and machine learning.
> 3. The illustration is a hybrid framework centred on ACT-R, chosen
>    because ACT-R's hybrid (symbolic plus subsymbolic) mechanisms suit
>    such integration.
> 4. Generative models are discussed for recent and future applications.
>    Related work by the same group builds commonsense knowledge graphs on
>    demand with generative models, and LLM-ACTR (arXiv:2408.09176) embeds
>    ACT-R decision traces into an LLM.
>
> **Not re-checked:** any experiment, number or architectural detail
> beyond these. Re-read the paper before quoting this critique.

## Fit with this project's goals

| Our goal | What the paper offers | Verdict |
|---|---|---|
| Small, independently trained neural models | The neural side is generative or knowledge-infusion models (claim 4). In this line of work these are typically large pretrained models. | **Do not adopt at runtime.** Large generative models break the 4 GB target and lockstep determinism. They are acceptable only *offline*, for example to draft labelled selection data, with provenance recorded. Even then the labels need checking against outcomes, not taken as ground truth. |
| Typed finite semantic vocabularies | Knowledge graphs and commonsense resources, built on demand by generative models (claim 4). | **Adopt the structure, not the generation.** A finite typed vocabulary is a small closed knowledge graph. Store it in petgraph (#2), authored or curated rather than generated at runtime. On-demand generation makes the vocabulary open-ended and unreplayable, which defeats the point. |
| Deterministic execution of selected algorithms | The cognitive architecture orchestrates the external components (claim 2). | **Already applied.** Soar's decision procedure (`crates/soar-decide`) is the orchestrator. It picks one subroutine per request (`crates/selector`), which then runs deterministically through the typed `Subroutine` interface. The paper supports the shape. It does not argue for determinism; that requirement is ours. |
| Rust/WASM runtime, 4 GB target | ACT-R's reference implementations are not Rust or WASM, and any LLM component is far over budget. | **Reuse mechanisms, not code.** ACT-R's numeric mechanisms are small closed-form rules and port cheaply (below). Its full system and its runtime coupling to generative models do not. |
| Reuse established scientific and game-engine algorithms | Claim 1 is the same diagnosis: infusing knowledge into one model is not enough, so reasoning must be delegated. | **Agrees.** This is the delegation design of #4: PETIR for numerics, goapai for planning, and petgraph and SAGE to come. |

## Critique

1. **It is a position paper, not evidence.** Going by claims 2–3, the
   contribution is an architecture and an illustration. It does not
   supply a benchmark we can reuse or a result that shows the hybrid
   beats a symbolic baseline. Our literature review (§6) already asks for
   that comparison, and it has to be run here: deterministic, Soar
   preferences, ACT-R utility, neural, and hybrid.
2. **"Orchestrator" hides the hard part.** The interface between the
   architecture and the neural module decides whether learned components
   can break symbolic guarantees. This project's answer is concrete:
   - learned models emit only *numeric preferences*;
   - Soar's require/prohibit/better/best and the applicability gate
     decide what is admissible;
   - an inapplicable routine can therefore never win. This is checked by
     a 3000-request property test in `crates/selector`.

   As far as the confirmed claims go, the paper does not pin this
   boundary down.
3. **ACT-R or Soar is not an either/or.** The paper builds on ACT-R; this
   project uses Soar's decision procedure. The useful parts of ACT-R are
   its subsymbolic rules, which sit on top of either architecture:
   - utility learning;
   - base-level activation, the counterpart of Soar's working-memory
     activation, scope item 3 in `docs/soar-survey.md`.
4. **Generative knowledge is not replayable.** On-demand knowledge
   construction (claim 4) helps with novel situations, but for an NPC in
   a lockstep simulation the same inputs must give the same knowledge. If
   generative models are used at all, freeze their output into the
   vocabulary at build time.
5. **The inverse direction is the more transferable idea.** LLM-ACTR
   distils a cognitive architecture's decision traces into a neural
   model. At our scale the analogue is to train the small Burn selector on
   recorded `SelectionTrace`s. One caveat: pure imitation of the
   deterministic selector can at best match it. Traces need *outcome*
   labels (success, cost, reward) for the model to have anything to learn
   beyond the rule.

## Applied in this change

- **`selector::ActrUtility`.** ACT-R's utility-learning rule
  `U ← U + α(R − U)` (Anderson et al. 2004, reference [4]), keyed per
  (input kind, routine), as a `UtilityModel`. It is a learned selector
  that:
  - is deterministic and uses `+ − ×` only;
  - needs no neural network;
  - can only rank applicable routines.

  It is the learned baseline a Burn model must beat (§6.2), and gives the
  "outcome labels" route of point 5 a working consumer. Tests cover the
  equation, convergence, invalid settings, and steering the selector
  without ever choosing an inapplicable routine.

## Not applied, and why

- **Runtime generative models and LLMs:** over budget and not replayable.
- **ACT-R's utility noise:** replaced by greedy choice with a seeded
  tie-break, to keep lockstep replay. Soar's exploration policies can add
  controlled randomness later.
- **ACT-R declarative memory and base-level activation:** deferred to
  Soar scope item 3 (working-memory activation), which covers the same
  mechanism.
