# Causal Reinforcement Learning for Complex Card Games: A Magic The Gathering Benchmark

**PDF:** `causal-rl-mtg-benchmark.pdf`
**arXiv:** 2605.06066 — https://arxiv.org/abs/2605.06066 (preprint, ~May 2026)
**Authors:** Cristiano da Costa Cunha, Wei Liu, Tim French, Ajmal Mian (U. Western Australia)

## What it's about

A **gameplay benchmark for causal RL** — the most relevant of the MTG papers for the *playing* problem because it's an actual working Gymnasium MTG environment with a PPO loop. The gap it targets: no existing env combines sequential decisions + structural causal variables + hidden info + masked actions.

## Why MTG is hard (its framing)

Partial observability + stochastic draws; **478 discrete actions (only 2–15 legal per state)**; 10–20 sequential decisions per turn; layered causality (mana→spells→board→win); Turing-completeness.

## Method

Hand-designed **Structural Causal Model (SCM)** over 4 layers (resources→board→strategic position→outcome); WinProb = sigmoid over 6 parents (CardAdv, BoardPress, Tempo, LifeBuffer, ThreatDensity, RemovalAvail) with Pearl `do(·)` interventions.

RL backbone: **masked PPO** (Stable-Baselines3, GAE, LR 3e-4→1e-5). Novel agent **CGFA-PPO** adds per-factor critic heads indexed by SCM parents, a state-conditional residual gate (scalar vs factored advantage), and a Pearson-correlation calibration loss aligning learned advantages with SCM intervention effects. Baselines: random, heuristic, vanilla PPO, Causal-World-Model. Gymnasium interface, 3,077-dim flat observation, three reward schemes.

## Scope

Deliberately tiny: **56 unique Standard-2025 cards, five fixed 60-card archetype decks** (Mono-Red Aggro, Azorius Control, Dimir Midrange, Domain Ramp, Boros Convoke). 20 life, 7-card hand.

## Key results (verified)

No agent dominates. CGFA-PPO vs PPO: Azorius 25.6 vs 20.6 (+5.0), Mono-Red 70.6 vs 67.8 (+2.8), but Boros 53.3 vs 57.2, Dimir 50.0 vs 52.2, **Domain Ramp 12.2 vs 20.6 (−8.3)**. → **wins 2/5 archetypes, loses 3/5.** Transfer (leave-one-out): PPO 76.1 in-dist / 80.8 held-out; CGFA-PPO 72.6 / 76.8. Random ~5–10%. Cost: 10^6 env steps/opponent, full pre-registered stat protocol (paired bootstrap CIs, Wilcoxon, Holm-Bonferroni). At n_seeds=2, p-values "exploratory, not confirmatory."

## Limits the authors admit

SCM hand-designed (no structure discovery); 56-card/5-deck pool tiny; opponents fixed/non-adaptive; **causal method does not yet show a uniform win-rate gain over PPO.**

## Imperfect info / action space

Large action space → **action masking** (478→2–15 legal). Partial observability → observation/reward restricted to agent-observable quantities (no leakage). But it does **not** do belief-state inference / opponent-hand modeling — **hidden info is simply omitted**, not solved.

## Relevance to us

Most relevant for the gameplay side: a real (tiny) Gymnasium MTG env + masked PPO baseline, confirming **action-masking** as the practical answer to the 478-action space. Takeaways: the env design, observation encoding, masking, and the rigorous statistical-eval protocol. **Not** the causal machinery (unproven, mixed results) and not the scope (too small, fixed opponents, hidden info sidestepped).
