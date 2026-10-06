# Layout: Conversation-first workbench

User-directed familiar chat layout, not a new brand direction. Render-candidate
exploration omitted because the user explicitly selected this interaction pattern.

```text
existing sidebar | 대화            [history] [new] [delete]
                 | [target / size] [connections]          [provider]
                 | ------------------------------------------------
                 |       assistant reply (800px reading measure)
                 |                         user message bubble
                 |       [app evidence / status ▸]
                 |       (transcript alone scrolls)
                 | ------------------------------------------------
                 |       actual phase · elapsed time       [stop]
                 |       [message textarea          ] [send]
                 |       AI 모델 [selected model ▾] 추론 강도 [default / level ▾]
                 |       compact privacy note
```

No title/composer overlaps: rows share a bounded grid, not sticky overlays.
390px: history compact, target/provider reflow, transcript width preserved;
native modal takes available width and scrolls internally. All final review
controls stay expanded; read-only results start collapsed and mount on demand.

Trash-confirmation delta: within the expanded evidence, show target → exact effect
and recovery → `[아니오] [예, …휴지통으로 이동]`. Both controls are in the transcript,
not a second modal. On narrow windows the scope-specific affirmative label wraps
and actions reflow. Existing glass tokens, dock and typography stay unchanged.

Model-selection delta: labelled44px select below the message field, followed by
catalog provenance. Settings uses the same Picker and local preference map;
provider switching restores that provider's choice. No extra overlay, animation,
blur. Unsupported integrations show a disabled CLI default.

Reasoning-selection delta: a second labelled44px select shares the Picker and
provider/model-specific local preference. Only CLI-reported Codex levels appear;
the default shows a verified catalog default where available. Both controls reflow
on narrow widths, with compact provenance and an explicit stale-choice warning/reset.
No extra overlay or motion; the composer remains reachable and busy choices disabled.
