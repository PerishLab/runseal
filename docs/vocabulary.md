# Vocabulary

Runseal's stable atoms describe its two planes and three profile capabilities.

- `control`: Runseal-owned commands parsed without a colon.
- `profile`: the selected env, argv, and symlink declaration.
- `tool`: one native atomic capability for a not-plumbable third-party
  dependency.
- `resource`: committed inert material under `.runseal/resources`.
- `local`: ignored secret or machine-local material under `.local`.
- `lease`: a symlink target owned for one invocation and refused when occupied.

No compound atoms are currently registered. New vocabulary must represent a
durable product distinction rather than an implementation helper.
