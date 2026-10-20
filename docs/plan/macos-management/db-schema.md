# Settings journal schema

Existing rusqlite dependency; separate app-data mac-settings-v1.sqlite3, not an alteration of chat/file stores. No auth/user tables (single local role).

```sql
CREATE TABLE IF NOT EXISTS mac_setting_changes (
  id TEXT PRIMARY KEY NOT NULL,
  setting_id TEXT NOT NULL,
  previous_value INTEGER CHECK(previous_value IN (0,1)),
  applied_value INTEGER NOT NULL CHECK(applied_value IN (0,1)),
  status TEXT NOT NULL,
  created_at_unix_ms INTEGER NOT NULL
);
```

NULL previous_value means no explicit stored override. `rowid DESC`, not wall-clock time, orders the rollback chain; timestamp is display only. One row is one bounded change. Status allowlist is checked on load. Every write connection uses DELETE journal + synchronous FULL; prepared INSERT/restorePrepared UPDATE is committed before native effect, final state only after readback. Read-only snapshot does not create a DB.128 records is a hard new-change ceiling preserving every original chain; restore remains available. No silent pruning. No cross-process OS CAS guarantee.

No relational ERD needed for a single journal table. Primary key supports exact opaque change ID; rowid provides insertion order. No extra index at128 rows. Initial schema only; no preexisting table migration.
