# Unknowns / blindspot pass

Known: Rust/Tauri is the product; full_access is file-trash authority; OS26.4 is installed; no actual host mutations permitted during tests.
Known unknowns: future OS support of every Finder key, applying Finder's visual state without restart, profiles changing while apply runs.
Implicit expectation: readable one-place controls, genuine values, one explicit local Apply instead of nested approval dialogs.
Hidden risks: missing!=false, managed policy, crash between write/readback, external changes overwritten, logical-vs-physical bytes, stale async UI state.

Architecture questions, inferred conservatively: all41 at once? no, five first. AI can mutate? no local execution only. Restart Finder? no. Permanent snapshot/model removal? no. New walker? no reuse bounded engine. Global memory write? no project-specific dictionary only.
