Here's the architecture discussion summarized:

**Effect system — two tiers, split by whether behavior composes or not:**
- **Composable stats** (atk, def, element) — plain numeric/enum fields that combo cards additively (or XOR, for elements) contribute to a shared accumulator. This is genuine composition, so no dispatch needed — the resolver just sums/combines fields directly.
- **One-off effects** (curses, miracles, sell, summon) — dispatched through a single effect-tag per card/case, not a list of independently-checked flags. Compound-sounding names like `boostHPOrDealDamage` turned out to be bespoke branching logic for one specific card, not generic combinators — so the right model is "one effect, some parameter fields (value, curseId, element, target)," not a flag list. This is a mild, acceptable DRY violation, since forcing branch/conditional logic into a generic composition system tends to require inventing a mini scripting language, which is worse.

**Closed vs. open extensibility:**
- A Rust **enum of primitives** (`DealDamage`, `AddCurse(CurseId)`, `SetElement`, etc.) stays permanently closed and exhaustive — since mods can only *use* existing effects, never add new ones, this enum never needs to grow for modding reasons, only when the engine itself gains new capability.
- **Cards themselves aren't enum variants** — they're Lua data/scripts that call into this fixed primitive surface (`engine.deal_damage(...)`, `engine.add_curse(...)`). Vanilla cards are just the first mod, declared the same way third-party mods would be.
- **Phases** are an ordered list mods can splice into by name; **events** are a string-keyed hook table (`hooks["on_death"] = [...]`) the engine fires at specific points, without needing to know who's listening. Both are open-ended without touching the closed primitive enum.

**Card data shape:** nested structs for genuinely reused groupings — `target_data` (starting target, count, filter) and `combat`/`stats` (atk, def, element) — each validated independently on load; behavior (`on_play`, `hooks`) stays flat, not nested, since it's code, not data.

<!-- **Lua VM:** leaning toward `piccolo` (pure-Rust, sandboxed, wasm-friendly) over `mlua`'s vendored C Lua.

**Mod packaging:** a mod splits into `logic.lua` -->
edit: piccolo seems insecure
mod only really needs to just declare a struct, i think. so something like toml
(host-only, drives simulation) plus `assets/` and `locale/` (needed by every peer, for rendering only) — since only the host actually simulates, only the host needs the logic; everyone else just needs to display results.

**Networking:** host-authoritative, not deterministic lockstep — rejected lockstep specifically because your game has hidden per-player state (hands), and lockstep requires every peer to simulate the full true state locally, leaking hands to anyone willing to read client memory; it also has no safe resync path for a no-autosave competitive match the way Factorio's world-state lockstep does. Host-authoritative avoids both problems and keeps the door open to swapping the trusted "host" for a real dedicated server later (for ranked PvP) without rearchitecting — same code, different deployment. Transport: likely WebRTC via something like Trystero for free serverless signaling, to avoid hosting/paying for a server.

## MVP (Phase 1) -- scope cuts to get a working loop

Everything below is deliberately smaller than the full design above, to
get something actually playable before tackling the hard parts.

- **Multiplayer:** none. Single process, one human vs one dumb bot.
  Networking is Phase 2+; `GameState` is written so it *could* later sit
  behind a network boundary without an API change, but nothing wired.
- **There is no separate "non-combat effect" track for HP.** A heal is
  just an attack with negative `combat.atk` -- you can attack yourself,
  or "heal" an enemy, same as any other card. `CardEffect` only covers
  MP/gold (`RestoreMana`/`RestoreMoney`) plus `RestoreHealth`, which
  exists solely for hooks (Sun Amulet's on_death) since those fire
  outside the normal play/target/defend pipeline entirely.
- **`TargetKind` is a bot-only hint**, not an enforced target. Humans
  freely choose self or enemy for any playable card at play time.
- **Spells are called miracles.** A miracle is just a combat card with
  `mana_cost > 0` and `persistent = true` -- nothing more special than
  that.
- **Hand is 18 fixed slots** (`Vec<Option<String>>`, col = slot%9, row =
  slot/9), not a loosely-ordered list. Playing N cards (base + combos)
  discards the non-persistent ones and draws N replacements into the
  front-most empty slots; persistent (miracle) cards instead relocate to
  the back-most empty slot instead of being discarded -- which is why
  playing one miracle among several cards nets +1 occupied slot overall.
- **Defense is free multi-select**, separate from the attack's
  base+combo model: any number of DEF cards can be stacked (summed), no
  combo restriction. Exclusive defenses (a "reflect" that can't combine
  with anything else) aren't modeled in the MVP.
- **Elemental Wheel:** cosmetic only. `wheel::attack_display_color`
  looks at the elements contributed by an attack's base + combo cards:
  exactly one distinct element -> that color; zero or 2+ distinct
  elements -> black. No blocking, no resistances, no combining logic.
- **Bot AI, deliberately dumb:**
  - Own turn: pick a random playable weapon (non-combo, any signed atk),
    attach *every* combo card currently in hand, target per that base
    card's `TargetKind` hint (defaulting to the enemy).
  - Defense: closest-subset-sum of its DEF cards to the incoming atk
    (a bounded subset-sum DP, not literal brute force, but same result).
- **Cut for now:** `DeckCategory`/multiple named pools (`count` on
  `CardDefinition` just says how many copies are in the single base
  pool), Gambler's-Coin-style random-outcome cards.
- **`src/bin/cli.rs`:** a stdin console harness -- `cargo run --bin cli`.
  The base mod is baked into the binary via `include_str!`
  (`CardRegistry::load_embedded_base`), so it doesn't depend on a
  working directory; `CardRegistry::load_mod` (filesystem-based) is kept
  for real third-party mods later.
- **`index.html` is wired up.** `src/wasm.rs` exposes a small
  JSON-string-in/JSON-string-out API (`WasmGame`) rather than a
  fine-grained wasm-bindgen type surface, specifically because nothing
  in this pass could be compiler-checked -- one serialization boundary
  is a lot less risky than marshaling a dozen Vec<struct> types through
  wasm-bindgen's own type system. `index.html`'s script now imports
  `./pkg/godwheel_2.js` (from `wasm-pack build --dev --target web`) and
  drives the existing visual scaffold from real game state; a small
  action-bar element and card selection/eligibility outlines were added
  for the play/target/defend interaction, but no existing CSS rule was
  changed. The multi-player roster editor (add/remove player, editable
  name/HP/MP/gold) was removed since the MVP is strictly 1v1 -- the
  panel now always shows exactly "You" and "Bot", read-only.
- Not built/tested in this pass: no Rust toolchain was available (and
  wasm-pack/rustup's own domains aren't reachable from here either), so
  none of this has been compiled -- not the native lib/CLI, not the wasm
  build. Checked by hand instead: brace/paren/bracket balance with
  string/comment content stripped (all clean) and `node --check` on the
  extracted `<script type="module">` body (clean). Run `cargo check`
  first and expect to fix a few small things before `wasm-pack build`.