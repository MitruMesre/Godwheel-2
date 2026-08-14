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
- **Elemental Wheel:** cosmetic only. `wheel::attack_display_color`
  looks at the elements contributed by an attack's base + combo cards:
  exactly one distinct element -> that color; zero or 2+ distinct
  elements -> black. No blocking, no resistances, no combining logic.
  `wheel::combine_elements_for_attack` / `filter_elements_for_defense`
  (the old `todo!()`s) are gone -- not needed until the full Wheel comes
  back.
- **Cards:** still TOML + serde, still per-mod (`mods/<name>/cards.toml`
  + `mods/<name>/locale/<lang>/cards.toml`), same as originally planned
  -- just with the syntax bugs fixed. A card is either a **combat card**
  (`combat.atk`/`combat.def`, resolved by summing atk vs def across the
  attack/defense exchange) or an **effect card** (`effect`, a one-off
  `CardEffect` resolved directly against a target) -- not both, until a
  real card needs it.
- **Cut for now:** `Afflict`/status effects (would need a status-effect
  store on Player), `DeckCategory`/multiple named pools (`count` on
  `CardDefinition` just says how many copies are in the single base
  pool), `HandCard`/`AllEnemies`/`AllPlayers` targeting (only
  `SelfTarget`/`Enemy` are used with one human + one bot).
- **`src/bin/cli.rs`:** a stdin console harness -- `cargo run --bin cli`
  from the project root. Lets you actually play the loop end to end
  before any wasm/`index.html` wiring happens. `index.html` is still
  the disconnected static mockup it always was; wiring it up via
  wasm-bindgen is the next phase after the CLI loop feels right.
- Not built/tested in this pass: no Rust toolchain was available to run
  `cargo build`, so the code hasn't been compiler-checked. Run
  `cargo build` first and expect to fix a few small things.