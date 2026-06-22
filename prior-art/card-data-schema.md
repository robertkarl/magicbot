# MTG Card Data Schemas for a Rules-Engine CardDb

Research synthesized from the Scryfall and MTGJSON data models (Scryfall docs are 403 to bots; verified via Scryfall API mirrors, MTGJSON, and typed client libraries that mirror the schema 1:1). Rules claims checked against the Comprehensive Rules (yawgatog.com mirror).

---

## 1. Core card fields (structured vs. free text)

Scryfall's `Card` object and MTGJSON's `Card (Set)` model carry the same gameplay-relevant core. Mapping:

| Concept | Scryfall | MTGJSON | Structured? |
|---|---|---|---|
| Display name | `name` (`//` joins faces) | `name` (`//` joins faces) + per-face `faceName` | Structured string |
| Mana cost | `mana_cost` e.g. `{1}{B}` | `manaCost` | **Structured** (symbol-delimited, machine-parseable) |
| Mana value (a.k.a. CMC) | `cmc` (numeric; CR 202.3) | `manaValue`, plus per-face `faceManaValue` | Structured numeric |
| Type line | `type_line` e.g. `Legendary Creature — Elf Warrior` | `type` (raw line) **plus** decomposed `supertypes`, `types`, `subtypes` arrays | Scryfall: semi-structured text; **MTGJSON pre-splits it** |
| Rules text | `oracle_text` | `text` | **Free text** (the hard part) |
| Power / Toughness | `power`, `toughness` (**strings**, e.g. `"*"`, `"1+*"`) | `power`, `toughness` (strings) | Semi-structured strings (not always numeric) |
| Loyalty | `loyalty` (string) | `loyalty` | Semi-structured |
| Defense (battles) | `defense` (face-level) | `defense` | Semi-structured |
| Colors | `colors` (array of W/U/B/R/G) | `colors` | **Structured** |
| Color identity | `color_identity` | `colorIdentity` | **Structured** (a Commander deck-building concept, CR 903.4 — see note below) |
| Keywords | `keywords` (array) | `keywords` (array) | **Structured but incomplete** — a curated keyword list, not a full ability model |
| Produced mana | `produced_mana` (array) | (n/a) | Structured, best-effort |

Key takeaways:
- Reliably structured: `mana_cost`/`manaCost`, `cmc`/`manaValue`, `colors`, `color_identity`, `keywords`, and (in MTGJSON only) the decomposed type arrays.
- Free text / not behaviorally structured: `oracle_text`/`text`. The engine must parse or hand-encode this.
- Gotcha: `power`/`toughness`/`loyalty`/`defense` are **strings**, not ints, to allow values produced by characteristic-defining abilities (CR 604.3): `*`, `1+*`, a printed `X`, `∞` (Un-cards), etc.
- `type_line`: Scryfall gives the raw line with an em-dash (`—`); MTGJSON additionally pre-splits into `supertypes`/`types`/`subtypes`, which is more convenient for an engine.

Terminology notes (rules-faithful):
- **Mana value (CR 202.3)** is the current term; the field name `cmc` ("converted mana cost") is the legacy term, deprecated since Strixhaven (2021). It is an integer for all tournament-legal cards; Scryfall types it as a decimal only because Un-set `{1/2}` symbols can yield fractional values.
- **Color (CR 105 / 202.2)** is what the rules engine consults during gameplay. **Color identity (CR 903.4)** is a separate, Commander-format deck-building concept — do not treat them as interchangeable.

Sources: [Scryfall Card Objects](https://scryfall.com/docs/api/cards), [MTGJSON Card (Set)](https://mtgjson.com/data-models/card/card-set/), [go-scryfall Card struct](https://pkg.go.dev/github.com/BlueMonday/go-scryfall), [CR 202.3 / 105 / 903.4 (yawgatog)](https://yawgatog.com/resources/magic-rules/).

> **Color-identity precision (CR 903.4):** color identity is the color(s) of any mana symbols in a card's **mana cost or rules text**, plus colors defined by its **color indicator** and by **color-setting characteristic-defining abilities**. **Reminder text is excluded** (CR 903.4c) — e.g. Extort and Blind Obedience are colorless / mono because their hybrid symbols appear only in reminder text. Scryfall/MTGJSON compute `color_identity`/`colorIdentity` this way (reminder text correctly ignored).

---

## 2. Multi-face card modeling

There are **two distinct mechanisms**, and the choice depends on whether the faces share one physical card object.

### A. `card_faces` array — faces that live on one card object
Used when both faces are the **same Scryfall printing / same physical card**. The root object holds shared data (`color_identity`, `cmc`, `oracle_id`, identifiers); the `card_faces[]` array holds per-face `name`, `mana_cost`, `type_line`, `oracle_text`, `power`, `toughness`, `loyalty`, `defense`, `colors`, and (for DFCs) per-face `image_uris`. Layouts that use `card_faces`:

- **`split`** — two halves (e.g. Fire // Ice); both faces share one object.
- **`aftermath`** — split variant where the second half is cast only from the graveyard and is exiled as it resolves; modeled as its own Scryfall layout `aftermath` with faces in `card_faces`.
- **`flip`** — Kamigawa flip cards; both halves on one upright object.
- **`transform`** — DFC that physically transforms (front/back).
- **`modal_dfc`** — MDFC, castable as either side. Root carries combined `color_identity`; each `card_faces` entry is an independently castable spell.
- **`adventure`** — creature + adventure-spell; two faces, single object.
- **`reversible_card`** — two **unrelated** sides; notable because each `card_faces` entry has its **own** `layout` field, and the two sides are treated as separate objects for gameplay/deckbuilding.
- **`double_faced_token`** — two token faces on one object.
- **`room`** (newer, DSK) — split-style enchantment with two doors; modeled with `card_faces`.
- **`case`**, **`class`**, **`saga`**, **`prototype`**, **`leveler`** — single-faced but multi-region; generally `normal`-style single objects (no `card_faces`), with the region structure living inside `oracle_text`.

Face identity within `card_faces` is by **array order + `name`**. For DFCs/MDFCs, index 0 = front, index 1 = back. There is no separate per-face oracle id on the root for most layouts (some clients expose a face-level `oracle_id` for MDFC halves).

In **MTGJSON**, by contrast, multi-face cards are **split into multiple Card objects** (one per face/side), linked by:
- `side` — `"a"`, `"b"`, etc., identifying which face.
- `faceName` — the individual face's name (vs. the combined `//` `name`).
- `otherFaceIds` — array of MTGJSON `uuid`s pointing to the sibling faces.

### B. `all_parts` / `otherFaceIds` — faces that are separate physical cards
Used for **meld**, where the two front halves and the melded back are three different physical cards:

- **`meld`** layout uses **`all_parts`** (NOT `card_faces`). `all_parts[]` is an array of Related Card objects; the `component` field is one of `meld_part` or `meld_result` (also `token`, `combo_piece` for other relationships). Each part is a separate full card object reachable by URI. (Verified: Gisela/Bruna are `meld_part`, Brisela is `meld_result`.)
- In MTGJSON, meld pieces are separate cards linked by `otherFaceIds`.

So: same-object faces → `card_faces` (Scryfall) / `side`+`otherFaceIds` (MTGJSON); separate-object relationships (meld, token generation) → `all_parts` (Scryfall) / `otherFaceIds` + relationship modeling (MTGJSON).

Sources: [Scryfall Layouts & Faces](https://scryfall.com/docs/api/layouts) ([accessible mirror](https://curso-r.github.io/scryr/articles/layouts.html)), [MTGJSON Card (Set)](https://mtgjson.com/data-models/card/card-set/).

---

## 3. Tokens

- **Scryfall**: tokens are first-class card objects with `layout: "token"` (or `double_faced_token`). They have their own `oracle_id`, `name`, `type_line` (e.g. `Token Creature — Soldier`), `power`/`toughness`, `colors`, and `oracle_text`. The **link from a real card to the token it makes** is via that card's `all_parts[]` with `component: "token"`. Tokens are also queryable directly (`is:token`).
- **MTGJSON**: tokens are catalogued in a per-set **`tokens`** array using the **Card (Token)** data model (a sibling of Card (Set)). Cross-references use `identifiers` and `otherFaceIds`.

For a rules engine, treat tokens as their own CardDefs keyed by token `oracle_id`, and resolve "makes a Soldier token" by following `all_parts`/the token catalog rather than parsing oracle text.

Sources: [MTGJSON Card (Token)](https://mtgjson.com/data-models/card/card-token/), [Scryfall Card Objects](https://scryfall.com/docs/api/cards).

---

## 4. Identity layers (which key means "same card")

| ID | Scryfall | MTGJSON | Granularity |
|---|---|---|---|
| **Oracle identity** | `oracle_id` (UUID) | `identifiers.scryfallOracleId` | **The gameplay-stable key.** Same across all printings/reprints of the same card; distinct between different cards sharing a name (tokens, Un- variants). |
| **Printing identity** | `id` (UUID) | `identifiers.scryfallId` + MTGJSON `uuid` | One specific printing (set + collector number + finish). Changes per reprint. Note Scryfall `id`: "cards with multiple faces are not unique" across faces. |
| **Gatherer** | `multiverse_ids` (array of ints) | `identifiers.multiverseId` | WotC Gatherer ids; **array** because multi-face cards have one per face; some cards lack them. |
| **Digital** | `mtgo_id`, `mtgo_foil_id`, `arena_id` | `mtgoId`, `mtgoFoilId`, `mtgArenaId` | Platform-specific printing ids. |
| Illustration | `illustration_id` | `scryfallIllustrationId` | Art-level, stable across reprints of same art. |
| MTGJSON uuid | (n/a) | `uuid` (v5) | MTGJSON's per-printing key. |

**For a rules engine, `oracle_id` (Scryfall) / `scryfallOracleId` (MTGJSON) is the canonical "this is the same card" key.** Printing-level ids (`id`, `uuid`, `multiverse_ids`) only matter for art/collection/provenance, not rules behavior.

Sources: [Scryfall Card Objects](https://scryfall.com/docs/api/cards), [MTGJSON Identifiers](https://mtgjson.com/data-models/identifiers/).

---

## 5. What these schemas do NOT capture (you must encode it)

Both schemas are **catalog/printing** data, not a rules model. Missing entirely:

- **Ability semantics.** `oracle_text` is free prose. Triggered/activated/static abilities, costs, targets, timing, replacement effects, and resolution are **not** structured. `keywords` is a flat tag list, not parameterized (e.g. it says "Ward" but not "Ward {2}"; "Annihilator" but not its count).
- **The actual rules of mechanics** (the stack, priority, the layer system / characteristic-defining abilities, state-based actions, combat, mana abilities, zones).
- **Costs beyond mana**: tap, sacrifice, additional/alternative costs are buried in text.
- **Targeting and legality constraints.**
- **P/T as computed values** — `*`/printed `X`/`1+*` are strings; the characteristic-defining ability (CR 604.3) must be implemented.
- **Mode/region structure for sagas, classes, levelers, cases, battles, planeswalker loyalty abilities** — only present as text/layout hints.
- **Token contents fully** — you get the token's stats, but "what triggers make it and how many" is in the source card's text.
- **Replacement/continuous effect interactions, copy rules, layer ordering (the layer system).**

In short: these give you reliable **static characteristics** (name, cost, types, colors, P/T, identity), but **zero executable behavior**.

---

## 6. Recommendation for the static `CardDef`

**Key the CardDef on `oracle_id`** (Scryfall) / `scryfallOracleId` (MTGJSON). It is the stable, reprint-invariant, name-collision-safe identity. Keep printing ids (`scryfallId`/`uuid`, `multiverse_ids`) only as optional metadata on a separate Printing record.

**Ingest from MTGJSON preferentially** for gameplay (it pre-decomposes type lines and splits faces cleanly), cross-checking with Scryfall `oracle_id`.

Suggested `CardDef` fields (per face where multi-faced):

- `def_id` ← `oracle_id` / `scryfallOracleId`
- `name`, and `faces[]` for multi-face cards (each: `name`, `mana_cost`, `mana_value`, `type_line`)
- `mana_cost` (keep the `{..}` symbol string — already structured) and parsed `{cost}` vector
- `mana_value` (`cmc`; CR 202.3 — store numeric, allow fractional for Un-cards)
- Decomposed types: `supertypes[]`, `types[]`, `subtypes[]` (from MTGJSON; else parse `type_line`)
- `oracle_text` (retain raw text as the source for your hand-/auto-encoded ability model)
- `power`, `toughness`, `loyalty`, `defense` (**store as strings**, plus a parsed-or-CDA flag; CDA = CR 604.3)
- `colors[]` (gameplay color, CR 105/202.2), `color_identity[]` (Commander concept, CR 903.4 — reminder text excluded)
- `keywords[]` (as hints, not behavior)
- `layout` (drives face/relationship handling)
- Multi-face linkage: `faces[]` order (0=front) for `card_faces` layouts; `related[]` (component + target def_id) for `meld`/token relationships
- `produced_mana[]` (hint)

Plus a **separate, hand-authored `Behavior` layer** keyed by the same `def_id`, holding the executable abilities/costs/effects that the catalog can't express. Design the loader so an unencoded card still has full static characteristics and merely lacks behavior — that lets the engine boot on the entire card pool and you encode behavior incrementally.

---

### Sources
- [Scryfall — Card Objects](https://scryfall.com/docs/api/cards)
- [Scryfall — Layouts and Faces](https://scryfall.com/docs/api/layouts) (accessible mirrors: [curso-r/scryr](https://curso-r.github.io/scryr/articles/layouts.html), [CRAN scryr vignette](https://cran.r-project.org/web/packages/scryr/vignettes/layouts.html))
- [MTGJSON — Card (Set)](https://mtgjson.com/data-models/card/card-set/)
- [MTGJSON — Card (Token)](https://mtgjson.com/data-models/card/card-token/)
- [MTGJSON — Identifiers](https://mtgjson.com/data-models/identifiers/)
- [go-scryfall typed Card/CardFace structs](https://pkg.go.dev/github.com/BlueMonday/go-scryfall)
- [Comprehensive Rules (yawgatog mirror)](https://yawgatog.com/resources/magic-rules/) — CR 105/202.2 (color), 202.3 (mana value), 604.3 (characteristic-defining abilities), 903.4 (color identity; reminder text excluded)