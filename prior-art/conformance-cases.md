I'll produce the conformance test checklist directly from the verified research corpus. This is a synthesis task using the provided rules research, so I'll write the checklist now.

# MTG Rules Engine — Conformance Test Checklist (Golden Tests)

Priority key: **P0** = breaks basic correctness (engine is fundamentally wrong without it). **P1** = breaks common real-game interactions. **P2** = rarer/corner cases, still rules-mandated.

Each item: **Setup → Action → Expected outcome** with CR references.

---

## A. Zone-change / dies / "new object" (CR 400.7, 700.4, 603.6)

- [ ] **A1 (P0) — "dies" is exactly Battlefield→Graveyard.** Setup: a creature on the battlefield with a "When this dies, draw a card" trigger. Action: destroy it. Expected: trigger fires. Then exile a *second* identical creature from the battlefield. Expected: its dies-trigger does NOT fire (exile is LTB but not "dies"). CR 700.4.

- [ ] **A2 (P0) — Zone change mints a new object; per-object state is dropped.** Setup: creature with two +1/+1 counters, marked damage, and an Aura attached, controlled by a player who is not its owner. Action: bounce it to owner's hand, then recast it. Expected: the new battlefield object has zero counters, zero marked damage, no Aura, controller = caster, and a new object identity. CR 400.7.

- [ ] **A3 (P0) — Redirect to owner's zone.** Setup: player B controls (via theft) a creature owned by player A. Action: an effect would put that creature into B's graveyard/hand/library. Expected: it goes to A's (owner's) corresponding zone instead. CR 400.3.

- [ ] **A4 (P0) — Instant/sorcery cannot enter the battlefield.** Setup: an effect attempts to put an instant card onto the battlefield. Expected: it remains in its previous zone. CR 400.4a.

- [ ] **A5 (P1) — LTB trigger finds the object only in the FIRST zone it reached.** Setup: a creature with "When this leaves the battlefield, exile it." Action: it dies (goes to graveyard) and a replacement immediately moves it elsewhere. Expected: the trigger looks for it in the graveyard (first destination) only; if moved on, the "do something to it" part fails. CR 603.6c, 603.10a.

- [ ] **A6 (P1) — "from anywhere" is never an LTB ability.** Setup: a card with "Whenever a creature dies / is put into a graveyard from anywhere." Action: a creature is milled (library→graveyard). Expected: the "from anywhere" ability triggers; a strict LTB ability does NOT. CR 603.6c.

- [ ] **A7 (P1) — Zone-change trigger finds the new object in a public destination (400.7e).** Setup: an Aura with "When this Aura is put into a graveyard from the battlefield, return it to your hand." Action: it's destroyed. Expected: the resolving trigger finds the new graveyard object and can return it (graveyard is public). CR 400.7e.

- [ ] **A8 (P2) — Exiling an already-exiled object mints a new object without a zone change.** Setup: a card already in exile. Action: a second effect exiles it. Expected: it does not "change zones," but becomes a new object that has just been exiled (re-arms "just entered exile" triggers; breaks prior linked references). CR 400.8.

- [ ] **A9 (P2) — Permanent spell's effects/ability info carry across to the permanent (400.7a/d).** Setup: a creature spell on the stack given +2/+0 until end of turn while on the stack, cast with X=3. Action: it resolves and enters. Expected: the +2/+0 still applies to the permanent, and an ability that references "the X paid" sees 3. CR 400.7a, 400.7d.

---

## B. State-Based Actions (CR 704)

- [ ] **B1 (P0) — SBA loop runs to fixpoint before priority, off-stack.** Setup: a board where one creature dying causes another to reach 0 toughness. Action: trigger the first death. Expected: both are moved to graveyard in successive SBA passes before any player gets priority; no stack object is used for the deaths. CR 704.3, 117.5.

- [ ] **B2 (P0) — 0 toughness → graveyard, regen can't save it.** Setup: a 1/1 with a regeneration shield active; apply a -1/-1 effect making it 0/0. Action: SBA check. Expected: it's put into graveyard; regeneration does NOT replace this. CR 704.5f.

- [ ] **B3 (P0) — Lethal marked damage ≥ toughness → destroyed; regen CAN save.** Setup: a 2/2 with a regeneration shield; mark 2 damage. Action: SBA check. Expected: destruction is replaced by regeneration (tap, remove from combat, clear damage, shield consumed). CR 704.5g, 614.8.

- [ ] **B4 (P0) — SBAs ignore transient state mid-resolution.** Setup: a spell that during resolution sets a creature's toughness to 0 and then back to 1 within the same resolution. Action: resolve. Expected: the creature is NOT destroyed (SBAs don't check mid-resolution). CR 704.4.

- [ ] **B5 (P0) — +1/+1 and -1/-1 counters annihilate as an SBA.** Setup: a creature with three +1/+1 and two -1/-1 counters. Action: SBA check. Expected: two of each are removed, leaving one +1/+1 counter. CR 704.5q.

- [ ] **B6 (P1) — Legend rule.** Setup: a player controls two legendary permanents with the same name. Action: SBA check. Expected: that player chooses one to keep; the other(s) go to graveyard. CR 704.5j.

- [ ] **B7 (P1) — Aura attached illegally / unattached → graveyard.** Setup: an Aura enchanting a creature; the creature leaves. Action: SBA check. Expected: the Aura is put into its owner's graveyard (and its LTB triggers can find it). CR 704.5m, 400.7f.

- [ ] **B8 (P1) — Equipment unattaches but stays.** Setup: equipped creature leaves. Action: SBA check. Expected: Equipment becomes unattached and remains on the battlefield. CR 704.5n.

- [ ] **B9 (P1) — LKI for an SBA-driven death uses pre-SBA-event state (704.8).** Setup: a creature with Undying (returns with a +1/+1 counter if it had none) at 1/1 with a -1/-1 counter applied making it 0/0, plus a separate "it had a +1/+1 counter" check. Action: simultaneous SBA. Expected: LKI is derived from the game state *before* any of those SBAs were performed (e.g., Young Wolf / Undying counter-presence check uses pre-event counters). CR 704.8.

- [ ] **B10 (P1) — Token off the battlefield ceases to exist, but only AFTER one look.** Setup: a token creature with "When this dies, draw a card." Action: destroy it. Expected: it reaches the graveyard momentarily (dies trigger fires and the object is visible to triggers), then the next SBA check removes it from the graveyard. CR 704.5d, 111.7, 700.4.

- [ ] **B11 (P2) — Copy of a spell ceases to exist off the stack.** Setup: a copied instant on the stack. Action: it resolves/leaves the stack. Expected: it ceases to exist as an SBA (does not go to graveyard). CR 704.5e.

- [ ] **B12 (P2) — Planeswalker loyalty 0 → graveyard; Saga final chapter → sacrifice; Battle defense 0 → graveyard.** Three separate assertions per CR 704.5i, 704.5s, 704.5v.

- [ ] **B13 (P2) — Multiple SBAs with the same result share one replacement.** Setup: two creatures would simultaneously die and a single replacement effect ("if a creature would die, exile instead") applies. Action: SBA. Expected: the one replacement replaces all of them. CR 704.7.

---

## C. The Stack & Priority (CR 117, 405, 608)

- [ ] **C1 (P0) — Priority core loop ordering: SBA-to-fixpoint, then triggers on stack APNAP, repeat, then grant priority.** Setup: an action that causes both an SBA and a triggered ability. Action: perform it. Expected: all SBAs resolve to fixpoint first, then triggers go on stack in APNAP order, re-checking SBAs/triggers between, before priority is granted. CR 117.5, 704.3, 603.3b.

- [ ] **C2 (P0) — Stack is LIFO; "in response" resolves first.** Setup: spell A on stack; in response cast spell B. Action: all pass. Expected: B resolves before A. CR 405.2, 117.7.

- [ ] **C3 (P0) — Empty stack + all pass ends the step/phase.** Setup: empty stack, all players pass. Expected: current step/phase ends, next begins. CR 405.5, 117.4.

- [ ] **C4 (P0) — An ability on the stack is independent of its source.** Setup: a permanent activates an ability; in response, destroy the source. Action: resolve the ability. Expected: it still resolves fully, using LKI of the source. CR 113.7a / 405.4, 608.2b.

- [ ] **C5 (P0) — Targets and X are locked at announcement, re-validated only at resolution.** Setup: a spell cast targeting a creature with X=3. Action: at resolution attempt to "change" X or target. Expected: frozen values used; only legality is re-checked. CR 601.2, 608.2b.

- [ ] **C6 (P0) — All targets illegal → spell countered by rules; a spell goes to graveyard, an ability ceases.** Setup: single-target spell whose only target becomes illegal (e.g., gains hexproof / leaves zone). Action: resolve. Expected: spell doesn't resolve, removed from stack to owner's graveyard. CR 608.2b, 608.2n.

- [ ] **C7 (P1) — Partial target legality.** Setup: a spell with two targets; one becomes illegal. Action: resolve. Expected: it still resolves, affecting only the legal target; the illegal-for-that-part effect doesn't happen. CR 608.2b.

- [ ] **C8 (P1) — APNAP for simultaneous stack puts.** Setup: at the start of a step, both players have a beginning-of-step trigger. Action: place on stack. Expected: active player's goes on lowest, then non-active player's on top (so NAP's resolves first). CR 405.3, 603.3b.

- [ ] **C9 (P1) — Mana abilities don't use the stack.** Setup: tap a land for mana in response to something. Expected: mana is produced immediately, no stack object, can't be responded to. CR 405.6c, 605.

- [ ] **C10 (P2) — A countered spell / spell leaving the stack does NOT fire LTB ("leaves the battlefield") triggers.** Setup: a creature spell is countered. Expected: no dies/LTB triggers (it was never a permanent). CR 603.6c (LTB is Battlefield→).

---

## D. Triggered Abilities (CR 603)

- [ ] **D1 (P0) — Triggering does nothing immediately; goes on stack at next priority.** Setup: an ETB trigger. Action: permanent enters. Expected: nothing happens at entry time; the trigger is placed on the stack the next time a player would receive priority. CR 603.2, 603.3.

- [ ] **D2 (P0) — Intervening "if" double-check.** Setup: "At the beginning of your upkeep, if you control three or more creatures, draw a card." Trigger condition true at upkeep but control drops below three before resolution. Action: resolve. Expected: ability is removed from stack and does nothing (condition re-checked on resolution). CR 603.4, 608.2a.

- [ ] **D3 (P0) — "Becomes" triggers only on transition, not on a pre-existing/ETB-in-state.** Setup: "Whenever a creature becomes tapped." Action: a creature enters the battlefield already tapped. Expected: no trigger (no untapped→tapped transition). CR 603.2e.

- [ ] **D4 (P0) — Prevented/replaced events don't trigger anything.** Setup: a "Whenever this is dealt damage" trigger; all damage to it is prevented. Action: damage event. Expected: no trigger. CR 603.2g, 614.6.

- [ ] **D5 (P1) — Look-back-in-time triggers use the pre-event snapshot.** Setup: "When a creature with power 4+ dies, draw a card," on a creature pumped to 4 power that then dies (pump and death simultaneous, or pump source also gone). Action: it dies. Expected: trigger evaluates against the creature's characteristics immediately before the event. CR 603.10a.

- [ ] **D6 (P1) — Source not visible to any player → ability doesn't trigger.** Setup: a face-down/hidden-zone source whose trigger event occurs while it's not visible to all players. Expected: no trigger. CR 603.2f.

- [ ] **D7 (P1) — State trigger latch.** Setup: "When a player has 0 or less life, that player loses" style state trigger (or "when you have no cards in hand"). Action: condition becomes true. Expected: triggers once; does not re-trigger until it has left the stack and the condition still holds. CR 603.8.

- [ ] **D8 (P2) — Zone-change trigger fails the "do X to it" part if the object can't be found.** Setup: "When this dies, return it to the battlefield," but a replacement sends it to exile instead of graveyard. Action: resolve. Expected: the return part fails (object not in expected zone). CR 603.6, 603.6c.

- [ ] **D9 (P2) — Reflexive trigger ("when you do") checks events earlier in the same resolution.** Setup: "You may sacrifice a creature. When you do, draw a card." Action: resolve, choosing to sacrifice. Expected: the reflexive trigger is created and fires. CR 603.12.

- [ ] **D10 (P2) — "enters tapped / enters with counters" is a replacement, not a trigger.** Setup: a creature that "enters with two +1/+1 counters." Action: it enters. Expected: it's already a (base+2/+2) creature when first seen on the battlefield; SBAs never see it at lower toughness; not placed on the stack. CR 603.6d, 614.1c.

---

## E. Replacement Effects & Ordering (CR 614, 616)

- [ ] **E1 (P0) — Replacement effects don't use the stack and each applies at most once per event.** Setup: "If you would draw, instead draw two." Action: draw. Expected: you draw two; the effect doesn't recursively re-fire on the modified draw. CR 614.5, 614.1a.

- [ ] **E2 (P0) — Self-replacement applies before other replacements.** Setup: a resolving spell with a self-replacement on its own draw, plus an external draw-replacement. Action: resolve. Expected: self-replacement is applied first. CR 614.15, 616.1a.

- [ ] **E3 (P0) — Multiple replacements on one event: affected player/controller orders them, re-evaluating after each.** Setup: two applicable "if ~ would die, instead ~" effects on one creature death, controlled by the affected object's controller. Action: death. Expected: controller picks order; after applying one (marked used), the set is re-evaluated; loop until none apply. CR 616.1, 616.1f.

- [ ] **E4 (P1) — 616.1 priority ordering: control-entry → copy-as-enters → back-face → free.** Setup: a token entering where one effect changes who controls it as it enters and another adds +1/+1 counters. Action: enter. Expected: the control-entry replacement must be chosen before the free-choice one. CR 616.1b–e.

- [ ] **E5 (P1) — Outer-before-inner nesting.** Setup: Doubling Season (doubles tokens/counters) and a token that itself enters with counters. Action: create the token. Expected: the outer (doubling) effect is applied/chosen before the token's own enters-with-counters choices. CR 616.1g.

- [ ] **E6 (P1) — ETB-modifying RE evaluates characteristics "as it would exist on the battlefield."** Setup: an enters-with-counters interacting with a static anthem and the permanent's own self-static. Action: enter. Expected: characteristics computed accounting for already-applied ETB mods, self-statics, and existing continuous effects. CR 614.12.

- [ ] **E7 (P1) — Regeneration is a destruction-replacement.** Setup: regeneration shield on a creature; deal lethal (non-deathtouch, non-0-toughness) damage. Action: SBA destroy. Expected: replaced — creature tapped, removed from combat, damage cleared, shield consumed; creature survives. CR 614.8, 704.5g.

- [ ] **E8 (P2) — "Can't" event can only be replaced by a self-replacement.** Setup: an effect saying a player "can't draw cards," plus a generic draw-replacement. Action: would-draw. Expected: the "can't" stands unless a self-replacement specifically applies. CR 614.17c.

- [ ] **E9 (P2) — APNAP for simultaneous replacement-ordering choices.** Setup: two players each must order replacements for the same event simultaneously. Expected: choices made in APNAP order. CR 616.1.

---

## F. Layer System (CR 613)

- [ ] **F1 (P0) — Characteristics are recomputed, never stored.** Setup: a creature under an anthem (+1/+1 static). Action: remove the anthem source. Expected: P/T recomputed without it immediately; no stale stored value. CR 613.5.

- [ ] **F2 (P0) — Cross-layer order is fixed regardless of timestamp.** Setup: a later-timestamp color effect and an earlier-timestamp type effect. Expected: type (layer 4) applied before color (layer 5) irrespective of timestamps. CR 613.1.

- [ ] **F3 (P0) — Layer 7 sublayer order: set (7b) before modify/counters (7c).** Setup: a creature with base 3/3, an effect setting base P/T to 1/1 (7b), and a +1/+1 counter (7c). Action: compute. Expected: 2/2 (set to 1/1, then +1/+1), regardless of which came first by timestamp. CR 613.4b, 613.4c.

- [ ] **F4 (P0) — CDA P/T (7a) before sets (7b).** Setup: Tarmogoyf-style `*/1+*` CDA plus an effect setting it to 0/1. Action: compute. Expected: 7a defines `*/1+*`, then 7b set to 0/1 overrides as a set. CR 613.4a.

- [ ] **F5 (P0) — Humility + anthem (set vs modify).** Setup: Humility (base P/T 1/1 in 7b, strip abilities in layer 6) and Glorious Anthem (+1/+1 in 7c). Action: compute a creature. Expected: 2/2 with no abilities. A naive timestamp-only engine gets the number wrong. CR 613.4b/c, 613.1f.

- [ ] **F6 (P1) — Layer 6 add-then-remove ordering.** Setup: effect A grants flying (timestamp 1); effect B removes all abilities (timestamp 2). Action: compute. Expected: no flying (later removal wins; CDA-first then timestamp). Then reverse timestamps → flying present. CR 613.1f, 613.3.

- [ ] **F7 (P1) — Dependency overrides timestamp (Blood Moon vs Urborg-style).** Setup: "nonbasic lands are Mountains" and "all lands are also Swamps" in layer 4 where each changes what the other affects. Action: compute. Expected: dependency order applied (613.8a/b), recomputed after each application (613.8c). CR 613.8.

- [ ] **F8 (P1) — Dependency loop falls back to timestamp.** Setup: two mutually dependent same-layer effects forming a loop. Action: compute. Expected: rule ignored, applied in timestamp order. CR 613.8b.

- [ ] **F9 (P1) — CDA vs non-CDA never form a dependency.** Setup: one CDA-sourced and one non-CDA effect in the same layer that would otherwise depend. Action: compute. Expected: treated as independent (timestamp order). CR 613.8a (clause c).

- [ ] **F10 (P1) — Effect-set lock-in (613.6).** Setup: "creatures you control get +1/+1 until end of turn" begins applying; later a new creature enters; then the source is removed. Action: compute. Expected: the effect applies to the locked-in set (not the newcomer), and continues applying in later layers even after its source is gone. CR 613.6.

- [ ] **F11 (P1) — Switch P/T (7d) is last.** Setup: a 1/4 creature with +2/+0 (7c) then a P/T switch (7d). Action: compute. Expected: first 3/4, then switched to 4/3. CR 613.4d.

- [ ] **F12 (P2) — Re-timestamping reshuffles intra-layer order.** Setup: two Auras on a creature; re-attach the earlier one. Action: compute. Expected: re-attached Aura now has the latest timestamp and applies last within its layer. CR 613.7.

- [ ] **F13 (P2) — CDA functions in all zones.** Setup: a `*/*` CDA creature (P/T from cards in graveyard) sitting in hand. Action: query its power. Expected: the CDA value applies even in hand. CR 604.3, 613.

- [ ] **F14 (P2) — Player/rules effects applied after object characteristics, in timestamp order.** Setup: two effects modifying a player and the game rules. Expected: applied in timestamp order after object characteristics are computed. CR 613.10, 613.11.

---

## G. Copy / Copiable Values (CR 707, 706)

- [ ] **G1 (P0) — Copy acquires copiable values only, not counters/auras/buffs/status.** Setup: a 1/1 creature with two +1/+1 counters, a +3/+3 until-end-of-turn buff, an Aura, and tapped. Action: Clone it. Expected: the Clone is a base 1/1 (counters/Aura/buff/tapped NOT copied); counters/auras then re-apply only in their own layers if separately present. A naive engine copying rendered P/T gives 3/3. CR 707.2.

- [ ] **G2 (P0) — Clone of a creature WITH +1/+1 counters that the clone separately keeps.** Setup: copy a 1/1 that has two +1/+1 counters. Expected per CR 707.2 counters aren't copied; but verify the documented case "Clone of a 1/1 with two +1/+1 counters is 3/3" only when the copy itself receives counters — the test must confirm counters are NOT auto-copied, so the bare Clone is 1/1. (Distinguish from naive snapshot.) CR 707.2.

- [ ] **G3 (P0) — Copy effect lives in layer 1a and defines copiable values.** Setup: "becomes a copy of X" on a creature that also has a +1/+1 counter and a Giant Growth. Action: compute. Expected: copy applied at 1a (wipes prior layer-1 mods), then counter (7c) and Giant Growth (7c) re-apply on top. CR 613.2a, 613.2c, 707.3.

- [ ] **G4 (P1) — Type-changing and text-changing effects are NOT copied.** Setup: a creature currently animated/text-changed by external effects. Action: copy it. Expected: the copy reflects printed text/types, not those external layer-3/4 effects. CR 707.2.

- [ ] **G5 (P1) — Copy of a copy sees baked-in copy, not transient layers.** Setup: A is a copy of B (baked into A's copiable values). Action: C copies A. Expected: C gets A's copiable values (i.e., B's printed values), not A's transient layered state. CR 707.3.

- [ ] **G6 (P1) — Copying a stack object copies the choices.** Setup: a spell on the stack cast with mode/targets/X/kicked. Action: copy the spell. Expected: copy carries those choices (new targets may be chosen only where allowed). CR 707.2.

- [ ] **G7 (P1) — CDA is copied (as printed rules text) and re-derives on the copy.** Setup: copy a Tarmogoyf-style `*/1+*` creature. Action: compute the copy's P/T. Expected: the copy has its own `*/1+*` re-derived from current game state, not a frozen number. CR 707.2, 604.3.

- [ ] **G8 (P2) — Copy of a permanent SPELL becomes a non-"created" token on resolution.** Setup: copy a creature spell on the stack; let the copy resolve. Action: resolve. Expected: it enters as a token (not "created"), and ceases to exist as an SBA if it later leaves the battlefield. CR 608.3f, 704.5d.

---

## H. Face-Down Permanents (CR 708, 707.2, 702)

- [ ] **H1 (P0) — Face-down default profile is 2/2 colorless creature, no name/types-beyond-creature/mana cost/abilities.** Setup: a morph/manifest face-down permanent of an actually-large creature. Action: query characteristics. Expected: 2/2 colorless, no name, no abilities, regardless of the hidden card. CR 708.2, 707.2.

- [ ] **H2 (P0) — Hidden identity must not leak to opponents.** Setup: face-down permanent. Action: opponent queries it. Expected: only the 2/2 profile is visible; the real card is private state. CR 708, 400.2.

- [ ] **H3 (P1) — Turning face up is a special action, same object identity preserved.** Setup: face-down 2/2 with a +1/+1 counter, marked damage, and an Aura. Action: turn it face up (special action, not on stack). Expected: same object — counters/damage/Aura/summoning-sickness persist; real characteristics now apply; cannot be responded to. CR 116.2, 708, 707.2.

- [ ] **H4 (P1) — Copying a face-down permanent copies the 2/2 face-down copiable values.** Setup: Clone targets a face-down creature. Action: copy. Expected: the Clone is a plain 2/2 (face-down copiable values), not the hidden card. CR 707.2.

- [ ] **H5 (P2) — Cloak face-down profile is 2/2 with ward {2}.** Setup: a cloaked card. Action: query. Expected: 2/2 with ward {2}. CR 701 (cloak), 708.

- [ ] **H6 (P2) — Manifested noncreature/land is still a 2/2 creature while face down.** Setup: manifest a land. Action: query. Expected: 2/2 creature; turning up reveals the land (with restrictions). CR 701 (manifest), 708.

- [ ] **H7 (P2) — Face-down modification is layer 1b, applied within layer 1.** Setup: face-down permanent under other continuous effects. Action: compute. Expected: face-down profile applied at 1b, defining copiable values before layers 2–7. CR 613.2b.

---

## I. Merged Permanents / Mutate (CR 730, 613.2a)

- [ ] **I1 (P0) — One object: characteristics from top, abilities from all components.** Setup: mutate creature B onto creature A with B on top. Action: query. Expected: a single permanent with B's characteristics (name/P/T/types) but abilities from both A and B. CR 730.

- [ ] **I2 (P0) — On leaving, one LTB event, then each component goes to its owner's appropriate zone.** Setup: a merged 3-card permanent with components owned by two players. Action: destroy it. Expected: one "leaves the battlefield" event fires; then each component card is placed in its own owner's graveyard. CR 730.

- [ ] **I3 (P1) — Counters/attachments/damage live on the merged object, discarded on un-merge.** Setup: merged permanent with two +1/+1 counters and marked damage. Action: it dies. Expected: the single object's counters/damage are not transferred to the individual graveyard cards. CR 730, 400.7.

- [ ] **I4 (P1) — Copying a merged creature copies top's copiable values + merge-granted abilities.** Setup: Clone targets a mutate pile. Action: copy. Expected: the copy uses the top card's copiable values; verify whether merge-granted abilities are reflected per current copiable-value/merge rules (CR 613.2a folds merge into layer 1a). CR 613.2a, 707.2.

- [ ] **I5 (P2) — Merge characteristics computed in layer 1a.** Setup: merged permanent under external layer effects. Action: compute. Expected: the merge characteristic determination happens in layer 1a (copiable-value layer). CR 613.2a.

---

## J. Phasing (CR 702.26)

- [ ] **J1 (P0) — Phase-out is NOT a zone change and NOT a new object.** Setup: a creature with two +1/+1 counters and an Aura. Action: phase it out, then phase it in at untap. Expected: same object identity, counters and Aura intact; no ETB/LTB triggers fire. CR 702.26.

- [ ] **J2 (P0) — Phased-out permanents are excluded from all game queries.** Setup: a phased-out creature. Action: query "creatures you control," attempt to target it, run SBAs. Expected: it's treated as though it doesn't exist (not a valid target, not counted, SBAs ignore it). CR 702.26.

- [ ] **J3 (P1) — Attached permanents phase indirectly and return attached.** Setup: an Aura/Equipment on a creature that phases out. Action: phase out, then in. Expected: the attachment phases out with it (indirectly) and returns still attached. CR 702.26.

- [ ] **J4 (P1) — No ETB triggers on phase-in.** Setup: a creature with an ETB trigger phases out and back in. Action: phase in at untap. Expected: ETB does NOT trigger. CR 702.26.

---

## K. Last Known Information & Linked Abilities (CR 607, 608.2h, 603.10)

- [ ] **K1 (P0) — LKI for a creature that left before resolution.** Setup: "Draw cards equal to target creature's power"; the creature (power 4) dies in response. Action: resolve. Expected: draw 4 (its last-known power with all layers/counters applied), not 0. CR 608.2h.

- [ ] **K2 (P0) — Sacrifice-as-cost then reference the sacrificed object.** Setup: "Sacrifice a creature: gain life equal to its toughness." Action: pay cost (sacrifice a 5-toughness creature), resolve. Expected: gain 5 from LKI; the effect can find the object it moved as a cost to a public zone. CR 608.2h, 400.7j.

- [ ] **K3 (P0) — LKI is frozen, not live-updating.** Setup: an effect references a creature that left the battlefield; later the graveyard object is modified. Action: resolve. Expected: uses the snapshot at the moment it left, not later state. CR 608.2h.

- [ ] **K4 (P1) — Linked abilities pair per-instance, not globally.** Setup: two copies of a Banisher-Priest-style permanent ("Exile target creature" + "When this leaves, return the exiled card"), each exiling a different creature. Action: one leaves. Expected: only its own exiled card returns. CR 607.1–607.3.

- [ ] **K5 (P1) — If the first linked ability didn't happen, the second does nothing.** Setup: the exiling ability was countered / never exiled anything. Action: the "return the exiled card" ability resolves. Expected: it references nothing (not "all cards in exile"). CR 607.4.

- [ ] **K6 (P2) — Linked reference survives the exiled card's object-identity change in exile.** Setup: a card exiled by a linked ability; it's a new object in exile. Action: resolve the return. Expected: the linkage record still finds it. CR 607, 400.7e/j.

---

## L. Continuous Effects from Spells vs Statics (CR 611, 604)

- [ ] **L1 (P0) — Resolved "creatures get +1/+1 until end of turn" locks its affected set.** Setup: cast the pump; afterward a new creature enters. Action: query the newcomer. Expected: newcomer does NOT get +1/+1 (fixed set). CR 611.2c.

- [ ] **L2 (P0) — Static anthem dynamically picks up new objects.** Setup: an anthem static ("other creatures you control get +1/+1"); a new creature enters. Action: query. Expected: newcomer gets +1/+1. CR 604, 613.

- [ ] **L3 (P1) — "For as long as" effects DO re-check.** Setup: an effect "for as long as ~ is true, creatures get +1/+1." Action: condition changes / new creatures enter. Expected: it re-evaluates membership unlike a fixed-set effect. CR 611.

- [ ] **L4 (P1) — "Until end of turn" ends at cleanup.** Setup: a +3/+3 until end of turn. Action: reach cleanup. Expected: the effect ends. CR 514 / 611 (duration).

---

## M. Combat & Damage (CR 509–510, 704)

- [ ] **M1 (P0) — Damage is MARKED, not subtracted from toughness; wiped at cleanup.** Setup: deal 2 damage to a 3/3. Action: query toughness, then reach cleanup. Expected: toughness still 3 with 2 marked damage; lethal is determined by marked-damage SBA; damage clears at cleanup. CR 704.5g, 514.

- [ ] **M2 (P0) — Deathtouch damage destroys via SBA regardless of amount.** Setup: a creature dealt 1 deathtouch damage. Action: SBA. Expected: destroyed (regen can replace). CR 704.5h.

- [ ] **M3 (P1) — Combat damage dealt simultaneously; first strike adds an extra step only when needed.** Setup: attacker with first strike vs blocker without. Action: combat damage. Expected: a first-strike step then a regular step; non-first-strikers deal in the second. CR 510, 509.

- [ ] **M4 (P1) — Damage assignment order locked before damage; trample/multiple blockers.** Setup: trampler blocked by two creatures. Action: assign damage. Expected: lethal assigned to blockers per chosen order before excess tramples. CR 510.

- [ ] **M5 (P2) — -1/-1 effect vs marked damage vs toughness-set are distinct mechanisms.** Setup: a 3/3 with 2 marked damage and a separate -2/-2 effect. Action: SBA. Expected: toughness computed as 1 (layer 7c -2/-2), marked damage 2 ≥ 1 → destroyed; verify the engine doesn't conflate the two. CR 613.4c, 704.5g.

---

## N. Tokens (CR 111, 707, 704)

- [ ] **N1 (P0) — Token ceases to exist off the battlefield via SBA after triggers look.** (See B10.) Setup: token bounced to hand. Action: it reaches hand momentarily; SBA. Expected: any "leaves" triggers can see it, then it vanishes from hand. CR 111.7, 704.5d.

- [ ] **N2 (P1) — A "copy" effect that makes a token copy uses the originating effect's copiable-value definition.** Setup: populate / "create a token copy of target creature." Action: create. Expected: copies the target's copiable values (not current modified P/T) and the result is itself a token. CR 707.2, 111.

- [ ] **N3 (P2) — Token retains existence in graveyard long enough for "dies" triggers.** (Tie to A1/B10.) Setup: token with a dies trigger. Action: it dies. Expected: dies trigger fires before cease-to-exist SBA. CR 700.4, 704.5d.

---

## O. Cross-cutting infrastructure assertions (meta-tests)

- [ ] **O1 (P0) — Every object carries an instance ID + zone-change counter; state is keyed on instance, not card identity.** Test: same physical card across two battlefield stints has distinct instance IDs and does not share "until end of turn" / counter state. CR 400.7.

- [ ] **O2 (P0) — Event pipeline order: determine event → gather REs → order via 616.1 → mark used (614.5) → emit modified event → check triggers on POST-event objects (except 603.10 look-back set).** Test a single composite event exercising all stages. CR 400.6, 614, 616, 603.10.

- [ ] **O3 (P0) — Characteristics are a pure function recomputed on query from (copiable base, ordered continuous effects, counters, status).** Test: mutate any input (add/remove effect, counter, tap) and confirm recomputation with no stale cache. CR 613.5.

- [ ] **O4 (P1) — Snapshots maintained for: 603.10 look-back triggers, 608.2b/h LKI, 704.8 SBA-LTB LKI.** Test each snapshot path independently. CR 603.10, 608.2h, 704.8.

This checklist is complete and ready to be converted directly into golden/unit tests. No file paths are relevant (no files were created; this is returned inline per instructions).