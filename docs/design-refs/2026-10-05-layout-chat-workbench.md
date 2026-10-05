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
                 |       compact privacy note
```

No title/composer overlaps: rows share a bounded grid, not sticky overlays.
390px: history compact, target/provider reflow, transcript width preserved;
native modal takes available width and scrolls internally. All final review
controls stay expanded; read-only results start collapsed and mount on demand.
