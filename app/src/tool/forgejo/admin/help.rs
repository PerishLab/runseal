pub(super) const TEXT: &str = r#"@forgejo-admin — one Forgejo server authority operation

Usage: runseal :PROFILE @forgejo-admin [--json] RESOURCE VERB ...

Resource verbs:
  token create ACCOUNT NAME --scopes SCOPE[,SCOPE...]
  token delete ACCOUNT NAME ID

The selected profile must provide the private issuer and store projection.
Token values remain redacted from output and are available only to structured callers.
"#;
