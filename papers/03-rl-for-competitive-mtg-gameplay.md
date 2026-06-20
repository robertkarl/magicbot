# Reinforcement Learning for Competitive Magic: The Gathering Gameplay

**PDF source:** https://www.reinforcement-learning-for-mtg.com/downloads/MTG_with_RL.pdf
**Listed authors:** Alex Thaikanov (MIT), Blair Probst (Stanford), Casey Tzao (CMU), Dana Evelyn (Oxford). "Preprint. Under review."

> ⚠️ **Strong caveat: this is a blueprint/template document, not an empirical result.** Contact emails are `{...}@example.edu`, the names are generic, and the results tables are literally labeled *"fill with your results."* Treat authorship/venue as unverified and possibly synthetic. Value it as an **architecture checklist**, not as evidence anything works.

## What it's about

A *proposed* recipe for a full competitive MTG agent. No code, no numbers — but the most directly-aligned blueprint for the exact engine-plus-imperfect-info-agent goal.

## Why MTG is hard (its framing — the cleanest list of the three papers)

Long-horizon, stochastic, partially observable, adversarial, combinatorial action space, rules "that rival programming languages in expressivity." Named obstacles: priority passing + LIFO stack + replacement/triggered effects; hidden hands + random draws; combinatorial decision nodes (attacker/blocker subsets, stacked responses); non-stationary metagame from rotation.

## Proposed method (three parts — proposed, not implemented)

1. **Rules-faithful simulator with two-level hierarchical action abstraction:** macro-intents ("Develop board", "Hold up interaction") → grammar-constrained parameterized primitives `Cast(CardID, Mode, Targets, CostChoice)`; candidate generation + intent-conditioned pruning for combinatorial attack/block subsets.
2. **Model-based self-play learner (MuZero spirit):** representation/dynamics/prediction nets; **MCTS with UCT in latent space**; GRU recurrence for partial observability; **opponent-hand sampling from a learned generative model** calibrated to decklists.
3. **Population-based co-evolutionary outer loop:** jointly optimize 60-card decks (evolutionary / Bayesian search over card-frequency embeddings) against a population of historical checkpoints + archetypes. Plus curriculum (creatures → stack → full sets) and potential-based reward shaping.

References MuZero, AlphaZero, DeepStack, Libratus/Pluribus, OpenAI Five, PBT.

## Results

**None.** Tables explicitly say "fill with your results." It proposes an eval protocol (ELO cross-play, matchup matrix, generalization to unseen decks/rotations, data-efficiency curves, ablations) but reports zero numbers.

## Imperfect info / action space (proposed)

Imperfect info → GRU belief-augmented latent + **generative opponent-hand sampling**. Action space → hierarchical macro-intent abstraction + pruning + a **pointer network for target selection**. Design proposals, unvalidated.

## Relevance to us

Despite no results, the **best architecture checklist** for the goal. Highest-value ideas to mine:
1. **Hierarchical action abstraction** over priority/stack to tame the action space.
2. **Generative opponent-hand sampling + recurrent belief state** as a concrete imperfect-info approach (vs Paper 2 which omits hidden info) — closest to DeepStack/CFR-style thinking.
3. **MuZero-style latent search** if a fast forward model exists.
4. **Population/co-evolution** to avoid self-play exploitability.
5. State encoding: permutation-invariant battlefield multiset, ordered stack encoder, learned card-identity embeddings (dovetails with Paper 1).
