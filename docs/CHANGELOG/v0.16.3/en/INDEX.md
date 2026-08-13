# Runseal v0.16.3

Runseal's structured native-tool library can now be embedded by Plumb without
forming a package dependency cycle.

## An acyclic library seat

The default Runseal binary still includes managed skill operations. That
Plumb-backed integration is now the optional `managed-skill` feature. A Rust
consumer that selects `default-features = false` receives the profile core,
HTTP transport, structured `Reply`, and `runseal::tool::call` without a Plumb
dependency.

Runseal now owns its core profile parsing, home discovery, and build-version
probe directly. Profile grammar, home paths, precedence, managed skill
behavior, and the complete `@forgejo` command surface are unchanged.
