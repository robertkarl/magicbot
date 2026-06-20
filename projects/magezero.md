# MageZero

**Repo:** https://github.com/WillWroble/MageZero
**Tagline:** "AlphaZero-Style RL for Magic: The Gathering"
**Built on:** a fork of **XMage** (the ~28k-card Java engine)
**Status:** v0.1.0-alpha (April 2026); **active** — last commit ~June 19 2026; ~75 commits, MIT.
**This is the most serious, most active, and most relevant of the existing projects. Read it first.**

## What it actually is

Not a single universal MTG agent. It trains **deck-specific agents** — each deck/matchup treated as a separate tractable subgame. (See `../overview.md` for the "why deck-specific vs. one general model" discussion — it's a tractability shortcut, not a claim that deck-specific is more powerful.)

## Architecture

Hybrid Java + Python:
- **Engine:** forked XMage (Java) with added RL modules `Player.AI.RL`, `Player.AI.MCTS`, `Mage.MageZero`. State captured in Java via `StateEncoder.java`.
- **Orchestration/training:** Python `mz` CLI — self-play data gen, dataset analysis, eval, replay buffer, multi-generation training.
- Java engine produces encoded states + runs MCTS rollouts; Python does NN training/inference.

## The AI (true AlphaZero/MCTS + RL self-play)

- **State encoding:** dynamic feature hashing into a 2M-dim sparse binary space (Weinberger-style), ~200 active features per state.
- **Network:** single-layer Transformer (d_model=512, 4 heads), 2M×512 sparse embedding, **4 policy heads** (player priority, opponent priority, targets, binary decisions) + 1 value head.
- **Search:** MCTS with **PUCT (c=1.0)**, no Dirichlet noise, TD-λ value targets (0.9–0.95), opponent-modeling visit-count blend.
- **Reported result (Dec 2025):** a UW Tempo deck went **16% → 66% win rate** vs minimax opponents; discovered non-obvious play.

## Imperfect information

Acknowledged open problem. Current handling is **forward-simulation + opponent-modeling via predicted visit counts** rather than perfect-information cheating. Roadmap: **MuZero-style learned dynamics** to model hidden hands/libraries. So: not flat-out cheating, but immature.

## Throughput — the key weakness (verified numbers)

- **~250 games/hour** local (13 CPU threads, 300-sim MCTS budget).
- Single-thread ~150 MCTS sims/sec; **8-thread only ~75 sims/sec (negative scaling — heap-limited)**.
- NN inferences ~100/sec.
- Their *future goal* is merely ~1,000 games/hour — i.e. **they themselves consider current speed inadequate** and list a fast sim framework as future work.

This negative threading scaling is the single most important empirical datapoint in the whole repo. It is direct evidence that XMage's object-heavy, GC-bound design is the bottleneck — and the core argument for a Rust/C++ rewrite (`../overview.md`).

## As a training-grade engine

- **Strong:** correctness (inherits XMage's full rules engine); RL scaffolding already wired (encoder, MCTS, policy/value heads, self-play loop).
- **Weak:** speed; threading scales *negatively*; no documented fast clone/restore or determinism guarantee.

## Verdict

Best starting point of the existing projects if you want AlphaZero-on-MTG with working RL code **today** — but **not** if raw self-play speed is your primary requirement. You inherit XMage's throughput ceiling, which is the exact unsolved problem. Use it as the reference implementation and the thing to beat.
