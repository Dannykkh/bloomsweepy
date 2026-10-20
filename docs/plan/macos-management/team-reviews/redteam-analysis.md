# Red-team perspective

Source: combined native plan-review job. Enforce fixed ID/Bool inputs, reject raw keys/domains/commands and unknown fields. Local main origin must guard mutations, not only missing AI tool entries. Preserve original absence, fail closed on journal/managed/type errors, no pending-write replay, compare current before restore. A CFPreferences comparison cannot prevent undetected concurrent external writes or ABA; claims must be narrow.

## Dictionary Updates

REFINE canRestore: checked current state, not an external lock. Accepted.
