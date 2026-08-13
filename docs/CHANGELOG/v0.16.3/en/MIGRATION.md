# Migrating to Runseal v0.16.3

Binary users and existing profiles require no changes. Managed skill commands
remain enabled by default.

In-process consumers that may sit below Runseal's managed-skill dependency use
the library-only seat:

```toml
runseal = { version = "0", registry = "perish", default-features = false }
```

Continue to call `runseal::tool::call("forgejo", argv, vars)` and consume its
structured `Reply`. Enable the default features only when the consuming binary
also needs Runseal's managed skill commands.
