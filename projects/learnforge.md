# LearnForge

**Repo:** https://github.com/thesilencelies/LearnForge
**Author:** Stephen Lilico ("thesilencelies")
**Built on:** the **Forge** engine
**Status:** **dead prototype** — ~23 commits, last commit **Jan 27 2016**, left mid-debugging (final commit: *"made NNevalNet static, still need to find what within that is causing the bug"*). No releases. README is a build-instructions stub.

## What it is

A reinforcement-learning AI for MTG built on a **full fork of the Forge tree** (the repo contains Forge's `cardsfolder`, `forge-gui-desktop`, Maven build). The RL code is mixed directly into the Forge source rather than sitting as a clean external layer.

- ~99% Java.
- Build: Maven-compile Forge → copy fat JAR into `Runnable_folder` → `java -jar`.

## The AI (inferred from file names / commits)

A neural-network evaluation net ("**NNevalNet**") used as a learned value/eval function on top of Forge game states — i.e. RL/learned-eval, **not** a full AlphaZero/MCTS+policy stack. No evidence of a working MCTS+policy self-play loop. Abandoned mid-bug; likely never produced a trained, functioning agent.

## Imperfect information

Undocumented. Given Forge's underlying AI sees engine state, a naive eval-net hookup would likely have had access to more than a real player would. No evidence either way.

## Verdict

**Not a viable fork-base.** Abandoned 10+ years, undocumented, pinned to old Forge/Java 7, never reached a working state. Only value: a reference for *how someone wired a NN eval into Forge*. If you want Forge-based RL, fork **current upstream Forge** directly, not this.
