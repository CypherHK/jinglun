# Candidate A — stateless Rust, browser-owned saves

## Problem and grounding

Deliver the accepted fixed 24-month governance prototype without moving map authorship or duplicating economic rules. Current generation accepts `MapConfig`, validates it, and runs CPU work under a semaphore (`crates/server/src/main.rs:31–61`); routes currently impose a **4096-byte global body cap** (`:74–84`). `MapConfig` supports 100 cells (`crates/world/src/lib.rs:64–78`), but defaults to 500 (`:32–39`). New-game must explicitly select 100.

Generation owns geography; gameplay owns its immutable interpretation and every socioeconomic mutation. `WorldMap` is Serialize-only and contains `&'static str`, directly and transitively (`crates/world/src/lib.rs:105–149,152–187`). Adding Deserialize is neither necessary nor sound as an assumed solution. The frontend shape is already separate (`frontend/src/types.ts:99–129`), and generation response checking is partial (`frontend/src/api.ts:8–42`). App owns current map and async replacement (`frontend/src/ui/App.tsx:21–39,46–67`); seasonal playback changes only a view month (`:72–81,117–122`); current export saves only map JSON (`:129–140`). Preserve these boundaries. Historical intent beyond these source facts is not established.

## Usage first

HTTP shell, pseudocode:

```rust
// POST /api/games/start; fixed scenario ID, optional personal goals
let game = Game::start(catalog.scenario(id)?, rules)?;
Json(game.export()?)

// POST /api/games/advance; prior complete envelope + command batch
let game = Game::restore(request.save_bytes(), catalog)?;
let next = game.advance(request.expected_revision(), request.commands()?)?;
Json(next.export()?)

// POST /api/games/validate; import never advances time
Json(Game::restore(upload.bytes(), catalog)?.export()?)
```

Browser caller:

```ts
const base = session.accepted; // immutable accepted envelope
const pending = { epoch: session.epoch, base: base.revision, commands };
const candidate = await advanceGame(base, commands);
// Only current epoch/base may replace session. Persist first, then display.
await acceptCandidate(pending, candidate);
```

Transport parsers own wire schemas; callers never coordinate separate reserve/build/tax operations. Rust returns reports and allowed policy actions; TS renders and submits intentions, never calculates economic outcomes.

## Shape and ownership

Add `crates/governance` using existing serde/serde_json only. Public surface:

```rust
pub struct Game { /* private immutable map, validated scenario/rules, state */ }
pub struct Revision { /* private month, state checksum */ }
pub struct Commands { /* private normalized typed policy/public-work actions */ }
pub struct SaveEnvelope { /* serializable private fields; inspect via getters */ }
impl Game {
    pub fn start(scenario: &Scenario, rules: &Rules) -> Result<Self, GameError>;
    pub fn restore(bytes: &[u8], catalog: &Catalog) -> Result<Self, GameError>;
    pub fn advance(&self, expected: Revision, commands: Commands)
        -> Result<Self, GameError>;
    pub fn export(&self) -> Result<SaveEnvelope, GameError>;
}
fn step(state: &State, commands: &Commands, context: &Context)
    -> Result<Transition, GameError>; // pure, private
```

Modules own knowledge, not monthly phases: `lib.rs` owns Game/lifecycle; `snapshot.rs` owns envelope parsing, canonical bytes, map adapter and validation; `economy.rs` owns state, ledger, allocation, commands and monthly transition; `scenario.rs` owns the single bundled scenario/rules catalog. Stages remain functions inside economy, not public subsystem APIs. Public DTO accessors expose reports without exposing writable domain internals. This deep interface hides allocation, conservation, validation and compatibility behind one operation, per boundary-discipline.

`MapSnapshot` contains the **complete owned JSON map**, generation parameters/version and checksum. Private parser validates the supported map schema into an owned display snapshot plus immutable gameplay projection: stable IDs, adjacency/passages, usable land, construction cost inputs, twelve climate/source ceilings. It never reconstructs a `WorldMap`, leaks strings to `'static`, or regenerates imported maps. Preserve all original map fields; projection is derived on restore, never an independent authority. Convert physical capacities once with documented conservative rounding to integer simulation units. Define the full accepted-map schema and conversion table before implementation; treating arbitrary JSON as trusted geography is unacceptable.

State uses stable-ID ordered collections and checked integer money/material/capacity arithmetic. Owner accounts, household/firm inventories, land reservations, source allocations, projects/escrow, pending deliveries/graduations/contracts, institutions, AI, plan baseline and reports belong to Rust. Double-entry transfers include external counterparties. Canonical command order and stable-ID integer remainders remove click/order ambiguity; reject conflicting commands rather than silently choosing. No wall clock, randomized hash iteration or floating simulation arithmetic; serialize PRNG state only if gameplay actually needs randomness.

## Monthly and persistence contracts

`month=0` is initialized baseline; advance computes month 1. Seasonal index is `(next_month-1)%12`; App's preview month remains independent. The accepted stage ordering and next-month activation rules come from `minimal-governance-v2.md` sections 3–4. Derive total/per-person/fixed-price reports from final ledger and state, including explicit aid 8→0 at month 13, all investment rejection reasons, three-country baselines, 12/24 checkpoints and post-24 continuation. Personal goals are report data, not arbitrary executable conditions.

Envelope includes save/scenario/rules versions, exact bundled rules content/checksum, app/generator provenance, complete map, initialized scenario inputs, current state, baseline, normalized command history and monthly ledger/reports. Restore checks catalog compatibility; embedded rules are provenance, never permission to execute unknown rules. Replay from stored initialization must reproduce accepted state and reports. Unknown versions fail without touching the existing session or original import file.

Use documented canonical JSON and a small explicitly noncryptographic checksum implemented without new dependencies; it detects accidental changes, **does not prove authenticity or prevent save editing**. Bound bytes before parse, recursion, all vectors/strings, IDs/references, 12-month arrays, command count, states/history and checked numeric ranges; validate land/water/order/budget conservation. Start with explicit engineering caps of 16 MiB envelope, 100 cells, 64 commands/month and 1200 elapsed months; measure fixture size before freezing. At the cap allow export/view, reject advance explicitly. These are support limits, not balance values. Override body limits only on gameplay routes; retain generation's 4 KiB cap and bounded blocking work permits.

Browser owns one accepted envelope in IndexedDB, replaced in one transaction. Failure leaves the old envelope; quota failure keeps candidate unaccepted and offers export/retry. No backend sessions or commits exist. Duplicate POST of identical base/commands produces identical candidate. Browser accepts only matching epoch and base revision, so duplicate responses cannot advance twice. Disconnect/restart retries from the persisted base. New-game/import/map regeneration increments epoch and invalidates pending responses; only a successful validated replacement atomically replaces map **and** game. Standalone generated maps remain map-preview mode. Multiple tabs use separate saves; no implicit shared-slot overwrite.

## Tradeoffs, alternative and risks

Accept larger requests and validation cost for zero server persistence and simple retry semantics. A server-owned session alternative hides large transfer but introduces session identity, durable commits, deployment/storage ownership and recovery beyond this slice. No candidate synthesis is claimed here; this is the independently evaluated A proposal.

Blocking data gaps for a playable 24-month claim: exact fixed seed/ownership, initial population, demand budgets, material/food origins, labor/teachers and construction conversion are still unspecified. The proposed 30 tax baseline must derive from those accounts. Two new firms yield tax 6 before added services; route viability depends on maintenance, timing and real demand. Do not claim A/B solvability until deterministic fixtures establish it.

## First implementation unit and parallel contract

Freeze wire examples, version/error codes, map adapter schema, command variants, report fields, canonicalization and bounds first. Backend then owns the new crate/server routes; frontend independently owns new `game-api.ts`, game types, save/session controller and report components against those fixtures. Preserve comparison files. First reviewable unit is fixed initialization + empty monthly transition + one firm investment/escrow transaction with conservation/retry/import replay evidence. Serialize App integration after both contracts stabilize; serialize complete economic stage integration and A/B calibration after primitive ledgers, not concurrent independent writers to monthly state.
