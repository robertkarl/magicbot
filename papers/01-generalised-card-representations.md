# Learning With Generalised Card Representations for Magic: The Gathering

**PDF:** `generalised-card-representations.pdf`
**arXiv:** 2407.05879 — https://arxiv.org/abs/2407.05879
**Authors:** Timo Bertram, Johannes Fürnkranz (JKU Linz), Martin Müller (U. Alberta)
**Venue/year:** IEEE Conference on Games (CoG) 2024

## What it's about

**Drafting / deck-building only — NOT gameplay.** Predicts which card a *human* picks from a draft pack given the partial deck so far. Included here only because its **card-representation idea is the reusable part** for a play agent.

## Why MTG is hard (its framing)

Enormous, *non-stationary* card pool: 3,037–26,524 unique cards depending on format; deck configs ~10^126–10^183; frequent set releases force one-hot/per-card models to be **retrained for every new set**.

## Method

Contextual Preference Ranking (CPR) via **Siamese networks + triplet loss** (margin 1). Learns "card j > card k given deck context C"; embeds cards/decks into a shared 512-dim space where pick quality = embedding distance. **No MCTS/CFR/RL** — supervised representation learning.

Card encodings compared: random (baseline), hand-coded features (1306-dim incl. sentence-transformer text embeddings), conv-autoencoder over card images (32/1024-dim), 16-dim meta stats (pick/win rates from public APIs), and combos. Deck encoder: 45×n matrix through conv layers.

## Scope / data

~**100M human draft picks** across 14 sets (M19 = 29.1M). Limited (draft) format. 80/20 split per set.

## Key results

- Known cards: encoding barely matters — all ≈67–68% pick accuracy (one-hot 67.8%).
- **Unseen cards:** big spread — random 23.8%, image-1024 31.1%, meta-only 42.1%, Features+Meta+Image 42.9%.
- Pre-train on 75M picks / 2,990 cards → test on held-out BRO set: **55.4% of human picks correct on cards never seen in training**; fine-tune → 62.3% and faster convergence.

## Limits the authors admit

42 picks treated as independent (ignores passing/signaling); meta-stats don't exist for brand-new sets; multi-modal fusion doesn't reliably beat single modalities ("mechanism not clear"); transfer speeds convergence but doesn't raise the ceiling; predicting *humans* caps accuracy (humans aren't optimal).

## Imperfect info / action space

Action space naturally small (2–15 cards/pack), handled by embedding-distance ranking. **Imperfect info essentially not modeled** — context is your own partial deck only.

## Relevance to us

Indirect but real: the **generalised card-embedding** (text + hand-features) is directly reusable as the **card-featurization front-end** for a play agent, so the agent isn't tied to a fixed card vocabulary — the concrete answer to the "rotates every 3 months" problem. The image-autoencoder and meta-stats tricks are likely overkill; hand-features + text embeddings carried most of the signal.
