# Overview & synthesis

The synthesis across everything in `papers/` and `projects/`, plus the two live design questions.

## State of the world (June 2026)

- A *playing* MTG AI exists only in narrow forms: Forge's **heuristic** bot; XMage's `ComputerPlayerMCTS`; and the new, active **MageZero** (AlphaZero-on-XMage, deck-specific, ~250 games/hr).
- Academic MTG work is thin and immature: Paper 1 is drafting-only; Paper 2 is a 56-card benchmark that *omits* hidden info; Paper 3 is a results-free blueprint.
- **Two blockers, both unsolved:** (1) no engine is fast/forkable enough for self-play; (2) nobody has cracked belief-state reasoning over MTG's hidden-info space.
- **The gap is real and wide open.** This is a legitimate project, not a crowded field.

## The natural two-person split

- **Engine (you):** the differentiated contribution is *not* "an MTG engine" (XMage/Forge exist) — it's a **training-grade** engine: headless, **fast**, **forkable state** (`clone`/`restore`), deterministic, clean `(observation, legal_actions) → action` + `step()` API, with **determinization support** (instantiate concrete hidden states from a belief distribution *without leaking the true state into the agent*).
- **AI (friend):** ISMCTS → Deep CFR / Depth-Limited Solving → Student of Games, over a generalised card representation. Thesis-shaped; "Player of Games for Magic" is genuinely unclaimed.

The **interface is the whole project**. Define `clone/step/observe/determinize` early, agree how hidden info is exposed, and you can work in parallel. (→ `engine-api.md`, todo.)

## Scoping move that makes this finishable

Pick **one frozen Pauper mirror matchup** (or a small custom cube). ~100 cards correct, sane action space, still fully imperfect-info-hard. Prove the loop end-to-end, then widen.

---

# Live question 1 — Port the Java engine to Rust for ~10× throughput?

**Your instinct: XMage tops out ~250 games/hr with *negative* thread scaling; the JVM/GC/object-churn overhead looks optional; rewrite in Rust and get 10×.**

This is the highest-leverage *systems* bet on the board, and the evidence partly backs it — but let's separate the part that's real from the part that's vibes (your own words).

## What's real (not vibes)

- **The negative thread scaling is a measured, damning datapoint.** MageZero: ~150 sims/sec single-thread → **~75 sims/sec on 8 threads.** Going *backwards* with more cores is the signature of GC pressure and/or shared-mutable contention and allocation churn. That is exactly the failure mode a value-typed, arena/pool-allocated Rust rewrite erases. A language with no GC, cache-friendly structs, and cheap `clone()` of a flat state struct is *designed* for "fork millions of game states fast."
- **Self-play fundamentally needs cheap `clone`/`restore` of game state.** In Java each state is a graph of heap objects; cloning it is allocation-heavy and the copies thrash the GC. In Rust you can lay the whole game state out as a flat, `Copy`-friendly (or `Clone`-cheap) struct / struct-of-arrays and snapshot it with a memcpy. **This single property — cheap state snapshotting — is plausibly worth more than the raw per-instruction speedup.**
- **So the *direction* is right:** the bottleneck for self-play is state-copy cost + parallelism, and Rust attacks both directly.

## What's vibes (where I'd make you slow down)

- **"10×" is a guess, not a measurement.** Raw compute language speedups (Java→Rust) are often more like 2–5× for branchy, pointer-chasing logic — and an MTG rules engine *is* branchy pointer-chasing logic, not a tight numeric loop where Rust shines most. The 10× (or more) almost certainly comes from the **allocation/GC/clone** axis, **not** from "JVM instructions are slow." So the headline number is real *if and only if* you architect for cheap state copies — it is **not** free from "Rust is faster."
- **The JVM is not as dumb as "all that stack-frame overhead" implies.** HotSpot JITs hot paths to native, inlines aggressively, and does escape analysis. A naive Rust port that keeps XMage's object-graph-per-state design could easily be *no faster* — you'd have moved the GC problem into manual `Rc`/`Box` churn. **The win is the data-layout rewrite, which Rust forces you to do; it is not the language per se.**
- **The cost is enormous and gets understated.** XMage encodes **~28,000 cards' worth of hand-written rules interactions.** That is the real asset. A Rust rewrite re-pays that entire cost unless you keep coverage tiny. This is why the **frozen-small-format scoping is not optional for the rewrite path** — it's what makes a Rust engine a one-year project instead of a ten-year one. ~100 cards correct in fast Rust beats 28,000 cards correct in slow Java *for self-play research*.

## The honest recommendation

**The Rust port is the right long-term systems bet, but for the wrong stated reason.** It's not "Java will fuck you on instruction overhead" — it's **"self-play lives or dies on cheap state cloning + real parallelism, and a from-scratch value-typed engine is the clean way to get both."** Rust is the natural vehicle (no GC, `Clone` you control, fearless threads, SoA layouts), but C++ or even a carefully arena-allocated rewrite would capture most of it.

**Decision rule:** don't rewrite to chase a language multiplier. Rewrite because you want a **flat, snapshot-cheap, deterministic state representation over a small frozen card pool** — and Rust is the best tool for *that*. Validate first by measuring MageZero's actual sims/sec and where its time goes (GC logs / profiler). If >50% is GC + allocation, the rewrite thesis is confirmed empirically rather than by vibes.

**Suggested first experiment:** build a Rust engine for *one* Pauper matchup (~80–120 cards), expose `clone/step/observe/determinize`, and benchmark games/sec vs MageZero on the same matchup. If you don't clear ~10–50× on *that*, the thesis is wrong and you should fork XMage instead.

---

# Live question 2 — Why is MageZero deck-specific? Wouldn't one model that knows all cards/archetypes be more powerful?

**Your instinct is correct about the end goal.** A single model that understands all cards and archetypes is strictly more powerful and is what "solving MTG play" actually means. Deck-specific is **not a claim that per-deck is better** — it's a tractability shortcut. Here's why a solo/small project reaches for it anyway:

## Why deck-specific is the pragmatic starting point

1. **It shrinks every hard dimension at once.** Fix both decklists and you collapse: the action space (only your ~15–40 distinct cards' actions exist), the belief space (you *know* the opponent's 60-card list, so hidden info reduces to "which known cards are in hand / library order" — enumerable!), and the state encoding (no need to generalise to unseen cards). The astronomically-large belief problem that breaks Student of Games (`papers/04`) becomes almost tractable when the decklist is known.
2. **It sidesteps the generalisation research problem entirely.** A general agent needs Paper 1's generalised card representations + transfer to unseen cards — an *open research problem*. Deck-specific needs none of it; the card vocabulary is closed and tiny.
3. **It makes self-play converge.** Self-play on a fixed matchup is a stationary 2p0s game with clean equilibrium semantics. Self-play across all decks is non-stationary and the metagame moves under you.
4. **It's a publishable/demonstrable unit.** "16% → 66% on UW Tempo" is a clean result. "We trained one model on all of Magic" is a moonshot with a high chance of mush.

## Why the general model is still the real prize (and where the power is)

You're right that the *power* is in generality, for concrete reasons:
- **Transfer / shared structure:** removal is removal, tempo is tempo, a counterspell threatens the stack the same way across decks. A general model amortizes these concepts; deck-specific models relearn them from scratch every time. This is exactly Paper 1's thesis (generalised card representations) applied to *play* instead of *draft*.
- **Opponent generality:** a deck-specific agent only knows *its* matchup. Real play requires reasoning about an *unknown* opponent deck — which is precisely the hard belief-state problem, and a per-matchup model is defined to ignore it.
- **The interesting AI lives in the general version.** Belief over unknown decklists, generalisation to new sets, archetype-level abstraction — that's the thesis-shaped frontier. Deck-specific is the warm-up.

## The synthesis (how to use both)

Deck-specific is **rung 1 of a ladder, not the destination**:
1. **Deck-specific, known matchup** (MageZero today) — prove the engine + self-play loop work; belief space is enumerable.
2. **Known-archetype pool** — generalise across a fixed set of decks; introduce real opponent-deck uncertainty (now you need belief over *which archetype*).
3. **Generalised card representation** (Paper 1 front-end) — drop the closed vocabulary; transfer to unseen cards/sets.
4. **General agent** — full belief over unknown 60-card lists from the open pool. The actual prize.

So: MageZero isn't wrong, it's on rung 1. Your friend's thesis is the climb from rung 2 to rung 4 — and that climb is where all the unclaimed research is. **Build the engine so it doesn't bake in the deck-specific assumption** (i.e. `determinize` should sample from a *belief over decklists*, with "known decklist" as the degenerate case) and you keep the whole ladder open.

---

# The one concrete next move

**Measure MageZero's actual games/sec and profile where the time goes** on a single matchup. That number decides everything:
- If it's GC/allocation-bound → the Rust rewrite thesis is confirmed; that's your half.
- If it's NN-inference-bound → the engine isn't the bottleneck and the rewrite is premature; focus on the agent.

Everything downstream (rewrite or not, deck-specific or general) hinges on that one measurement. Turn the bar conversation into a profiler run.
