# Architecture perspective

Source: native return-only code explorer. Existing capabilities/app_tools is the shared native/MCP composition; cleanup scan/revalidation/trash journal are reusable. Preference undo has distinct data from Trash and needs a small separate module/journal, not a rewrite of action recovery. No new dependency or separate MCP implementation. Independent module file ownership supports bounded parallel work.

## Dictionary Updates

ADD macosInspect: read-only common app capability, not an OS writer. Accepted.
