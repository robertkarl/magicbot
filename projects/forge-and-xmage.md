# Forge & XMage — the two mature engines

Both are full-rules **Java** MTG engines. Neither was designed as a high-throughput RL training environment — that's the central caveat, and the central opportunity.

## Forge (Card-Forge/forge)

- **Language/arch:** Java; rules engine with cards defined as text scripts (`cardsfolder`). GPL. Very actively maintained.
- **AI:** **Heuristic, not search/ML.** Wiki: *"The AI is not 'trained'. It uses basic rules and can be easy to overcome knowing its weaknesses."* Hardcoded per-decision heuristics with special-cased logic for iconic cards. OK at aggro/midrange, weak at combo.
- **Headless AI-vs-AI:** Yes — first-class **`sim` command**: `forge sim -d <decks> -n <numGames> -f <format> ...`. Built for running many games headless / scripting deck tests / tournaments. (This is what slightlymagic forum thread #20283 is about.)
- **Perf limits:** Wiki warns games *"can lag and become almost unbearably long when the AI has a lot to think about."* No published games/sec. No documented fast clone/restore or determinism guarantee for RL.

## XMage (magefree/mage)

- **Language/arch:** Java; **most rules-complete free engine (~28,000 cards)**, full rules enforcement. Client/server socket architecture. Very active.
- **AI:** Multiple implementations. Default `ComputerPlayer` ("Mad"). Crucially also ships **`ComputerPlayerMCTS`** — MCTS with thread-pool simulations and information-set handling. So XMage has more sophisticated built-in search AI than Forge.
- **Hidden info:** the MCTS variant uses information sets; simulation-based AI historically noted to effectively look at state it controls during rollouts. (Tellingly, MageZero built its *own* RL atop XMage rather than reuse the built-in MCTS.)
- **Perf limits:** Heavyweight, threading-limited for rollouts. MageZero's measured **~75 sims/sec on 8 threads (worse than single-thread)** directly reflects XMage's heap/object-churn cost per simulated state. Not designed for fast clone/restore.

## Engine choice summary (for a fast self-play MTG engine)

| Dimension | Winner |
|---|---|
| Rules completeness | XMage > Forge (both >> a rewrite) |
| Built-in headless batch runner | Forge (`sim`) — cleaner/documented |
| Built-in search AI to learn from | XMage (has MCTS); Forge is pure heuristics |
| Speed / forkable-determinism / clone-restore | **Both weak — unsolved in all projects** |

## Overall fork-base recommendation

- Want working RL+MCTS code **today** → **MageZero (on XMage)** is the only live complete option; accept the throughput ceiling.
- Want a genuinely **fast** self-play engine → neither gives cheap forkable/deterministic state. Realistic paths:
  - **(a)** hard-optimize an XMage fork (MageZero's bet), or
  - **(b)** accept reduced card coverage for a purpose-built fast engine — and this is where the **port-to-Rust** idea lives (see `../overview.md`).
