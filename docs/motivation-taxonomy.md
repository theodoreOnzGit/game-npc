# Motivation taxonomy v0.1: definitions and provenance

Status: **draft for review** (GitHub issue #1). This file covers the
"document the provenance of each proposed motivation" and "identify overlaps,
omissions and ambiguous categories" tasks. It is not yet complete.

## What has and has not been checked

- **Talevich et al. (2017)**, the primary reference, has **not** yet been
  compared against this list. The full text and its 161-motive hierarchy were
  not consulted when this draft was written, so the "Talevich" column below is
  empty on purpose. Filling it in is the next review step.
- The **Reiss**, **SDT** and **Fundamental Motives** columns are *candidate*
  correspondences written from general knowledge of those frameworks. They
  have **not** been checked against the papers cited in the issue. Treat each
  one as a hypothesis to verify, not as a citation.
- "Partial" means the two concepts overlap but are not the same thing.
- The 25 motivations are the issue's engineering synthesis, implemented as
  listed. The issue's text says 24, but its table and enum list 25 (see
  "Count discrepancy" below).

References:

- Talevich, J. R., Read, S. J., Walsh, D. A., Iyer, R., & Chopra, G. (2017).
  Toward a comprehensive taxonomy of human motives. *PLOS ONE* 12(2):
  e0172279. <https://doi.org/10.1371/journal.pone.0172279> (CC BY 4.0)
- Reiss, S. (2004). Multifaceted nature of intrinsic motivation: The theory of
  16 basic desires. *Review of General Psychology* 8(3): 179–193.
  <https://doi.org/10.1037/1089-2680.8.3.179>
- Deci, E. L., & Ryan, R. M. Self-Determination Theory.
  <https://selfdeterminationtheory.org/theory/>
- Fundamental motives framework, Kenrick and colleagues, as linked in the
  issue: <https://pmc.ncbi.nlm.nih.gov/articles/PMC3161125/>

## Definitions and candidate correspondences

Stable id = the `Motivation::id()` wire value. Its high nibble is the domain.

| Id | Motivation | Working definition | Reiss 16 (candidate) | SDT (candidate) | Fundamental motives (candidate) | Talevich 2017 |
|---|---|---|---|---|---|---|
| 0x00 | Sustenance | Want to obtain food and drink | Eating | — | Immediate physiological needs | not reviewed |
| 0x01 | Rest | Want to recover physical energy | — | — | Immediate physiological needs (partial) | not reviewed |
| 0x02 | Health | Want to avoid or recover from injury and illness | — | — | Disease avoidance (partial)* | not reviewed |
| 0x03 | Safety | Want to avoid physical threat and harm | Tranquility (partial) | — | Self-protection | not reviewed |
| 0x04 | Comfort | Want physically pleasant surroundings | — | — | Immediate physiological needs (partial) | not reviewed |
| 0x10 | Curiosity | Want to understand something specific | Curiosity | Competence (partial) | — | not reviewed |
| 0x11 | Mastery | Want to improve one's competence | — | Competence | — | not reviewed |
| 0x12 | Achievement | Want to complete tasks and visible accomplishments | — | Competence (partial) | Status (partial) | not reviewed |
| 0x13 | Novelty | Want new, varied experience for its own sake | Curiosity (partial) | — | — | not reviewed |
| 0x20 | Connection | Want contact with people in general | Social contact | Relatedness | Affiliation | not reviewed |
| 0x21 | Belonging | Want acceptance as a group member | Acceptance (partial) | Relatedness | Affiliation | not reviewed |
| 0x22 | Affection | Want close relationships with specific individuals | Romance / Family (partial) | Relatedness | Mate acquisition / retention (partial) | not reviewed |
| 0x23 | Recognition | Want status, respect and acknowledgement | Status | — | Status / esteem | not reviewed |
| 0x24 | Caregiving | Want to protect and provide for dependants | Family | Relatedness (partial) | Kin care / parenting | not reviewed |
| 0x30 | Autonomy | Want self-chosen action | Independence | Autonomy | — | not reviewed |
| 0x31 | Control | Want influence over people and events | Power | — | Status (partial) | not reviewed |
| 0x32 | Order | Want predictability and organisation | Order | — | — | not reviewed |
| 0x33 | Acquisition | Want to gain and keep possessions | Saving | — | — | not reviewed |
| 0x40 | Duty | Want to fulfil accepted obligations | Honor (partial) | — | — | not reviewed |
| 0x41 | Justice | Want fairness, and wrongs set right | Idealism (partial), Vengeance (partial) | — | — | not reviewed |
| 0x42 | SelfExpression | Want to express identity and creativity | — | Autonomy (partial) | — | not reviewed |
| 0x43 | Meaning | Want actions to serve a larger purpose | Idealism (partial) | — | — | not reviewed |
| 0x50 | Pleasure | Want enjoyable experiences | — | — | — | not reviewed |
| 0x51 | StressRelief | Want to reduce acute stress | Tranquility (partial) | — | — | not reviewed |
| 0x52 | EmotionalRecovery | Want to restore emotional reserves | — | — | — | not reviewed |

\* Disease avoidance appears in later fundamental-motives work; whether it is
in the specific paper the issue links has not been checked.

## Issues found for the review

### Count discrepancy

The issue says "24 motivations" and "96 bytes per character", but its domain
table and its proposed `enum` both list **25** (5 + 4 + 5 + 4 + 4 + 3). The
code implements the 25 as listed, giving a 100-byte vector. Decision needed:
keep 25, or drop one. EmotionalRecovery is the likeliest candidate (see the
overlaps below).

### Overlaps

- **Curiosity / Novelty.** Both are information- or experience-seeking. Reiss
  has a single Curiosity desire. Here the split is "specific thing to
  understand" against "variety for its own sake".
- **Connection / Belonging / Affection.** All three map to SDT relatedness and
  to Kenrick's affiliation. The split is "anyone" / "a group" / "specific
  individuals", and the planner reflects it (`SocialTarget`).
- **Mastery / Achievement.** Mastery is internal competence, Achievement is
  external accomplishment. Both map to SDT competence.
- **Rest / StressRelief / EmotionalRecovery.** All three restore a depleted
  resource, and in the baseline both EmotionalRecovery and FindPurpose plan
  the same action (`Withdraw`).
- **Autonomy / Control.** Freedom *from* the influence of others, against
  influence *over* others. They are distinct in Reiss (Independence vs
  Power), but they share a trigger (coercion) in the baseline.

### Possible category errors

The issue's own design principle is that motivation is distinct from resource
capacity. Rest, Health, StressRelief and EmotionalRecovery are each the *want*
that corresponds to a resource (energy, health, stress, emotional battery). That
is consistent (the want is separate from the level), but it means these four
add little beyond their resource. They are candidates for merging into a
single "restore resource" motivation that takes a parameter.

### Candidate omissions

These are compared against Reiss's 16 and the fundamental-motives framework
only. Talevich has not been reviewed yet.

- **Physical activity** (Reiss: Physical exercise). Nothing here covers it.
- **Romantic / mate-seeking** (Reiss: Romance; Kenrick: mate acquisition and
  retention). Affection covers it only partially. Whether a game NPC needs it
  is a design call.
- **Vengeance** (Reiss). Partly folded into Justice, which conflates
  "fairness" with "getting even". Those can drive opposite behaviour.

## Baseline need signals (implementation note)

`DeterministicModel::need` gives each motivation a need signal in `[0, 1]`
built from character state and perception. These formulas are **v0.1
engineering heuristics**. They are not derived from the literature and have
not been calibrated against any data. They exist to provide a reproducible
reference baseline and regression tests. When the Burn model is trained, it
should be compared against this baseline, not tuned to agree with it.
