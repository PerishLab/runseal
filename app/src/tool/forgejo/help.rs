pub(super) const TEXT: &str = r#"@forgejo — one Forgejo HTTP operation

Usage: runseal :PROFILE @forgejo [OPTIONS] RESOURCE VERB [TARGET]

Options:
  --json                 emit a versioned JSON envelope
  --url URL              override FORGEJO_URL
  --token-file PATH      override the profile token file
  --repo OWNER/REPO      override the cwd origin repository
  --limit N              cap a fully paged list

Resource verbs:
  user show
  issue show ID
  issue list
  issue create --title TEXT [--body TEXT]
  issue edit ID [--title TEXT] [--body TEXT] [--state open|closed]
  issue comment list ID
  issue comment create ID --body TEXT
  pull show ID
  pull list
  pull create --title TEXT --head BRANCH [--base BRANCH] [--body TEXT]
  pull edit ID [--title TEXT] [--body TEXT] [--state STATE]
  pull merge ID --head SHA [--do STRATEGY]
  review list ID
  review create ID --body TEXT [--event EVENT]
  status show REF
  branch show NAME
  branch create NAME --from REF
  protection show NAME
  protection create --body JSON
  protection edit NAME --body JSON
  repo show
  repo edit --body JSON
  repo delete
  secret list
  secret set NAME --body VALUE
  secret delete NAME
  workflow dispatch FILE --ref REF [--inputs JSON]
  run show ID
  job log RUN JOB [--attempt N] [--watch] [--poll-ms N] [--timeout-ms N]
  task list RUN_NUMBER
  label list
  get URL
"#;
