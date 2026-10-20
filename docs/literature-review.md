# Literature Review: Neuro-Symbolic Cognitive Architectures for Lightweight Autonomous Agents

> **Provenance.** Supplied by the maintainer on 2026-10-20 and ingested
> verbatim apart from Markdown formatting. It is a preliminary synthesis,
> not a systematic review (see its own verification note at the end).
> **Not re-checked:** none of the references were opened when this file was
> ingested. Network access to the publishers is blocked in the session that
> ingested it. Reference metadata stands as supplied.
>
> **Where this repository already touches it** (added at ingestion, not part
> of the review):
>
> | Review section | Code or issue |
> |---|---|
> | §2.1 Soar, operator selection, impasses | #4; `docs/soar-survey.md`; `crates/soar-decide` |
> | §2.2 Soar-RL | #4 scope item 2 in `docs/soar-survey.md` |
> | §2.4 CLARION motivational subsystem | #1; `src/` motivation core |
> | §3.1–3.2 GOAP and implementations | #3 (closed); `crates/goapai` |
> | §3.3 learned motivation → goals → planning | #1 (`MotivationModel`, `generate_goal`) + #3 |
> | §5.2 symbolic applicability, then learned preference | #4 selector; `ValidatedAction` in `src/action.rs` |
> | §6.3 explainability and replay | `src/trace.rs` (checksum, bit-exact replay) |
> | [11], [12] Talevich, Reiss | #1; `docs/motivation-taxonomy.md` |
> | [13] Oltramari (ACT-R orchestration) | `docs/oltramari-2023-critique.md`; `selector::ActrUtility` |
> | [4] ACT-R utility learning | `selector::ActrUtility` |

**Research focus:** Deterministic-first cognitive architectures combining
symbolic reasoning, learned operator selection, hierarchical planning, and
specialised computational algorithms.

**Target implementation:** Rust, WebAssembly, PETIR, Burn, petgraph, GOAP,
and potentially Soar-derived components.

**Review scope:** Foundational cognitive architectures and selected research
through 2026.

## 1. Introduction

Artificial intelligence research has historically pursued two broad
approaches to reasoning and decision-making: symbolic systems, which
represent knowledge and operations explicitly, and subsymbolic systems,
which learn relationships through numerical representations.

Symbolic architectures provide structured reasoning, explicit state
representations, and interpretable decision procedures. Neural
architectures offer adaptive pattern recognition and learned heuristics,
particularly where complete deterministic specifications are difficult to
construct.

Neuro-symbolic systems attempt to combine these capabilities.

This review examines cognitive architectures, operator-selection
mechanisms, hierarchical planning, and contemporary neuro-symbolic
approaches relevant to a lightweight intelligence engine in which learned
models select among deterministic computational procedures.

The proposed engineering objective is not unrestricted general
intelligence. It is efficient, specialised reasoning and decision-making
within constrained domains such as non-player characters, software
engineering, and scientific computation.

## 2. Classical Cognitive Architectures

### 2.1 Soar

Soar is a symbolic cognitive architecture originating from research by
Newell, Laird, and Rosenbloom [1, 2].

Its central mechanisms include:

- Symbolic working memory.
- Production-rule matching.
- Operator proposal and selection.
- Preference-based decision-making.
- Impasse detection and substate creation.
- Chunking, which allows previously derived results to inform subsequent
  problem-solving.

Soar represents problem-solving as repeated operator-selection cycles.

When available knowledge is insufficient to select an operator, the
architecture can create a substate in which additional reasoning is
performed.

Chunking can subsequently compile aspects of successful problem-solving
into production rules, reducing the need to repeat intermediate reasoning.

These mechanisms provide an established precedent for an architecture in
which a relatively simple selector operates over a finite set of
procedures, while more complex reasoning emerges through higher-level
composition.

**Relevance:** Soar provides a candidate symbolic foundation for subroutine
selection and delegated reasoning.

Source: [Soar Cognitive Architecture](https://soar.eecs.umich.edu/) ·
[GitHub](https://github.com/SoarGroup/Soar)

### 2.2 Soar Reinforcement Learning

Soar has been extended with reinforcement-learning mechanisms that adjust
numerical operator preferences using experience [3].

This allows operator selection to reflect learned estimates of utility
rather than relying exclusively on manually specified preferences.

The distinction between operator applicability and operator preference is
particularly important.

A symbolic system can determine which operations are permitted or
applicable, while a learned system estimates their relative desirability.

This provides a conceptual precedent for integrating a neural evaluator
into Soar's operator-selection mechanism.

**Relevance:** A small Burn model could estimate preferences among
applicable operators while preserving symbolic applicability constraints.

### 2.3 ACT-R

Adaptive Control of Thought—Rational (ACT-R) is a cognitive architecture
that combines symbolic production rules with quantitative mechanisms for
memory activation and procedural utility [4].

Its architecture distinguishes declarative knowledge from procedural
knowledge and models how cognitive processes compete for limited
resources.

Production selection can be influenced by learned utility estimates,
allowing the architecture to adapt its behaviour through experience.

ACT-R demonstrates that numerical decision mechanisms can coexist with
explicit symbolic reasoning without requiring all cognition to be
implemented as a neural network.

**Relevance:** ACT-R provides a comparative model for learned procedural
utility and constrained cognitive resources.

### 2.4 CLARION

CLARION, developed by Ron Sun, explicitly combines implicit and explicit
cognitive representations [5].

Its architecture includes several interacting subsystems:

- Action-centred processing.
- Non-action-centred knowledge.
- Motivational processes.
- Metacognitive control.

CLARION is particularly relevant because it separates motivational
processes from action selection and knowledge representation.

Its combination of implicit and explicit representations offers a
precedent for integrating neural models with symbolic cognitive
structures.

**Relevance:** CLARION provides an architectural reference for separating
emotional resources, motivations, knowledge, reasoning, and agency.

## 3. Goal-Oriented Planning

### 3.1 Goal-Oriented Action Planning

Goal-Oriented Action Planning (GOAP) represents actions through
preconditions, effects, and costs.

A planner searches for action sequences that transform an initial state
into one satisfying a goal.

GOAP is useful for autonomous agents because the space of possible
behaviours can be generated compositionally from a finite set of actions.

This avoids explicitly programming every possible action sequence.

However, conventional GOAP does not inherently solve the problem of
selecting which goals should be pursued.

Goal selection can therefore be separated from goal achievement.

### 3.2 Existing Open-Source Implementations

Several implementations provide potential reference architectures for Rust
translation:

| Implementation | Language | Relevant characteristics |
|---|---|---|
| [GPGOAP](https://github.com/stolk/GPGOAP) | C | Compact Boolean-state planning |
| [ReGoap](https://github.com/luxkun/ReGoap) | C# | Goals, memory, sensors, and planning |
| [CrashKonijn GOAP](https://github.com/crashkonijn/GOAP) | C# | Extensive Unity-oriented planning infrastructure |
| [goapai](https://github.com/akmonengine/goapai) | Go | Typed numerical states and configurable planning |

These implementations differ in maturity, representation, platform
dependencies, and extensibility.

A Rust translation should preserve intended algorithms and functionality
while allowing corrections to implementation defects and adaptation to
WASM.

### 3.3 Integration with Learned Motivation

A hybrid architecture can separate three problems:

1. Determine which motivations are currently important.
2. Translate motivations into candidate goals.
3. Construct feasible action sequences using deterministic planning.

A neural model can estimate motivation priorities, while GOAP remains
responsible for planning.

This permits individual character preferences to influence behaviour
without requiring neural networks to learn pathfinding, action
preconditions, or physical simulation.

## 4. Contemporary Neuro-Symbolic Research

### 4.1 Hierarchical Neuro-Symbolic Decision Transformers

The Hierarchical Neuro-Symbolic Decision Transformer (2025) combines
high-level symbolic planning with learned decision policies [6].

This research illustrates a broader architectural direction in which
symbolic structures constrain or organise neural decision-making.

The relationship between symbolic and neural components is hierarchical
rather than purely sequential.

**Relevance:** Supports investigation of symbolic operator constraints
combined with learned decision preferences.

### 4.2 Neuro-Symbolic World Models

Neurosymbolic World Models for Sequential Decision Making (ICML 2025)
investigates structured representations of environment dynamics for
sequential decision-making [7].

This research is relevant to agents operating in environments where
decisions change the state of the world.

Structured world models can support planning, prediction, and
interpretation of state transitions.

**Relevance:** Provides a research comparison for explicit emotional,
physiological, and environmental state-transition models.

### 4.3 Closed-Loop Neural-Symbolic Planning

Research on closed-loop neural-symbolic planning examines the integration
of planning, execution, environmental feedback, and subsequent
decision-making [8].

A closed-loop system differs from a static planner because observed
outcomes can influence later decisions.

For an NPC architecture, this supports repeated cycles of:

- State observation.
- Motivation or operator evaluation.
- Planning.
- Execution.
- Feedback.
- State revision.

**Relevance:** Supports a recurrent agent architecture in which
deterministic execution feeds back into learned decision mechanisms.

### 4.4 Task-Directed Neuro-Symbolic AI

The IJCAI 2025 survey *Neuro-Symbolic Artificial Intelligence: A
Task-Directed Survey* examines neuro-symbolic approaches in relation to the
tasks they address [9].

This perspective is useful because different reasoning problems may
benefit from different combinations of symbolic and learned computation.

A system need not use one universal neural-symbolic integration strategy
for every task.

**Relevance:** Supports modular, task-specific reasoning procedures rather
than a monolithic intelligence model.

### 4.5 Neuro-Symbolic Agentic AI

The 2026 survey *Neuro-symbolic agentic AI: Architectures, integration
patterns, applications, open challenges and future research directions*
reviews recent approaches to integrating symbolic and neural components in
autonomous agents [10].

The surveyed research highlights challenges involving integration,
explainability, reproducibility, generalisation, and metacognitive
control.

These challenges are relevant to architectures that delegate reasoning to
specialised algorithms.

**Relevance:** Provides a broader research context for modular
neuro-symbolic agents and higher-level reasoning orchestration.

## 5. Proposed Research Architecture

The literature suggests several mechanisms that can be investigated within
a lightweight, deterministic-first architecture.

The following is a proposed engineering synthesis, not an architecture
established or validated by the cited publications.

```text
Character / Problem State
           |
           v
Symbolic Working Memory
           |
           v
Applicable Operator Identification
           |
           v
Neural Preference Evaluation
           |
           v
Subroutine Selection
           |
           v
Delegated Computation
  - Epistemic reasoning
  - PETIR numerical methods
  - petgraph graph algorithms
  - GOAP planning
  - SAGE pathfinding
           |
           v
Typed Result
           |
           v
State and Belief Updates
           |
           v
Next Decision Cycle
```

### 5.1 Symbolic representation

The architecture will investigate finite, typed representations of
concepts, relations, operators, and reasoning requests.

Enums, structured expressions, and graphs can provide compact
representations suitable for Rust and WASM.

The principal challenge is ensuring that the representation remains
expressive enough for the intended domain.

### 5.2 Neural operator preferences

Burn will provide the experimental neural-learning component.

Rather than generating arbitrary reasoning sequences, a small model may
estimate preferences over a finite set of applicable subroutines.

The proposed selection process is:

$$
o_t = \underset{o \in O_{\mathrm{valid}}(s_t)}{\operatorname{argmax}}\, U_\theta(o, s_t)
$$

Where:

- $s_t$ is the current cognitive state.
- $O_{\mathrm{valid}}(s_t)$ is the set of applicable operators.
- $U_\theta$ is a learned operator-preference function.
- $o_t$ is the selected operator.

This design separates symbolic applicability from learned preference.

### 5.3 Delegated computation

The selected operator invokes a specialised algorithm through a typed
interface.

Scientific numerical operations may be delegated to PETIR, graph
operations to petgraph, planning to GOAP, and game-specific algorithms to
the Rust SAGE port.

The initial selector will choose one subroutine per invocation.

Recursive reasoning and multi-subroutine composition are reserved for a
separate higher-level orchestration system.

### 5.4 Cognitive state and motivation

The broader NPC architecture distinguishes:

- Personality and preferences.
- Emotional and physiological resources.
- Beliefs and memories.
- Motivations.
- Goals and commitments.
- Selected actions.

Neural motivation and operator selection may be implemented as separate
models.

The architecture should not assume that every cognitive subsystem requires
neural inference.

## 6. Research Gaps and Evaluation

The literature establishes substantial precedents for symbolic cognition,
learned operator preferences, and hierarchical planning.

The proposed contribution would therefore need to be demonstrated
empirically rather than assumed from architectural novelty.

### 6.1 Computational efficiency

Evaluate whether compact neural models provide meaningful improvements over
deterministic preference rules.

Measurements should include:

- Inference latency.
- Planning latency.
- Peak memory consumption.
- Model storage.
- Number of neural invocations.
- Energy consumption where measurable.

### 6.2 Decision quality

Compare the following configurations:

| Configuration | Symbolic reasoning | Neural selection |
|---|---|---|
| Deterministic baseline | Yes | No |
| Soar-style preference selection | Yes | No |
| Standalone neural policy | No dedicated symbolic selector | Yes |
| Proposed hybrid | Yes | Yes |

Evaluation should include task success, invalid decisions, generalisation,
and recovery from unexpected events.

### 6.3 Explainability and reproducibility

The architecture should record:

- Applicable operators.
- Selected operator.
- Learned preference scores.
- Invoked algorithm.
- Input assumptions.
- Returned result.
- Subsequent state changes.

Deterministic replay should be supported under documented numerical and
execution conditions.

### 6.4 Expressiveness

A finite semantic vocabulary and constrained operator set can substantially
reduce computational complexity.

However, these constraints may prevent some valid problems from being
represented.

Evaluation should distinguish intended domain restrictions from unintended
representational limitations.

### 6.5 Learning and generalisation

Small neural models may be effective for narrowly defined decisions, but
their representational capacity is limited.

Training should therefore focus on well-specified tasks with measurable
outcomes.

Potential training approaches include supervised operator-selection
examples, reinforcement learning from simulation outcomes, and preference
learning from expert demonstrations.

## 7. Conclusions

Existing research provides strong precedents for combining symbolic
cognitive architectures with learned numerical decision mechanisms.

Soar contributes operator selection, impasse resolution, and chunking.
ACT-R contributes quantitative procedural utility. CLARION contributes
explicit–implicit cognitive integration and motivational subsystems. GOAP
contributes deterministic planning over structured actions.

Contemporary neuro-symbolic research extends these ideas through learned
policies, structured world models, hierarchical planning, and
feedback-driven decision-making.

The proposed Rust/WASM intelligence engine would investigate a specialised
implementation combining:

1. Compact typed semantic representations.
2. Symbolic applicability and reasoning.
3. Learned operator preferences.
4. Deterministic delegated algorithms.
5. Hierarchical goal planning.
6. Explicit state transitions and verification.

The primary research question is whether such a modular architecture can
achieve useful decision-making performance with substantially lower memory
and computational requirements than more general neural systems.

---

## Bibliography

### Foundational cognitive architectures

[1] Laird, J. E., Newell, A., & Rosenbloom, P. S. (1987). Soar: An
architecture for general intelligence. *Artificial Intelligence*, 33(1),
1–64. <https://doi.org/10.1016/0004-3702(87)90050-6>

[2] Laird, J. E. (2012). *The Soar Cognitive Architecture*. MIT Press.
<https://mitpress.mit.edu/9780262122962/the-soar-cognitive-architecture/>

[3] Nason, S., & Laird, J. E. (2005). Soar-RL: Integrating reinforcement
learning with Soar. *Cognitive Systems Research*, 6(1), 51–59.
<https://doi.org/10.1016/j.cogsys.2004.09.006>

[4] Anderson, J. R., Bothell, D., Byrne, M. D., Douglass, S., Lebiere, C.,
& Qin, Y. (2004). An integrated theory of the mind. *Psychological Review*,
111(4), 1036–1060. <https://doi.org/10.1037/0033-295X.111.4.1036>

[5] Sun, R. (2006). The CLARION cognitive architecture: Extending cognitive
modeling to social simulation. In R. Sun (Ed.), *Cognition and Multi-Agent
Interaction: From Cognitive Modeling to Social Simulation*. Cambridge
University Press. <https://doi.org/10.1017/CBO9780511610721>

### Contemporary neuro-symbolic research

[6] Hierarchical Neuro-Symbolic Decision Transformer. (2025). *Proceedings
of Machine Learning Research*.
<https://proceedings.mlr.press/v284/baheri25a.html>

[7] Neurosymbolic World Models for Sequential Decision Making. (2025).
*Proceedings of the 42nd International Conference on Machine Learning*.
<https://proceedings.mlr.press/v267/hernandez-cano25a.html>

[8] Closed-loop neural-symbolic planning. (2025). *Knowledge-Based Systems*.
<https://www.sciencedirect.com/science/article/abs/pii/S095070512501086X>

[9] Neuro-Symbolic Artificial Intelligence: A Task-Directed Survey. (2025).
*Proceedings of IJCAI 2025*. <https://www.ijcai.org/proceedings/2025/1157>

[10] Neuro-symbolic agentic AI: Architectures, integration patterns,
applications, open challenges and future research directions. (2026).
<https://www.sciencedirect.com/science/article/pii/S1574013726000110>

### Additional relevant literature

[11] Talevich, J. R., Read, S. J., Walsh, D. A., Iyer, R., & Chopra, G.
(2017). Toward a comprehensive taxonomy of human motives. *PLOS ONE*,
12(2), e0172279. <https://doi.org/10.1371/journal.pone.0172279>

[12] Reiss, S. (2004). Multifaceted nature of intrinsic motivation: The
theory of 16 basic desires. *Review of General Psychology*, 8(3), 179–193.
<https://doi.org/10.1037/1089-2680.8.3.179>

[13] Enabling High-Level Machine Reasoning with Cognitive Neuro-Symbolic
Systems. (2023). arXiv:2311.07759. <https://arxiv.org/abs/2311.07759>

---

### Bibliographic verification note

References [1]–[5] and [11]–[12] include established bibliographic
metadata. References [6]–[10] and [13] retain the identified publication
links, but their complete author lists and final publication metadata
should be checked against the primary records before scholarly submission.
The CLARION chapter citation should also be checked for its precise chapter
title and pagination.

This review is a preliminary literature synthesis, not a systematic review
or a claim of exhaustive coverage.
