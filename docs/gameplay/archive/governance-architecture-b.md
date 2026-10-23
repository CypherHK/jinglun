# Candidate B — server-owned disposable session, local recovery snapshot

## Problem and grounding

The server owns the only live simulation; the browser owns a durable recovery copy, command drafts and presentation. A session is disposable, with no database or server files. This replaces whole-state stateless requests.

Today generation is a pure world function; HTTP validates `MapConfig`, acquires a two-slot semaphore and generates in `spawn_blocking` (`crates/world/src/lib.rs:1,259`; `crates/server/src/main.rs:31–61,74–84`). The UI owns the returned map and generation identity (`frontend/src/ui/App.tsx:21–28,46–59`). Existing climate playback changes a 0–11 preview value on a timer (`App.tsx:72–81`). Preserve generation and introduce a sibling gameplay boundary. Construction explicitly supplies natural constraints, not buildings or water rights (`crates/world/src/construction.rs:1,25–46`).

## Usage first

Browser caller examples, with HTTP hidden by `gameplay/api.ts`:

```ts
const receipt = await startScenario('transition-24-v1');
await saveRecovery(receipt.checkpoint); // IndexedDB, then expose next-month button
const next = await advanceMonth(receipt.session, {
  expectedRevision: receipt.revision, requestId, commands: draft
});
await saveRecovery(next.checkpoint); // atomically replaces snapshot + clears pending command
const restored = await resumeGame(await loadRecovery());
```

Before sending a month, persist its exact request together with the last acknowledged checkpoint. A timeout keeps this pending record. Retry unchanged; do not generate another ID or optimistically advance. Reload resumes the checkpoint and resubmits the pending request. UI displays server reports; it never prices projects or awards taxes.

## Shape and ownership

One new `crates/gameplay` owns domain types, validation, fixed scenario/rules, simulation, reports and save encoding. Internals remain private. `crates/server/src/gameplay.rs` owns bounded session registry and HTTP DTOs. `frontend/src/gameplay/` owns transport parsing, IndexedDB recovery and a panel; `App.tsx` only mounts it and selects a displayed map. No edits to world kernel, no packages added/upgraded.

```rust
// Public domain API, no HTTP or serde_json::Value exposed.
pub struct Game { /* immutable binding/rules + current State + baseline/history */ }
pub struct SnapshotBytes(Vec<u8>); // constructor enforces encoded byte cap
pub struct Revision(u32);
pub struct MonthCommands { /* typed policy, projects, contracts, plan edits */ }
pub struct Transition { pub game: Game, pub report: MonthReport }
pub fn start(scenario: ScenarioId) -> Result<Game, Failure>;
pub fn step(game: &Game, commands: &MonthCommands) -> Result<Transition, Failure>;
pub fn resume(bytes: SnapshotBytes) -> Result<Game, Failure>;
impl Game {
  pub fn snapshot(&self) -> SnapshotBytes;
  pub fn view(&self) -> GameView;
}
// Bodies are deliberately not implemented in this design sketch.
```

`Game` owns indexed, stable-ID country/population/company/facility/project/account/stock/escrow/land/source/order tables, pending arrivals, AI commitments, policy effective dates, RNG state, announced pressure and baseline/history. Checked integer money/quantities and distinct CellId/SourceId/AccountId prevent accidental mixing; stable arrays/BTreeMaps avoid nondeterministic iteration. `MonthCommands` rejects duplicate/conflicting targets; explicit stable ordering controls allocation. Authoritative constraints live here once, per boundary-discipline and encode-lessons-in-structure.

`step` is pure and returns a candidate, including ledger and report. Internally execute proposal v2's fixed phases; use opening capacities and release arrivals/completions/graduates only into next month's state. Shared source ceilings, land, material, payable demand and subsidy escrow each have one reservation owner. Enterprises choose investments autonomously from policy constraints and forecasts; no player `BuildPrivateFactory` command. Neighbor rules consume the same scarce ledgers. Announced assistance ends at month 13; plan remains report criteria and editable aspirations, not a victory score.

Interface depth: callers submit an entire monthly intent and receive one receipt; allocation, validation, accounting, reports and persistence representation stay hidden. No public phase methods or caller-managed escrow.

## Map/save representation

`WorldMap` is Serialize-only and embeds static strings (`world/src/lib.rs:105–108,132–147,178–187`); do not derive Deserialize there. Bundle a reviewed full 100-cell JSON map fixture produced from fixed `MapConfig`, preserving all map output, seed/config and generator version. Gameplay's private owned map decoder uses Strings/owned enums, validates it and projects only required natural constraints into an immutable `MapBinding`. Reuse the original full map for display/export; projection is derived, never an independent save writer. Frozen fixture eliminates generator drift on resume. Generator remains 0.6.0; application 0.6.1 does not imply save compatibility.

Snapshot envelope contains save/scenario/rules versions, full map JSON, full scenario/rules contents, canonical-content digests, full state, baseline, all reports and accepted normalized commands. Use a documented fixed noncryptographic digest implemented with std for accidental corruption/deterministic comparison; compare exact canonical bytes for approved bundled map/rules provenance, so collisions never authorize a different fixture. This is local game data, not tamper-proof competitive scoring. An unsupported rules/map version is rejected with original bytes retained; no regeneration or silent migration. Existing environmental fingerprint is not sufficient save provenance (`frontend/src/types.ts:99–126`).

Import boundary limits: initially 16 MiB encoded, 100 map cells, 3 countries, 2 groups, bounded polygon/source/history/command/table counts, JSON nesting and string lengths; validate references, finite natural values, month arrays, arithmetic ranges, accounting/land/source invariants and report consistency. Resume reconstructs from approved initial state and saved accepted commands, then checks terminal state and reports against the full snapshot. Cap initial prototype at 240 simulated months, explicitly reported; month 24 permits continued observation. Confirm actual fixture size before fixing these bounds. No new dependency is required.

## Session protocol and failure behavior

`POST /api/game/sessions` accepts scenario or checkpoint; response includes opaque session ID, revision, view and complete checkpoint. `POST /api/game/sessions/:id/month` accepts expectedRevision, requestId and bounded command DTOs. Each success returns the same receipt shape. Small commands retain 4 KiB limit; import route alone permits 16 MiB, leaving map endpoint's current 4096-byte limit intact (`server/src/main.rs:82–84`).

Registry serializes operations per session, not across sessions; cap at 8 sessions with explicit expiry/busy errors. Keep the latest accepted request and response: same ID/body returns that receipt, same ID/different body rejects; other stale revisions return 409 without applying. Compute outside registry lock under existing CPU admission policy, then compare revision and atomically swap candidate plus retry receipt. Disconnect does not cancel an admitted operation. UI never has two outstanding month requests. Server restart/expiry returns session-missing; browser restores acknowledged snapshot and exact pending command into a fresh session. Lost unacknowledged computation is reproduced deterministically, without repeated external effects. No promise of cloud persistence or cross-device saves.

## Frontend contract and reports

`GameView` includes map, selected focus IDs, month/revision, country comparisons, plan baseline/targets, next-month consequences, outstanding projects and current/all monthly reports. Reports contain integer raw values, display units and cause/ledger references for income, employment, housing, clinic, skills, budget and safety; history includes total/per-capita/fixed-price series. TS formats and charts returned data. Generation comparison remains independently owned; gameplay never takes its mutable output as state. Rename current `month` concept to `previewMonth` at integration; playback cannot call advance. Simulation month changes only via explicit receipt.

## Tradeoffs, alternatives and risks

We accept server registry/retry mechanics for small routine requests and one runtime owner. Browser-owned stateless full-step hides session expiry but exposes whole-state transport and repeat decoding every turn. Command-log-only saves are smaller but fail full-snapshot portability and make every load depend exclusively on replay; retain both snapshot and bounded history instead. Red-flag review finds no dual writer, importable internals, phase API or synced economic rules; TS DTOs still require contract fixtures to prevent wire drift.

Blocking data gaps before playable acceptance: reviewed exact cell ownership/connectivity and source binding; funded existing tax-paying activity, external buyers/suppliers, teachers and opening stocks; precise partial allocation/rounding and safety-contract timing. The proposed two routes and starting finances are assumptions, not validated balance. Two factories yielding tax 6 leaves only 2 beyond the pressure deficit before added maintenance; subsidy 20 per factory also consumes substantial starting liquidity. Replay must expose failures rather than fabricate demand/resources.

## First reviewable unit and dependency contract

After comparison lands, add gameplay crate with frozen fixture ingestion, owned map binding, versioned save/resume and deterministic empty month plus one investment/subsidy transaction; verify round-trip/replay, duplicate accounting and shared-resource caps. Backend can implement this and session DTO fixtures in isolation. Frontend can independently implement receipt parsing, pending-request recovery and report panel against those fixtures. Integrating routes depends on domain success; connecting advance depends on stable receipts/recovery; full 24-month A/B acceptance depends on complete scenario inventories and all monthly rules. This is untested architecture.
