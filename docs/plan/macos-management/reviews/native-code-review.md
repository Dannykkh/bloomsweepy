# Native Codex code review

2026-10-20. Source: native `codex review`, read-only, configured single model family. Session50395 exit0. Custom prompt and `--uncommitted` were mutually exclusive, so the supported custom-prompt mode was used with explicit source scope. Private screenshots, conversations, memory, real OS writes and user file operations were excluded.

The review found one P2: ordering restoration history by wall-clock timestamps could select the wrong head if the clock moved backward, stranding a valid rollback chain. Changed journal history ordering to `rowid DESC`; timestamps remain display metadata. The multistep regression now moves the newer row's timestamp to0, rejects an older-first restore, restores the newer change, then the older change back to original absence.

The independent source verifier rechecked the changed path. Final full workspace suite session76735 exited0 with478 passed/6 ignored, including that regression. This closes the reported finding; it does not claim comprehensive security or installed-app E2E approval.

#tags: native-review, rollback-chain, clock-rollback, regression, arch:016
