# QA scenarios

Target: current working tree, synthetic/isolated validation. Host Finder mutation and user-data cleanup intentionally excluded.

| TC-ID | Scenario | Evidence | Priority |
|---|---|---|---|
| MAC-01 | five fixed Bool IDs, unset!=false, wrong type/managed blocked | backend tests | P0 |
| MAC-02 | apply Bool, persisted original, restore original absence | fake store + isolated native domain | P0 |
| MAC-03 | journal unavailable/write failure/readback mismatch | backend failure tests | P0 |
| MAC-04 | pending restart no replay; external value conflict | backend tests | P0 |
| MAC-05 | raw key/domain/command/unknown field rejected, AI no writes | protocol + request tests | P0 |
| MAC-06 | developer paths/ages/Review; recent descendants/link/bounds excluded | core/helper tests | P0 |
| MAC-07 | stage does not apply; Apply once; Restore checked; busy UI | actual component Playwright mock | P0 |
| MAC-08 | loading/failure/managed/unset/unsupported displayed accurately | actual component Playwright mock | P1 |
| MAC-09 | 1280 and390px no horizontal overflow, fonts/focus/reduced motion | browser exploration assertions/screenshots | P1 |
| MAC-10 | common capability ID/view route; external disclosure;16KiB bounds | common protocol/regression tests | P0 |
| MAC-11 | full Rust/frontend/build/locale regression | executed integration suite | P0 |

Results are recorded after execution in verify-report.md and minos-report.md. Source presence alone is not PASS.

## Argos reinspection — 2026-10-20

Same-source runtime evidence is reused, not rerun merely to refresh counts. The latest verdict and limits are in verify-report.md.

- [x] MAC-01/02/03/04/05/06: implemented boundary/failure/cleanup cases traced to the passed Rust suite; native isolated-domain and actual read-only smoke distinguished from real Finder mutation.
- [x] MAC-07/08/09: actual component mock UI10/10 with1280/390 layout/font/focus/reduced-motion evidence. Restore checked means backend expected/current validation, not an extra confirmation checkbox.
- [x] MAC-10/11: common protocol/result bounds and Rust/frontend/build/locale evidence reused after source fingerprint check.
- [x] Additional native IPC negative: real isolated Wry10/10; eight rejection responses each followed by an unchanged five-setting/journal observation. See ../../qa/macos-management-argos/README.md.
- [ ] Installed-product normal Apply/Restore/business-flow success: NOT RUN; passing canary transport is not a successful mutation control.
- [ ] Full security approval: development dependency High1 and stated scanner/runtime coverage gaps remain. Overall Argos verdict is CONDITIONAL.
