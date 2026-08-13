# Restrained scenarios

## A named profile is missing

Explicit named selection refuses when `runseal.<name>.toml` does not exist.
Do not invent the file, fall back to the default profile, or continue with an
empty named profile.

## A symlink target is occupied

Refuse. Never replace, adopt, or silently repair an existing path. Remove the
occupant only when that is the caller's explicit intent, then retry.

## An external command can be isolated

Keep it external. `@tool` exists only for a third-party dependency that the
profile triad cannot isolate, and must remain one atomic operation. No native
tools ship. Do not add aliases, convenience wrappers, or command sequences.

## Profile mode has no command

A colon without a following command or `@tool` refuses. The colon is not a
default action.

## A skill candidate needs validation

Do not replace a managed stable seat. Stage the exact candidate under a new
isolated root ending in `runseal`, run the intended session against that path,
and remove the surrounding isolated root only after any needed evidence is
kept. Promotion later uses the same source revision as the validated
candidate.
