# Imperfect-Information Game Algorithms — the AI toolbox

The general (non-MTG) algorithm literature your friend would live in. Ordered from "realistic first bot" to "north star."

Local PDFs: `deep-cfr.pdf`, `depth-limited-solving.pdf`, `self-play-4player-imperfect.pdf`.

---

## ISMCTS / per-player-tree MCTS — the realistic FIRST bot

**Reference:** Heinrich & Silver, "Self-play MCTS in Computer Poker"; ISMCTS (Cowling et al.). Related demo: MDPI *Electronics* 12(11):2453 (2023), a **Mahjong** agent combining tree search + deep net + dropout.

**Idea:** Build the search tree over **information sets**, not fully-determined states. Each iteration: **determinize** (sample a concrete hidden state consistent with observations), run one MCTS iteration, aggregate stats at the infoset level across many determinizations. **Separate per-player trees** (Heinrich & Silver) index each tree by that player's own infosets so neither tree encodes the other's private cards.

**Problem it solves:** "Determinization cheating" / information leaking — naive MCTS on one sampled full state plans as if it knew the opponent's hidden cards.

**Requirements:** a forward simulator + a way to sample hidden states consistent with observations (belief sampling). Works for multiplayer.

**MTG relevance:** **The most directly implementable family near-term.** Needs only a fast simulator + belief sampling, not an equilibrium solver. How most card-game bots (Hearthstone, Big-2) are built. Hard parts for MTG: belief sampling means sampling plausible opponent *decklists + hands* conditioned on what's been played/mana/metagame. **Known weakness: strategy fusion / non-locality — vanilla ISMCTS can't represent genuine mixed/bluffing strategies and is exploitable.** Good first strong bot; not a path to equilibrium.

---

## Deep Counterfactual Regret Minimization (Deep CFR)

**Authors:** Brown, Lerer, Gross, Sandholm — ICML 2019 (arXiv 1811.00164).

**Idea:** CFR iteratively traverses the whole game tree; at each information set it accumulates **counterfactual regret** per action ("how much better had I always taken action a here, weighted by reach probability"). Averaging strategies provably converges to **Nash in 2p0s**. Tabular CFR needs one entry per infoset (infeasible at scale) and historically needed hand-built abstractions. **Deep CFR replaces the tables with neural nets** — a value net predicts the regrets, a policy net approximates the average strategy; data via external-sampling MC traversals + reservoir buffers. "CFR with function approximation."

**Problem it solves:** Reasons over infosets → robust to opponent exploitation; removes hand-crafted abstraction.

**Demonstrated:** heads-up limit / no-limit-flop hold'em; beats NFSP. Proof of principle, not Libratus-scale.

**Requirements:** 2p0s, perfect recall, a game model (it traverses the tree).

**MTG relevance:** 1v1 constructed *is* effectively 2p0s → decent fit. **Killer problem: tree traversal.** MTG's branching (priority, stack, targets, modal, X) + depth + pervasive chance nodes make full/sampled traversal impractical (authors flag external-sampling MCCFR struggles with extreme branching). Conceptually the right *equilibrium engine*, but raw Deep CFR doesn't scale to MTG without depth-limiting + abstraction.

---

## Depth-Limited Solving for Imperfect-Information Games

**Authors:** Brown, Sandholm, Amos — NeurIPS 2018 (arXiv 1805.08195).

**Idea:** In perfect-info games you can stop at depth d and substitute a heuristic value (AlphaZero/minimax). In imperfect-info this **fails** — a state has no single well-defined value; it depends on your belief distribution AND the opponent's whole adaptable strategy. Fix: at the depth limit, let the **opponent choose among several continuation strategies**; make your search robust to their best choice. With enough strategies offered, the depth-limited solution is part of a true equilibrium. <10 strategies already gives strong play.

**Problem it solves:** the "states don't have values" pathology — makes cheap real-time subgame solving feasible.

**Demonstrated:** "Modicum" — beat top prior HUNL bots on a **4-core CPU / 16GB**, work that previously needed a supercomputer. The technique that made strong poker AI cheap.

**Requirements:** 2p0s; a blueprint strategy as value scaffold; action abstraction; nested solving as play descends.

**MTG relevance:** the conceptual unlock for running anything CFR-like on MTG — you *must* depth-limit, and this is the principled way without the search cheating on hidden info. "Multiple opponent continuation strategies" maps onto "is my opponent on the aggro/control/combo line." Hard parts: needs a blueprint (big training effort); MTG actions don't bucket as cleanly as poker bet sizes; leaf-value model must encode beliefs over opponent hidden cards (huge structured space). A **required building block**, absorbed into Student of Games.

---

## Student of Games (a.k.a. "Player of Games") — the NORTH STAR

**Authors:** Schmid, Moravčík, Burch, et al. (DeepMind). arXiv 2112.03178 (2021); *Science Advances* 2023.

**Idea:** the clean unification of AlphaZero-style guided search with CFR-style game theory — **one algorithm for both perfect- and imperfect-info games.** Three pieces:
- **GT-CFR (Growing-Tree CFR):** grow a local subgame tree on the fly (MCTS-like expansion) and run CFR within it — alternating regret-update and expansion. Depth-limited, sound subgame solving.
- **CVPN (Counterfactual Value-and-Policy Network):** AlphaZero's value/policy net analogue, but outputs **counterfactual values per information state given a public belief state** — values conditioned on the distribution over hidden states, not a single scalar. Exactly what avoids "states have no value."
- **Sound search:** proven to converge to Nash as compute/capacity grow — inherits CFR's soundness, not MCTS's heuristic unsoundness in imperfect info.

**Demonstrated:** strong **chess & Go** (perfect info), beats **Slumbot** at HUNL poker, beats SOTA at **Scotland Yard** — *same algorithm, four games.* That breadth is the headline.

**Requirements:** 2p0s; perfect recall; a model/simulator; and crucially operates over **public belief states** — requires hidden info to factorize into a tractable public-state + per-player private-info structure.

### Why it's the cleanest north star for MTG

Only one of the four that is simultaneously (a) **sound/equilibrium-convergent** (won't be trivially bluff-exploited like ISMCTS), (b) **uses learned search** so it scales via neural guidance (not Deep CFR's full enumeration), and (c) **general** — proven on both info regimes, which matters because MTG mixes perfect-info board state with deep imperfect-info hands/libraries. It absorbs the other three as components.

### What makes MTG harder than chess/Go/poker/Scotland Yard

1. **Public belief state is the crux, and it barely holds in MTG.** SoG assumes hidden info factorizes into a compact public state + small enumerable private info (poker: "which 2 cards"). In MTG the hidden info is *the opponent's whole hand + the order of both libraries*, conditioned on an unknown 60-card decklist from a vast pool. Belief space is astronomically larger and not cleanly factorizable. **The single biggest obstacle.**
2. **Action space:** enormous, structured, state-dependent (priority/stack/targets/modal/X) vs fold-call-raise.
3. **No fixed-size board / no clean encoding:** variable-length relational state → needs a card-text-aware encoder (Paper 1).
4. **Pervasive high-variance stochasticity:** every draw/shuffle is a chance node → far more samples than poker.
5. **Not strictly 2p0s** in multiplayer/Commander; only 1v1 constructed gets the guarantees.
6. **Needs a complete, fast rules simulator** — itself a major project (poker/Scotland Yard simulators are trivial by comparison).

---

## DeepNash (Stratego) — the FALLBACK worldview

**Authors:** Perolat et al. (DeepMind). arXiv 2206.15378 (2022); *Science* 2022.

**Idea:** **not search-based, not CFR.** Uses **R-NaD (Regularized Nash Dynamics)** — model-free RL that modifies the learning dynamics (reward transform + v-trace-style value + Neural Replicator Dynamics policy update) so self-play converges to approximate Nash **without any tree search**.

**Result:** top-3 human on Gravon Stratego, ~84% vs strong humans.

**MTG relevance:** Stratego's tree (~10^535 states) is too big for search/CFR-subgame approaches — DeepNash shows the alternative: pure model-free equilibrium-seeking RL. If MTG's combinatorial explosion likewise defeats search-based methods, an R-NaD-style route is the fallback worth keeping in mind.

---

## Practical ladder for us

1. **ISMCTS + per-player trees** → realistic first strong MTG bot (needs fast sim + belief sampling).
2. **Depth-Limited Solving + Deep CFR** → the soundness/equilibrium components.
3. **Student of Games** → the architecture to aim at; reaching it hinges on (a) belief-state representation and (b) a fast full-rules simulator — neither of which any of these papers faced at MTG's scale.
4. **DeepNash / R-NaD** → fallback if the tree is simply too big to search.
