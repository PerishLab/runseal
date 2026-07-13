use super::super::help::{Entry, Section};

pub const ROOT: Entry = Entry {
    key: "github",
    usage: "runseal @tool github <scope> <command> [args]",
    about: Some("GitHub helpers for collaboration rules that need more than plain gh ergonomics."),
    sections: &[Section {
        title: "GitHub helpers",
        items: &[
            (
                "issue create",
                "create one issue with optional body shaping",
            ),
            (
                "issue comment create",
                "create one issue-style comment; also applies to top-level PR comments",
            ),
            (
                "issue body update",
                "update one issue-style body; also applies to top-level PR bodies",
            ),
            (
                "pr checks probe",
                "probe whether a PR head has checks before watching",
            ),
        ],
    }],
    examples: &[],
};

pub mod issue {
    use crate::core::tool::help::{Entry, Section};

    pub const ROOT: Entry = Entry {
        key: "github.issue",
        usage: "runseal @tool github issue <scope> <command> [args]",
        about: Some(
            "GitHub issue-style write helpers. GitHub PR top-level bodies and timeline comments also use this issue model.",
        ),
        sections: &[Section {
            title: "GitHub issue helpers",
            items: &[
                ("create", "create one issue"),
                ("comment create", "create one issue-style comment"),
                ("body update", "update one issue-style body"),
            ],
        }],
        examples: &[],
    };

    pub const CREATE: Entry = Entry {
        key: "github.issue.create",
        usage: "runseal @tool github issue create --repo <owner/name> --title <text> [--body <text>|--body-file <path>] [--body-max <n>] [--prefix-enable=<true|false>] [--token <text>|--token-file <path>|--token-env <name>]",
        about: Some(
            "Create one GitHub issue and print the API response JSON. Body is optional. Default `--body-max` is `0`, which means unlimited.",
        ),
        sections: &[Section {
            title: "Flags",
            items: &[
                ("--repo <owner/name>", "target GitHub repository"),
                ("--title <text>", "issue title"),
                ("--body <text>", "inline body text"),
                ("--body-file <path>", "read body text from one file"),
                (
                    "--body-max <n>",
                    "maximum user-body length; `0` disables the limit; default `0`",
                ),
                (
                    "--prefix-enable=<true|false>",
                    "prepend requested-by metadata for matching cross-repo writes",
                ),
                ("--token <text>", "explicit GitHub token"),
                (
                    "--token-file <path>",
                    "env-style file containing `GITHUB_TOKEN`",
                ),
                (
                    "--token-env <name>",
                    "read the token from one named environment variable; default fallback is `GITHUB_TOKEN`",
                ),
            ],
        }],
        examples: &[
            "runseal @tool github issue create --repo PerishCode/runseal --title demo --body-file body.md --prefix-enable=true",
        ],
    };

    pub mod comment {
        use crate::core::tool::help::{Entry, Section};

        pub const ROOT: Entry = Entry {
            key: "github.issue.comment",
            usage: "runseal @tool github issue comment create [args]",
            about: None,
            sections: &[Section {
                title: "GitHub issue comment helpers",
                items: &[("create", "create one issue-style comment")],
            }],
            examples: &[],
        };

        pub const CREATE: Entry = Entry {
            key: "github.issue.comment.create",
            usage: "runseal @tool github issue comment create --repo <owner/name> --number <n> (--body <text>|--body-file <path>) [--body-max <n>] [--prefix-enable=<true|false>] [--token <text>|--token-file <path>|--token-env <name>]",
            about: Some(
                "Create one GitHub issue-style comment and print the API response JSON. Default `--body-max` is `100`; over-limit bodies fail fast.",
            ),
            sections: &[Section {
                title: "Flags",
                items: &[
                    ("--repo <owner/name>", "target GitHub repository"),
                    ("--number <n>", "issue or pull request number"),
                    ("--body <text>", "inline comment body"),
                    ("--body-file <path>", "read comment body from one file"),
                    (
                        "--body-max <n>",
                        "maximum user-body length; `0` disables the limit; default `100`",
                    ),
                    (
                        "--prefix-enable=<true|false>",
                        "prepend requested-by metadata for matching cross-repo writes",
                    ),
                    ("--token <text>", "explicit GitHub token"),
                    (
                        "--token-file <path>",
                        "env-style file containing `GITHUB_TOKEN`",
                    ),
                    (
                        "--token-env <name>",
                        "read the token from one named environment variable; default fallback is `GITHUB_TOKEN`",
                    ),
                ],
            }],
            examples: &[
                "runseal @tool github issue comment create --repo PerishCode/runseal --number 46 --body-file body.md --prefix-enable=true",
            ],
        };
    }

    pub mod body {
        use crate::core::tool::help::{Entry, Section};

        pub const ROOT: Entry = Entry {
            key: "github.issue.body",
            usage: "runseal @tool github issue body update [args]",
            about: None,
            sections: &[Section {
                title: "GitHub issue body helpers",
                items: &[("update", "update one issue-style body")],
            }],
            examples: &[],
        };

        pub const UPDATE: Entry = Entry {
            key: "github.issue.body.update",
            usage: "runseal @tool github issue body update --repo <owner/name> --number <n> (--body <text>|--body-file <path>) [--body-max <n>] [--prefix-enable=<true|false>] [--token <text>|--token-file <path>|--token-env <name>]",
            about: Some(
                "Update one GitHub issue-style body and print the API response JSON. Default `--body-max` is `0`, which means unlimited.",
            ),
            sections: &[Section {
                title: "Flags",
                items: &[
                    ("--repo <owner/name>", "target GitHub repository"),
                    ("--number <n>", "issue or pull request number"),
                    ("--body <text>", "inline body text"),
                    ("--body-file <path>", "read body text from one file"),
                    (
                        "--body-max <n>",
                        "maximum user-body length; `0` disables the limit; default `0`",
                    ),
                    (
                        "--prefix-enable=<true|false>",
                        "prepend requested-by metadata for matching cross-repo writes",
                    ),
                    ("--token <text>", "explicit GitHub token"),
                    (
                        "--token-file <path>",
                        "env-style file containing `GITHUB_TOKEN`",
                    ),
                    (
                        "--token-env <name>",
                        "read the token from one named environment variable; default fallback is `GITHUB_TOKEN`",
                    ),
                ],
            }],
            examples: &[
                "runseal @tool github issue body update --repo PerishCode/runseal --number 46 --body-file body.md --prefix-enable=true",
            ],
        };
    }
}

pub mod pr {
    use crate::core::tool::help::{Entry, Section};

    pub const ROOT: Entry = Entry {
        key: "github.pr",
        usage: "runseal @tool github pr <scope> <command> [args]",
        about: Some(
            "GitHub pull request helper atoms. These are intentionally narrow and are usually called from wrappers.",
        ),
        sections: &[Section {
            title: "GitHub PR helpers",
            items: &[(
                "checks probe <number>",
                "print whether a PR head already has check runs or commit statuses",
            )],
        }],
        examples: &[],
    };

    pub mod checks {
        use crate::core::tool::help::{Entry, Section};

        pub const ROOT: Entry = Entry {
            key: "github.pr.checks",
            usage: "runseal @tool github pr checks <command> [args]",
            about: None,
            sections: &[Section {
                title: "GitHub PR check helpers",
                items: &[(
                    "probe <number>",
                    "print whether a PR head already has check runs or commit statuses",
                )],
            }],
            examples: &[],
        };

        pub const PROBE: Entry = Entry {
            key: "github.pr.checks.probe",
            usage: "runseal @tool github pr checks probe <number> [--token <text>|--token-file <path>|--token-env <name>]",
            about: Some(
                "Probe the current repository PR head commit for check runs or commit statuses. Prints `true` or `false`; on API probe failure, prints `true` so wrapper flows do not incorrectly skip check watching.",
            ),
            sections: &[Section {
                title: "Flags",
                items: &[
                    ("--token <text>", "explicit GitHub token"),
                    (
                        "--token-file <path>",
                        "env-style file containing `GITHUB_TOKEN`",
                    ),
                    (
                        "--token-env <name>",
                        "read the token from one named environment variable; default fallback is `GITHUB_TOKEN`",
                    ),
                ],
            }],
            examples: &["runseal @tool github pr checks probe 46 --token-env RUNSEAL_GITHUB_TOKEN"],
        };
    }
}
