# MTG Comprehensive Rules — Engine Data-Model Notes (CR 2026-02-27)

Source: official Comprehensive Rules text, `media.wizards.com/2026/downloads/MagicCompRules 20260227.txt`. All quotes verbatim against the 2026-02-27 text.

## §400 Zones

**The seven zones (400.1):** "There are normally seven zones: library, hand, battlefield, graveyard, stack, exile, and command. Some older cards also use the ante zone." Ownership: "Each player has their own library, hand, and graveyard. The other zones are shared by all players."

**Engine must represent:**
- A `Zone` enum: `Library, Hand, Battlefield, Graveyard, Stack, Exile, Command, Ante`.
- Per-player ownership for Library/Hand/Graveyard (and Ante); shared singletons for Battlefield/Stack/Exile/Command.
- Two orthogonal axes per object: **owner** (fixed) and **controller** (mutable, may be null e.g. in graveyard). 405.4: "The controller of a spell is the player who cast it."

**Public vs hidden (400.2):** "Public zones are zones in which all players can see the cards' faces, except for those cards that some rule or effect specifically allow to be face down. Graveyard, battlefield, stack, exile, ante, and command are public zones. Hidden zones are zones in which not all players can be expected to see the cards' faces. Library and hand are hidden zones, even if all the cards in one such zone happen to be revealed." → visibility is a property of the **zone**, not of momentary revelation; the only exception is cards specifically allowed to be face down. Model `Zone.is_public: bool`; face-down is a per-object override on a public zone.

**Ordered vs unordered (400.5):** "The order of objects in a library, in a graveyard, or on the stack can't be changed except when effects or rules allow it. The same is true for objects arranged in face-down piles in other zones. Other objects in other zones can be arranged however their owners wish, although who controls those objects, whether they're tapped or flipped, and what other objects are attached to them must remain clear to all players." → **Rules-fixed order**: Library, Graveyard, Stack, and *any face-down pile* (model as a sequence/list). **Positionally free**: Hand, Battlefield (owner may rearrange). For **Exile** and **Command**, order is not rules-fixed in general, but face-down exile piles (400.5) and specific effects that return exiled cards in a known/chosen order mean you should NOT assume a pure unordered set — model as a collection that can carry order when needed. Track tapped/flipped/attachments for battlefield objects.

**Redirect-to-owner (400.3):** "If an object would go to any library, graveyard, or hand other than its owner's, it goes to its owner's corresponding zone." → A move targeting a non-owner's hidden zone (or graveyard) is rewritten to the owner's. Engine: normalize destination before applying.

**Type-based zone restrictions (400.4):**
- 400.4a instant/sorcery would enter battlefield → "it remains in its previous zone."
- 400.4b conspiracy/phenomenon/plane/scheme/vanguard would leave command zone → "it remains in the command zone."

**Move algorithm (400.6) — engine must implement this pipeline:** "If an object would move from one zone to another, determine what event is moving the object." Then: (1) if moving to a public zone its owner can look at, owner looks at it for abilities affecting the move (and, for the battlefield, each other player able to look at it does so); (2) apply replacement effects (from the object or elsewhere); (3) if contradictory/mutually-exclusive effects, controller (else owner) chooses which to apply and what it does; (4) "Then the event moves the object." Note "two simultaneous 'destroy' effects" can be mutually exclusive.

### §400.7 — NEW OBJECT / NO MEMORY (critical)

**400.7:** "An object that moves from one zone to another becomes a new object with no memory of, or relation to, its previous existence. This rule has the following exceptions."

Engine principle: a zone change mints a **new object identity** (new game-object ID). Default: drop all marked damage, counters, attachments, continuous-effect attachments, last-known-info links, "this turn" memory. THEN apply the enumerated exceptions where info IS carried across:

**EXCEPTIONS (these matter — each is a carry-across the engine must special-case):**
- **400.7a** — Effects from spells/activated/triggered abilities that change characteristics or controller of a **permanent spell on the stack** continue to apply to the permanent it becomes.
- **400.7b** — Static abilities granting an ability to a permanent spell that functions on the battlefield continue to apply to the resulting permanent (see 611.3d).
- **400.7c** — Prevention effects applying to damage from a permanent spell on the stack continue to apply to damage from the resulting permanent.
- **400.7d** — A permanent's ability can reference info about the spell that became it as it resolved, "including what costs were paid to cast that spell or what mana was spent to pay those costs."
- **400.7e** — Zone-change-trigger abilities (e.g. "When this Aura is put into a graveyard from the battlefield") **can find the new object** it became in the destination zone, **if that zone is public**.
- **400.7f** — Enchanted-permanent-LTB triggers can find the new graveyard object each Aura became, if put there at the same time the enchanted permanent left (or as the SBA in 704.5m).
- **400.7g** — A granted "may cast" ability continues to apply to the new stack object after casting.
- **400.7h** — When an effect allows casting a card, other parts of that effect can find the new stack object.
- **400.7i** — When an effect allows playing a land, other parts can find the new battlefield object.
- **400.7j** — If an effect (or a spell/ability **cost**) moves an object to a **public** zone, that effect/spell/ability can find it.
- **400.7k** — Madness: after resolving the trigger, if the exiled card wasn't cast and moved to a public zone, effects referencing the discarded card can find it.
- **400.7m** — Stickers on an object in a public zone are retained moving to another public zone (123.5); their effects continue.

**Other "new object" rules (not actual zone changes):**
- **400.8** — Exiling an object already in exile: "it doesn't change zones, but it becomes a new object that has just been exiled."
- **400.9** — Face-up command-zone object turned face down → new object.
- **400.10** — Command-zone object put into command zone → new object that just entered.

**400.11** — "outside the game" is **not a zone** (sideboard); 400.11c such cards can't be affected except by CDAs printed on them and effects bringing them in. **400.12** — zone-wide instructions act on all cards in the zone; "The zone itself is not affected."

## §405 The Stack

- **405.1** — Cast spell → card put on stack; activated/triggered ability → goes on top "without any card associated with it."
- **405.2** — "The stack keeps track of the order... Each time an object is put on the stack, it's put on top of all objects already there." → LIFO list.
- **405.3** — Simultaneous puts: active player's objects lowest, then each other player's in **APNAP** order; a player with more than one chooses their relative order.
- **405.4** — Spell has all characteristics of its card; ability on stack "has the text of the ability that created it and no other characteristics." Controller rules (see above; delayed-trigger controller per 603.7d–f).
- **405.5** — "When all players pass in succession, the top (last-added) spell or ability on the stack resolves. If the stack is empty when all players pass, the current step or phase ends and the next begins."
- **405.6** — Things that **don't use the stack**: effects (405.6a), static abilities/CDAs (405.6b), mana abilities resolve immediately (405.6c), special actions (405.6d), turn-based actions (405.6e), state-based actions (405.6f), concession (405.6g), player leaving (405.6h). Engine: these bypass the stack object pipeline entirely.

**Engine:** Stack = ordered list of `StackObject { kind: Spell|ActivatedAbility|TriggeredAbility, source, controller, text, targets, modes, x_value, chosen_costs }`.

## §117 Priority

- **117.1** — "which player can take actions at any given time is determined by a system of priority. The player with priority may cast spells, activate abilities, and take special actions." 117.1a instants any time w/ priority; noninstant during own main phase w/ priority + empty stack. 117.1d mana abilities can be activated whenever priority, when casting/activating something requiring a mana payment, OR when a rule/effect asks for mana.
- **117.2** — Things outside priority: 117.2a triggered abilities ("nothing actually happens at the time an ability triggers"); 117.2b statics; 117.2c turn-based; 117.2d SBAs; 117.2e "no player has priority while a spell or ability is resolving."
- **117.3** — Who gets priority: 117.3a AP at start of most steps/phases after turn-based actions + beginning-of-step triggers on stack (none during untap step); 117.3b AP after a (non-mana) spell/ability resolves; 117.3c whoever acted retains priority; 117.3d passing moves to next in turn order.
- **117.4** — "If all players pass in succession... the spell or ability on top of the stack resolves or, if the stack is empty, the phase or step ends."
- **117.5 — THE CORE LOOP (engine must implement exactly):** "Each time a player would get priority, the game first performs all applicable state-based actions as a single event..., then repeats this process until no state-based actions are performed. Then triggered abilities are put on the stack... These steps repeat in order until no further state-based actions are performed and no abilities trigger. Then the player who would have received priority does so."
- **117.7** — "in response to": new spell/ability on top resolves first.

**Engine predicate:** `before_grant_priority(): loop { while SBA_applicable: perform_all_SBA_as_single_event(); put_triggered_abilities_on_stack_APNAP(); } until no_SBA and no_triggers`.

## §603 Triggered Abilities

- **603.1** — Structure: "[When/Whenever/At] [trigger condition or event], [effect]." 603.1a trailing targeting/can't-be-countered text "is not part of the ability's effect. It functions while the ability is on the stack."
- **603.2** — "Whenever a game event or game state matches a triggered ability's trigger event, that ability automatically triggers. The ability doesn't do anything at this point." 603.2c triggers once per occurrence (but multiple occurrences in one event → multiple triggers). 603.2e "becomes" triggers only on the transition, not on a pre-existing state nor on ETB-in-that-state. **603.2f** — if the source is "at no time visible to all players, the ability does not trigger." **603.2g** — "An event that's prevented or replaced won't trigger anything."
- **603.3** — Once triggered, controller puts it on the stack as a non-card object next time a player would receive priority; topmost; "has the text of the ability that created it, and no other characteristics." 603.3b two-phase APNAP placement (non-"ability-triggering" conditions first, then the rest), re-checking SBAs/triggers between.
- **603.4 — INTERVENING "IF" (engine: double-check):** "When/Whenever/At [trigger event], if [condition], [effect]." Checks condition (1) at trigger time — only triggers if true; and (2) **again as it resolves** — "If the condition isn't true at that time, the ability is removed from the stack and does nothing." Only applies to an "if" immediately following the trigger condition.
- **603.5** — "may"/"unless" abilities still go on the stack; the choice is made on resolution.
- **603.6 — zone-change triggers:** "During resolution, these abilities look for the object in the zone that it moved to. If the object is unable to be found... the part of the ability attempting to do something to the object will fail." 603.6a ETB; 603.6c LTB ("checks for it only in the first zone that it went to"; "from anywhere" is never an LTB ability); 603.6d "enters with/as/tapped" is a **static ability / replacement effect**, not a trigger.
- **603.7** — Delayed triggered abilities (created during resolution / by replacement / by static). 603.7d–f define source & controller. 603.7c references a particular object but "if that object left that zone and then returned, it's a new object and thus won't be affected. See rule 400.7."
- **603.8 — STATE TRIGGERS:** "Some triggered abilities trigger when a game state... is true, rather than... when an event occurs. These abilities trigger as soon as the game state matches the condition... These are called state triggers. (Note that state triggers aren't the same as state-based actions.) A state-triggered ability doesn't trigger again until the ability has resolved, has been countered, or has otherwise left the stack." Then, if the object is still in the same zone and the state still matches, it triggers again. Engine: latch per state-trigger; re-arm only after it leaves the stack and the condition still holds.
- **603.9** — "loses the game" triggers fire on loss OR leaving the game regardless of reason (except leaving as the result of a draw).
- **603.10 — "LOOK BACK IN TIME" (engine: must snapshot pre-event state for these):** "the game 'looks back in time'... using the existence of those abilities and the appearance of objects immediately prior to the event." List: 603.10a LTB / leaves-graveyard / public-object-put-into-hand-or-library; 603.10b phases out; 603.10c becomes unattached; 603.10d loses control / opponent gains control; 603.10e spell countered; 603.10f player loses game; 603.10g planeswalks away.
- **603.12** — Reflexive triggered abilities ("when you do") — follow delayed-trigger rules but are checked immediately after creation against events earlier in the same resolution.

**Engine:** every event must be checked against registered trigger conditions on **objects existing immediately after** the event (default 603.10/700.1), EXCEPT the 603.10 look-back set which uses the pre-event snapshot. A `TriggerCondition` carries: event-match predicate, optional intervening-if (re-evaluated on resolve), zone-change-find semantics, look-back flag, state-trigger latch.

## §608 / §609 Resolving Spells & Abilities

- **608.1** — top of stack resolves on all-pass.
- **608.2** — instant/sorcery/ability resolution order: 608.2a then 608.2b first; 608.2c–m as appropriate; 608.2n/2p last.
- **608.2a** — re-check intervening "if" (→ 603.4); if false, remove, do nothing.
- **608.2b — TARGET LEGALITY (engine: re-validate every target on resolve):** "A target that's no longer in the zone it was in when it was targeted is illegal." "If all its targets, for every instance of the word 'target,' are now illegal, the spell or ability doesn't resolve. It's removed from the stack and, if it's a spell, put into its owner's graveyard." Partial illegality: illegal targets aren't affected for parts they're illegal for; legal ones still are. "If the source of an ability has left the zone it was in, its last known information is used during this process."
- **608.2h — LKI:** uses current info if the object is in the public zone it was expected to be in; "if it's no longer in that zone, or if the effect has moved it from a public zone to a hidden zone, the effect uses the object's last known information." 608.2i exception: "look back in time" cost/info effects.
- **608.2n** — final part: instant/sorcery → owner's graveyard; ability → removed from stack, ceases to exist.
- **608.2p** — after all of 608.2c–n, "resolves" triggers fire.
- **608.3** — Permanent spell resolution: 608.3a no targets → becomes a permanent, enters battlefield under controller's control; 608.3b target check; 608.3c Aura attaches; 608.3e if controller can't put it onto the battlefield, "that player puts it into its owner's graveyard"; 608.3f copy of a permanent spell becomes a (non-"created") token.
- **§609 (Effects)** referenced as the result type produced by resolution; effects never go on the stack (405.6a).

## §700.4 "dies" / Leaves-the-Battlefield

- **700.4 (verbatim):** "The term **dies** means 'is put into a graveyard from the battlefield.'" Engine predicate: `dies(obj) ≡ zone_change(obj, from=Battlefield, to=Graveyard)`. (Note: owner's graveyard always, via 400.3.)
- **Leaves-the-battlefield** (603.6c): any move `from=Battlefield, to=any other zone`, OR a phased-in permanent leaving the GAME because its owner leaves the game. LTB triggers look back in time (603.10a) and find the object only in the **first** zone it reached.
- **New-object principle on death/LTB (400.7):** the graveyard object is a brand-new object — see undying example in **704.8**: last-known-info for a permanent leaving via SBA "is derived from the game state before any of those state-based actions were performed."

## §614 Replacement Effects

- **614.1** — "Such effects watch for a particular event that would happen and... replace that event with a different event." Identifiers: 614.1a "instead"; 614.1b "skip"; 614.1c "enters with/as", "As ... enters"; 614.1d continuous "enters" effects; 614.1e "As ... is turned face up."
- **614.4** — must exist before the event; can't go back in time.
- **614.5** — "A replacement effect doesn't invoke itself repeatedly; it gets only one opportunity to affect an event or any modified events that may replace that event." (Engine: mark an RE as used-for-this-event so it can't re-fire on the modified event.)
- **614.6** — "If an event is replaced, it never happens. A modified event occurs instead, which may in turn trigger abilities." → so 603.2g: no triggers from the original event. The modified event may trigger abilities.
- **614.8** — Regeneration is a destruction-replacement ("The next time [permanent] would be destroyed this turn, instead...").
- **614.12** — ETB-modifying REs: check characteristics "as it would exist on the battlefield," accounting for already-applied ETB modifications, the permanent's own self-statics, and existing continuous effects. 614.12a choices made before entering.
- **614.15 — SELF-REPLACEMENT:** "an effect of a resolving spell or ability that replace[s] part or all of that spell or ability's own effect(s)... self-replacement effects are applied before other replacement effects."
- **614.17** — "can't" effects: not replacement effects but follow similar timing rules; 614.17c an event that "can't happen" can only be replaced by a self-replacement effect.

**Engine:** REs are continuous effects registered as event-interceptors keyed by event shape. The move/event pipeline (400.6) queries applicable REs, applies one, re-queries (per 616.1f), tracks per-event used-flags (614.5).

## §616 Ordering Multiple Replacement/Prevention Effects

- **616.1 (engine: this is the ordering algorithm):** "If two or more replacement and/or prevention effects are attempting to modify the way an event affects an object or player, the affected object's controller (or its owner if it has no controller) or the affected player chooses one to apply, following the steps listed below. If two or more players have to make these choices at the same time, choices are made in APNAP order."
  - **616.1a** — if any self-replacement effects present, one **must** be chosen first.
  - **616.1b** — else if any modify "under whose control an object would enter," one must be chosen.
  - **616.1c** — else if any make it "become a copy as it enters," one must be chosen.
  - **616.1d** — else if any cause it to "enter with its back face up," one must be chosen.
  - **616.1e** — otherwise any applicable effect may be chosen.
  - **616.1f** — "Once the chosen effect has been applied, this process is repeated (taking into account only replacement or prevention effects that would now be applicable) until there are no more left to apply." (Loop.)
  - **616.1g** — nested events: when one RE applies to an event and another applies to an event contained within it, the **inner (contained)** effect can't be chosen until after the **outer** effect has been chosen (e.g. Doubling Season before the token's own ETB choices).
- **616.2** — an RE can become applicable as a result of another RE modifying the event (chains regardless of timestamp).

**Engine:** repeated single-pick loop with a priority ordering (self-replacement → control → copy → back-face → free choice), the picker chosen by affected object's controller/owner (APNAP on ties), re-evaluating the applicable set after each application, and respecting outer-before-inner nesting.

## §704 State-Based Actions

- **704.1** — "State-based actions are game actions that happen automatically whenever certain conditions... are met. State-based actions don't use the stack." 704.1a abilities watching a game state are **triggered abilities (state triggers), not SBAs**.
- **704.2** — checked throughout, "not controlled by any player."
- **704.3 — check timing (engine loop, mirrors 117.5):** "Whenever a player would get priority, the game checks... then performs all applicable state-based actions simultaneously as a single event. If any state-based actions are performed... the check is repeated; otherwise all triggered abilities... are put on the stack, then the check is repeated. Once no more state-based actions... and no triggered abilities are waiting..., the appropriate player gets priority." (Cleanup-step special case noted.)
- **704.4** — "Unlike triggered abilities, state-based actions pay no attention to what happens during the resolution of a spell or ability." (Transient toughness 0 mid-resolution does not kill it.)
- **704.5 — the SBA list (engine: each is a predicate over current game state; perform all matching simultaneously):**
  - 704.5a life ≤ 0 → lose. 704.5b drew from empty library since last check → lose. 704.5c ≥10 poison → lose.
  - 704.5d token off the battlefield → ceases to exist. 704.5e copy of a spell off the stack / copy of a card off stack-or-battlefield → ceases to exist.
  - 704.5f creature toughness ≤ 0 → graveyard ("Regeneration can't replace this event"). 704.5g lethal marked damage ≥ toughness → destroyed (regen can). 704.5h dealt deathtouch damage → destroyed (regen can). 704.5i planeswalker loyalty 0 → graveyard.
  - 704.5j legend rule. 704.5k world rule. 704.5m Aura attached illegally / unattached → graveyard. 704.5n Equipment/Fortification illegally attached → unattach (stays). 704.5p battle/creature attached → unattach; other non-Aura/Equip/Fort attached → unattach (stays).
  - 704.5q +1/+1 and -1/-1 counters annihilate (remove N of each). 704.5r over-max counters trimmed. 704.5s Saga final chapter → sacrifice. 704.5t dungeon bottommost room → remove from game. 704.5v battle defense 0 (and not the source of an as-yet-unresolved triggered ability) → graveyard. 704.5w/x battle protector rules. 704.5y if a permanent has more than one Role controlled by the SAME player, all but that player's most-recent-timestamp Role → graveyard. 704.5z start-your-engines! with no speed → speed 1.
  - 704.6 — variant SBAs (2HG, Commander 21-damage & commander-to-command-zone, Archenemy, Planechase).
  - 704.7 — "If multiple state-based actions would have the same result at the same time, a single replacement effect will replace all of them."
- **704.8 — LKI for SBA-driven LTB (engine: snapshot before the simultaneous SBA event):** "If a state-based action results in a permanent leaving the battlefield at the same time other state-based actions were performed, that permanent's last known information is derived from the game state before any of those state-based actions were performed." (Undying/Young Wolf example.)

---

## Cross-cutting engine takeaways

1. **Object identity is the spine.** Every zone change (400.7) mints a new object ID; default-drop all per-object state; apply the 400.7a–m carry-across exceptions and the "doesn't change zones but new object" cases (400.8/.9/.10).
2. **Two priority/SBA loops are the same shape** (117.5 ≈ 704.3): SBA-to-fixpoint, then put triggers on stack, repeat, then grant priority.
3. **Snapshots matter.** Maintain pre-event/last-known-info snapshots for: 603.10 look-back triggers, 608.2b/608.2h LKI on resolution, 704.8 SBA-LTB LKI.
4. **Event pipeline is the choke point** (400.6 + 614 + 616): determine event → find object's abilities → gather REs → order/pick via 616.1 priority loop (outer-before-inner) → mark used (614.5) → emit modified event → check triggers on post-event objects (603.10 default), except the look-back set.
5. **"dies" is exactly `Battlefield→Graveyard`** (700.4); LTB is `Battlefield→anything` (or a phased-in permanent leaving the game with its owner), found only in the first destination zone (603.6c), looking back in time (603.10a).

Sources:
- [WotC Comprehensive Rules (2026-02-27 txt)](https://media.wizards.com/2026/downloads/MagicCompRules%2020260227.txt)
- [Yawgatog hyperlinked CR](https://yawgatog.com/resources/magic-rules/)

(Local working copy of the rules text used for verbatim extraction: `/tmp/cr.txt`.)