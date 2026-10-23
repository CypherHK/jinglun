# Gameplay contribution checkpoint

## Delivered

Candidate-region comparison is committed as `05a5fd3bbbbabab98d4aaa02a14daf5bbae232da`, application version `0.6.1-gameplay`, generator version still `0.6.0`.

- Worktree: `/workspace/jinglun-gameplay`, branch `gameplay/0.6.1-candidate-comparison`.
- Baseline: upstream `c2dc9f0d9ab404214952264c9313970750c423a5`, application 0.6.1.
- Review PR: https://github.com/CypherHK/jinglun/pull/1 . Open, ready, not merged; exact head and base verified after creation. One commit, ten changed files, 274 additions, four deletions.
- Its base is `CypherHK:gameplay/0.6.1-review-base`, which points to upstream 0.6.1. The fork's main is still the older `fb92387f308810239b36174cb27b5778e572bb6b` and was not changed.
- Upstream PR creation returned GitHub `403 Resource not accessible by integration`. The Cloud CLI's request to `api.github.com/user` also returned Forbidden. No upstream PR was created. The fork PR body mentions `@CircuitMurderer` and links the upstream comparison page. Notification receipt is unknown.
- The PR was attached to the task with `codex_app.attach_artifact`.

## Verified

- `npm run check`, `npm run test:web`: 18 tests passed after the keyboard fix.
- `npm run build`: passed, including the final CSS-only sticky metric column change.
- Baseline `cargo test --workspace --locked --offline`: world 21 tests and server 1 test passed. Rust was not changed in this feature.
- Release Rust server build passed.
- `verify_comparison.py` passed against the real Rust server and production frontend using system Chromium 154. It covers ordered bounded pins, removal focus, period/layer/playback isolation, keyboard navigation, unique shared water sources and numeric conversion, mixed/ocean land metrics, drawer/theme/selection preservation, 390px scrolling, failed regeneration preservation, same-seed success reset, and clear focus.
- `comparison-browser-results.json` records 11 acceptance groups and zero uncaught page errors. Its one console HTTP 503 was intentionally injected for the failure case.
- `comparison-visual-results.json` separately verifies the final sticky column in both themes at 390px. Screenshots are `comparison-mobile-dark.png` and `comparison-mobile-light.png`.
- A fresh GPT-6.1 Sol review found the scrolling-key conflict, which was reproduced and fixed. Fresh comment review recommended zero deletions. `deslop` from cursor-team-kit is unavailable; scoped diff inspection was used, not an automated deslop run.
- `git diff --check` passed. Both repository worktrees are clean. The temporary node_modules symlink was removed without changing the existing dependencies. Local preview and Rust server were stopped. No dependencies were installed or upgraded, and nothing was merged or deployed.

## Accepted gameplay direction

The user wants all four goals, developed in reviewable stages: region comparison; fixed scenario and national plan; comprehensive monthly governance; reports/trends/save-load. Goal 1 is implemented. Goals 2–4 have a revised proposal and architecture alternatives, not an implemented monthly game.

User decisions: one announced fiscal or diplomatic pressure, enterprises make investment decisions while government guides via policy/subsidy, 100 generated cells with player focus on 8–12. The author owns map design and generation. New subagents must use GPT-6.1 Sol. Pstack remains the lead workflow. Do not reopen these decisions.

`minimal-governance-v2.md` proposes 24 months, 3 countries, focus on 10 cells, and a fiscal pressure announced at start: transition aid ends at month 13. This is the chosen prototype example of the user's broader pressure decision. All monetary amounts, capacity thresholds and target values remain tunable assumptions. Enterprise forecasts use demand, cash, skills, land and infrastructure. Subsidies are bounded and escrowed; public works are separate. Construction, output, arrivals and graduates activate the following month. The proposal covers limited institutions, education, two peaceful contracts, security, monthly reports and full-snapshot saves.

No 24-month economic replay, enterprise decision test or gameplay save/load test has been run. Those systems do not yet exist. The two viable development routes in the proposal are acceptance requirements, not established balance results.

## Map feasibility and architecture evidence

Actual release API sample: `经纶·综合治理·01`, 100 cells, relaxations 2, land ratio 0.45. Generator compute time was 201.3 ms in one run, not a benchmark distribution. The response was 6,500,121 bytes; the compact complete map snapshot is 6,500,066 bytes. There are 42 majority-land cells and 1,893 source IDs.

`scenario-map-feasibility.json` records a preliminary connected set of ten cells with positive year-round freshwater planning sources, non-null construction costs and at least 20 km² buildable area. Candidate IDs: 3, 20, 43, 33, 41, 55, 48, 58, 85, 71. Their natural adjacency avoids sea/lake crossings. This does not establish built roads, water rights, actual supply, or final scenario ownership. Natural cost multipliers span roughly 1.31–2.18 and must enter economic calibration.

Independent sketches are `governance-architecture-a.md` and `governance-architecture-b.md`. A places accepted saves in the browser and uses a pure Rust transition; B adds disposable server sessions and recovery receipts. Cross-judge preferred A's smaller ownership boundary, but both complete-envelope monthly protocols need revision/measurement before HTTP implementation: maps remain large even at 100 cells, and B still returns full checkpoints each month. Do not infer that a 16 MiB request cap or a long replay horizon has been validated.

Next verifiable unit: an independent Rust gameplay domain, owned immutable map binding, versioned scenario/rules/state, checked money/material/land/shared-water reservations, and one enterprise-selected investment/subsidy transaction. Prove deterministic results, failed decisions with no deductions, escrow conservation, competing reservations, next-month activation and export/restore/continue equality before wiring the full UI. Exact opening populations, firms, buyer/supplier stocks and budgets must be made explicit and then calibrated. These are scenario implementation inputs, not reasons to ask the user again about the accepted direction.

Keep the generator Serialize-only structure in mind; it contains static strings, so do not assume WorldMap can simply gain Deserialize. Preserve the full owned map for save/display, derive a gameplay projection, and do not regenerate old saves with a newer generator. Keep map preview month separate from simulation month. Retain the existing 4096-byte map-generation route cap; gameplay limits must be route-specific. Avoid global sessions or new persistence infrastructure until actual monthly payload measurements justify them.

## Environment

All project work was Cloud-only. Imported Pstack is `/workspace/.task-skills/pstack-import/pstack`; read the adjacent `CLOUD-HARNESS.md`. The user's explicit GPT-6.1 Sol selection overrides the imported inherit-parent model preference.

Cached Node dependencies remain `/workspace/jinglun/node_modules`. Rust commands used `PATH=/workspace/.rustup/toolchains/1.96.1-x86_64-unknown-linux-gnu/bin:$PATH`, `CARGO_HOME=/workspace/jinglun/.cache/cargo`, `RUSTUP_HOME=/workspace/.rustup`, `CARGO_TARGET_DIR=/workspace/jinglun/target`, and offline/locked flags. No installation is needed to resume validation.

Current managed network policy remains restricted; the public deployment IP is not allowed. No public-site verification occurred in this checkpoint. Do not bypass the proxy or switch to the user's local machine for project work.
