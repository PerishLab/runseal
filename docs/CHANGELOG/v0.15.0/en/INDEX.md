# Runseal v0.15.0

This is the first stable release of the 0.15 line. It ships the managed skill
seat that 0.14 left as product law without an installable brief.

## A first-class skill brief

Runseal now ships the closed three-file brief under `skills/runseal`. The
frontmatter names when to use it: inspect or apply a profile, edit a
`runseal.toml`, or change the Runseal repository.

The control plane adds `runseal skill` for managed install, upgrade, status,
stage, list, and uninstall. Managed seats accept stable. Exact non-stable
briefs use `skill stage` at a new path ending in `runseal`.

The repository declares the brief as a `[[document]]` strategy and
`[release].skill = true`. Packaging and placement are derived.

## Repository-local law stays in AGENTS.md

`AGENTS.md` keeps repository-local operating law. Operator grammar lives in
the brief. Neither retells the other's surface.
