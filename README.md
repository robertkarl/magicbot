# magicbot — notes toward a Magic: The Gathering *playing* AI

A research/scoping repo for building an AI that **plays** Magic: The Gathering (not drafting, not deckbuilding — playing). Engine + imperfect-information self-play agent.

Two-person split:
- **Engine (systems):** a *training-grade* MTG engine — headless, fast, forkable state (`clone`/`restore`), deterministic, clean `(observation, legal_actions) → action` API.
- **AI (research):** search + learning under imperfect information with a nasty combinatorial action space. Thesis-shaped.

## The one-paragraph state of the world (June 2026)

A *playing* MTG AI does exist in narrow forms (Forge's heuristic bot; XMage's `ComputerPlayerMCTS`; the brand-new **MageZero** doing AlphaZero-on-XMage). What does **not** exist is a strong, imperfect-information self-play agent over a real card pool. The two blockers are (1) no engine is fast/forkable enough for self-play — everything tops out around hundreds to ~1,000 games/hour because the mature engines are heavyweight Java, and (2) nobody has solved belief-state reasoning over MTG's astronomically large hidden-information space. The academic MTG-RL work is either drafting-only, a tiny-pool benchmark, or a blueprint with no results. **The gap is real and wide open.**

## Why this is hard (the honest list)

1. **The rules engine is the actual project.** ~250pp Comprehensive Rules, ~30k cards, and every card can rewrite the rules. The stack, priority, layers, replacement effects, state-based actions.
2. **Imperfect information.** Hidden hands + library order. This is poker-shaped, not chess-shaped → **AlphaZero/MCTS is the wrong algorithm shape**; you need ISMCTS / CFR-family / belief states.
3. **Combinatorial, state-dependent action space.** Targets, stack ordering, mana payment, modal choices, X spells.
4. **Non-stationary game.** Rotates every ~3 months; can't amortize a huge training run over a moving target → motivates *generalised card representations*.
5. **No fast simulator exists.** This is the bottleneck, not the AI.

## Algorithm shape (north star)

- **First working bot:** ISMCTS with per-player trees (avoids information leaking). Needs only a forward simulator + belief sampling. Weakest theoretically (strategy fusion / can't bluff) but realistic.
- **Soundness components:** Depth-Limited Solving + Deep CFR.
- **North star:** DeepMind's **Student of Games** (a.k.a. "Player of Games") — unifies AlphaZero-style learned search with CFR soundness over *public belief states*. The crux for MTG: the public-belief-state assumption *barely holds* — opponent hidden info is their whole hand + both library orders conditioned on an unknown 60-card list from a vast pool.
- **Fallback worldview:** if MTG's tree is too big to search at all, DeepNash-style model-free Nash-seeking RL (R-NaD).

## Suggested scoping move

Don't solve all of Magic. Pick **one frozen Pauper (or small custom-cube) mirror matchup**: ~100 cards correct, sane action space, still fully imperfect-info-hard. Prove the self-play loop end-to-end, then widen.

## Contents

- [`papers/`](papers/) — downloaded PDFs + per-paper summaries
- [`projects/`](projects/) — summaries of existing engines/agents (MageZero, LearnForge, Forge, XMage)
- [`overview.md`](overview.md) — the full synthesis and the open question of the Rust port + one-model-vs-deck-specific
- [`engine-api.md`](engine-api.md) — *(todo)* the `clone/step/observe/determinize` interface that separates the two halves

## The two live design questions (see `overview.md`)

1. **Port the heavyweight Java engine (XMage) to Rust/C++ for ~10× throughput?** Plausibly the highest-leverage systems bet — but the JVM may not be the real bottleneck (see analysis).
2. **One model that knows all cards/archetypes, vs. MageZero's deck-specific agents?** One general model is the *right end goal*; deck-specific is a tractability shortcut, not a belief about where the power is.
