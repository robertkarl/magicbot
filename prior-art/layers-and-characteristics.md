# MTG Layer System (CR 613), Copiable Values (CR 707), and CDAs — Engine Research

Sources: rule text quoted from the Comprehensive Rules mirror at [ancestral.vision (613)](https://ancestral.vision/spells-abilities-and-effects/interaction-of-continuous-effects.html), [ancestral.vision (707)](https://ancestral.vision/additional-rules/copying-objects.html), and [yawgatog.com](https://yawgatog.com/resources/magic-rules/); CDA definition cross-checked against [MTG Wiki: CDA](https://mtg.wiki/page/Characteristic-defining_ability). Layer/timestamp overview confirmed via [Pocket Judge: How Layers Work](https://www.pocket-judge.com/guides/how-layers-work) and [Quiet Speculation: Copiable Values](https://www.quietspeculation.com/2022/01/layers-part-one-copiable-values/).

---

## 1. The Seven Layers — CR 613.1 (in order)

The values of an object's characteristics are determined by starting with the actual object (613.1), then applying all applicable continuous effects in a series of layers in this fixed order. **The cross-layer order is fixed by 613.1 regardless of timestamp or controller; timestamp/dependency only order effects WITHIN a layer (613.7 / 613.8).**

- **613.1a — Layer 1: Rules and effects that modify copiable values are applied.**
- **613.1b — Layer 2: Control-changing effects are applied.**
- **613.1c — Layer 3: Text-changing effects are applied.**
- **613.1d — Layer 4: Type-changing effects are applied.** \"These include effects that change an object's card type, subtype, and/or supertype.\"
- **613.1e — Layer 5: Color-changing effects are applied.**
- **613.1f — Layer 6: Ability-adding effects, keyword counters, ability-removing effects, and effects that say an object can't have an ability are applied.**
- **613.1g — Layer 7: Power- and/or toughness-changing effects are applied.**

### Layer 1 sublayers — CR 613.2

613.2: within layer 1, apply effects in a series of sublayers in the order below; within each sublayer, apply in timestamp order. (Copy effects are themselves a modification of copiable values, hence layer 1.)

- **613.2a — Layer 1a:** \"Copiable effects are applied. This includes copy effects and changes to an object's characteristics determined by merging an object with a permanent.\"
- **613.2b — Layer 1b:** \"Face-down spells and permanents have their characteristics modified as defined in rule 708.2.\"
- **613.2c —** \"After all rules and effects in layer 1 have been applied, the object's characteristics are its copiable values.\" This is the load-bearing statement: copiable value = result after layer 1, before layers 2–7.

### CDAs apply first within layers 2–6 — CR 613.3

**613.3:** within layers 2–6, apply effects from characteristic-defining abilities first, then all other effects in timestamp order. (This is the rule basis for CDAs applying first in those layers; layer 7's CDA precedence is handled separately by sublayer 7a / 613.4a.)

### Layer 6 (abilities) — CR 613.1f

Layer 6 handles all four ability operations together: adding abilities, **keyword counters** (e.g. flying counters), removing abilities, and \"can't have an ability\" effects. Within the layer they sort by 613.3 (CDAs first) then timestamp/dependency, so an add-then-remove vs remove-then-add ordering matters (see Humility gotcha below).

### Layer 7 sublayers — CR 613.4 (P/T)

613.4: within layer 7, apply in a series of sublayers in this order; within each sublayer, timestamp order:

- **613.4a — 7a:** Effects from **characteristic-defining abilities that define power and/or toughness** (e.g. Tarmogoyf's `*/1+*`).
- **613.4b — 7b:** Effects that **set** P and/or T to a specific number or value (e.g. \"becomes 0/1\", Humility's \"base power and toughness 1/1\").
- **613.4c — 7c:** Effects **and counters** that **modify** P/T but don't set it to a specific number/value (e.g. \"+2/+2\", Giant Growth, +1/+1 / -1/-1 counters).
- **613.4d — 7d:** Effects that **switch** a creature's power and toughness.

Crucial: **set (7b) always applies before modify/counters (7c)**, no matter their timestamps. A creature set to 1/1 in 7b then given a +1/+1 counter in 7c is 2/2; the set never \"overwrites\" the later counter.

---

## 2. Ordering within a layer — Timestamp (613.7) and Dependency (613.8)

- **613.7 — Timestamp (verbatim):** \"Within a layer or sublayer, determining which order effects are applied in is usually done using a timestamp system. An effect with an earlier timestamp is applied before an effect with a later timestamp.\" Subrules 613.7a–613.7m specify timestamp assignment (static abilities, spell/ability resolutions, counters, zone entry, attachments, face-down permanents, transforming permanents, plane/phenomenon/scheme/vanguard/conspiracy cards, stickers, and simultaneous objects in APNAP order). An object can be re-timestamped, which can re-sort effects.
- **613.8 — Dependency (sometimes used; overrides timestamp):** \"Within a layer or sublayer, determining which order effects are applied in is sometimes done using a dependency system.\"
  - **613.8a —** An effect \"depends on\" another iff: (a) it's applied in the **same layer (and, if applicable, sublayer)** as the other; AND (b) applying the other would change the text or existence of the first effect, what it applies to, or what it does to any of the things it applies to; AND (c) **neither effect is from a CDA, or both effects are from CDAs.** Otherwise they're independent and use timestamps.
  - **613.8b —** A dependent effect waits to apply until just after all effects it depends on are applied. **Loop fallback (same subrule):** \"If several dependent effects form a dependency loop, then this rule is ignored and the effects in the dependency loop are applied in timestamp order.\"
  - **613.8c —** After each effect is applied, the order of remaining effects is **reevaluated** and may change if an unapplied effect becomes dependent on or independent of others.

- **Effect-set lock-in — 613.6:** If an effect has started to apply, it continues to apply to the **same set of objects** in each later applicable layer/sublayer, even if the ability generating it is removed during the process. An effect that should apply in multiple layers has each part applied in its appropriate layer.

- **Continuous-effect interaction — 613.9 / 613.10 / 613.11:** 613.9 — one continuous effect can override another affecting the **same characteristic** (resolved by timestamp). 613.10 — effects that affect **players** rather than objects are applied in timestamp order after object characteristics are determined. 613.11 — effects that affect the **rules of the game** rather than objects are applied in timestamp order (with cost exceptions per 601.2f).

- **Automatic/instantaneous application — 613.5:** The layer system is continually and automatically performed by the game; resulting characteristic changes are instantaneous.

---

## 3. Copiable Values — CR 707.2 / 707.3

**707.2 (verbatim):** \"When copying an object, the copy acquires the copiable values of the original object's characteristics and, for an object on the stack, choices made when casting or activating it (mode, targets, the value of X, whether it was kicked, how it will affect multiple targets, and so on). The copiable values are the values derived from the **text printed on the object** (that text being **name, mana cost, color indicator, card type, subtype, supertype, rules text, power, toughness, and/or loyalty**), as modified by other copy effects, by its face-down status, and by 'as . . . enters the battlefield' and 'as . . . is turned face up' abilities that set power and toughness (and may also set additional characteristics). **Other effects (including type-changing and text-changing effects), status, counters, and stickers are not copied.**\"

**What IS copied:** printed characteristics (name, mana cost, color indicator, types/subtypes/supertypes, rules text, P/T, loyalty); prior **copy effects** already baked into the original; **face-down** modifications; **\"as ~ enters\" / \"as ~ is turned face up\"** P/T-setting (and additional-characteristic-setting) values; for objects on the stack, the **choices** (modes, targets, X, kicked, etc.). **CDA-defined characteristics are copied indirectly:** the CDA is printed rules text, so it is copied and re-derives its value on the copy — it is not one of 707.2's explicitly enumerated \"as modified by\" categories.

**What is NOT copied:** non-copy continuous effects (Giant Growth, control change, animate, etc.), **type-changing and text-changing effects**, **status** (tapped/flipped/face-down/phased), **counters**, and **stickers**.

**707.3:** \"The copy's copiable values become the copied information, as modified by the copy's status (see rule 110.5). Objects that copy the object will use the new copiable values.\" (Copies of copies see the baked-in copy effect, not the transient layered effects.)

**Layer interaction:** copy effects live in **Layer 1a** and the result (after all of layer 1) **defines the copiable values (613.2c)**. So a copy is computed first, then layers 2–7 (control, text, type, color, abilities, P/T) and counters/status are applied **on top of** that copy. This is why \"becomes a copy\" wipes prior layer-1 modifications but leaves counters/auras/Giant Growth, which re-apply in their own layers afterward.

---

## 4. Characteristic-Defining Abilities (CR 604.3)

A static ability is a **CDA** (604.3a) iff: (1) it defines an object's **color, subtypes, power, and/or toughness**; (2) it is **printed on the card**, granted to a token by its creator, or acquired via a **copy or text-changing effect**; (3) it does **not** directly affect other objects' characteristics; (4) it is **not** an ability the object grants to itself; and (5) it does **not** set the values only conditionally.

Why CDAs are special for an engine:
- **They function in all zones** and even outside the game / before the game begins (604.3) — power/color of a card in hand, graveyard, exile, on the stack, etc. is defined by the CDA. Non-CDA continuous effects mostly apply only on the battlefield.
- **They factor into copiable values** — a CDA is part of the printed/acquired rules text, so it is copied (707.2) and re-derives its value; CDA-defined P/T applies in sublayer **7a** (before sets in 7b), and CDA effects in layers 2–6 apply first per **613.3**.
- **They are segregated in dependency** (613.8a clause c): a CDA-sourced effect and a non-CDA effect never form a dependency on each other (dependency only forms when neither or both are from CDAs).

---

## 5. Gotcha cases that break naive engines

- **Dependency vs timestamp (Blood Moon vs Urborg-style):** Within layer 4, \"nonbasic lands are Mountains\" and \"all lands are also Swamps\" can depend on each other; applying one changes what the other affects. Engine must compute dependency (613.8a) per-evaluation, not just sort by timestamp. Mutual loops fall back to timestamp order (613.8b).
- **Becomes-a-copy + counters:** \"becomes a copy of X\" is Layer 1a → it redefines copiable values, but **counters are not copied (707.2)** and re-apply in 7c. A Clone of a 1/1 with two +1/+1 counters is 3/3 — naive engines that snapshot final P/T into the copy get it wrong.
- **Animate-land + type change:** Animating a land (layer 4 add creature type / layer 7b set P/T) interacts with later \"lands you control are 0/0\" etc. Order: type in layer 4, P/T set in 7b, counters in 7c. Setting P/T after counters were \"already there\" must not erase them — the set is earlier in the sublayer order, so counters apply on top numerically.
- **Set vs modify ordering (Humility + Glorious Anthem):** Humility sets base P/T to 1/1 (7b) and removes abilities (layer 6); an anthem +1/+1 (7c) applies after, giving 2/2. A naive \"apply effects in timestamp order regardless of kind\" engine produces the wrong number whenever a set and a modify coexist.
- **Humility + Opalescence (classic dependency/loop):** Opalescence makes enchantments creatures with P/T = mana value (layer 4 type change + layer 7 P/T), Humility sets creatures to 1/1 and strips abilities. Whether Opalescence's own ability still applies depends on whether Humility removed it — a layer-6/layer-7 dependency tangle. Must re-evaluate after each application (613.8c). Note 613.6: an effect that has started applying to a set of objects continues to apply to that set in later layers even if its source is removed.
- **Ability add then remove (layer 6):** Two effects in the same layer 6 — one grants flying, one removes all abilities — resolve by timestamp (after CDA-first per 613.3); later wins. Removing an ability can remove a CDA-granting ability, changing downstream layer 1/7 results — but per 613.6 an effect that already locked its object set keeps applying to that set in later layers; the engine recomputes characteristic VALUES continuously (613.5) without dropping in-progress effects' object sets.
- **Re-timestamping:** Re-attaching an Aura/Equipment or flipping/transforming gives a new timestamp (613.7), reshuffling intra-layer order. A cached-order engine becomes stale.

---

## 6. Engine implication — CONFIRMED

Current characteristics **must be a pure function**, recomputed on every query (consistent with 613.5: application is continuous and automatic), of:

```
characteristics(perm) = applyLayers(
    copiableBase,            // printed text + CDAs + face-down (layer 1 result, 613.2c)
    orderedContinuousEffects,// active effects, each tagged with layer/sublayer + timestamp
    counters,                // layer 6 keyword counters + layer 7c +1/+1/-1/-1
    status                   // tapped, flipped, face-down, phased, summoning sickness
)
```

The engine must **not** store final P/T/types/abilities on the permanent. Required inputs to the function:

1. **Copiable base** — printed characteristics, CDAs, any baked-in copy effect, face-down state (the layer-1 fixed point per 613.2c).
2. **The full set of currently-applicable continuous effects**, each carrying: its **layer** (and sublayer for 1 and 7), its **timestamp**, its **dependency-relevant predicate** (what it applies to / what it does), whether its source is a **CDA**, and (per 613.6) the **object set it locked onto** when it began applying.
3. **Counters** (layer 6 keyword + layer 7c numeric).
4. **Status** (tapped/flipped/face-down/phased; relevant for face-down layer 1b and for which effects are applicable).

The recomputation algorithm: build copiable values (layer 1a copy → 1b face-down → 1c freeze base), then for layers 2→7 apply effects; within each layer order **CDAs first (613.3, or 7a for P/T CDAs)**, then by **dependency (613.8) before timestamp (613.7) as tiebreak/fallback**, **re-evaluating dependency after each application (613.8c)**, breaking mutual loops by timestamp (613.8b). P/T uses the strict 7a→7b→7c→7d sub-order (613.4). Effects on players (613.10) and on the rules of the game (613.11) are handled in timestamp order after object characteristics are computed.

Because dependency and applicability are **state-dependent** (an effect's target set can change as earlier effects apply, subject to the 613.6 lock-in), the order is **not** statically cacheable per-game; it must be derived during each evaluation. Memoization is only safe if invalidated on any change to effects, counters, status, or timestamps. So: **pure function, recomputed on query, with the four inputs above — confirmed.**

---

Sources:
- [CR 613 — Interaction of Continuous Effects (ancestral.vision)](https://ancestral.vision/spells-abilities-and-effects/interaction-of-continuous-effects.html)
- [CR 707 — Copying Objects (ancestral.vision)](https://ancestral.vision/additional-rules/copying-objects.html)
- [Hyperlinked Comprehensive Rules (yawgatog.com)](https://yawgatog.com/resources/magic-rules/)
- [Characteristic-defining ability (MTG Wiki)](https://mtg.wiki/page/Characteristic-defining_ability)
- [How Layers Work in MTG (Pocket Judge)](https://www.pocket-judge.com/guides/how-layers-work)
- [Layers, Part One: Copiable Values (Quiet Speculation)](https://www.quietspeculation.com/2022/01/layers-part-one-copiable-values/)
- [Layer (MTG Wiki)](https://mtg.wiki/page/Layer)