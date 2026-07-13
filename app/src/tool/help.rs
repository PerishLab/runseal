use super::{cloudflare::help as cloudflare, github::help as github};

#[derive(Clone, Copy)]
pub(super) struct Entry {
    pub(super) key: &'static str,
    pub(super) usage: &'static str,
    pub(super) about: Option<&'static str>,
    pub(super) sections: &'static [Section],
    pub(super) examples: &'static [&'static str],
}

#[derive(Clone, Copy)]
pub(super) struct Section {
    pub(super) title: &'static str,
    pub(super) items: &'static [(&'static str, &'static str)],
}

const ENTRIES: &[Entry] = &[
    forgejo::ROOT,
    forgejo::repo::ROOT,
    forgejo::repo::GET,
    forgejo::repo::CREATE,
    forgejo::pr::ROOT,
    forgejo::pr::FIND,
    forgejo::pr::CREATE,
    forgejo::pr::GET,
    forgejo::pr::MERGE,
    forgejo::pr::GUARD,
    forgejo::secret::ROOT,
    forgejo::secret::UPSERT,
    forgejo::variable::ROOT,
    forgejo::variable::UPSERT,
    forgejo::workflow::ROOT,
    forgejo::workflow::DISPATCH,
    forgejo::run::ROOT,
    forgejo::run::LIST,
    forgejo::run::GET,
    forgejo::run::WATCH,
    forgejo::run::CANCEL,
    github::ROOT,
    github::issue::ROOT,
    github::issue::CREATE,
    github::issue::comment::ROOT,
    github::issue::comment::CREATE,
    github::issue::body::ROOT,
    github::issue::body::UPDATE,
    github::pr::ROOT,
    github::pr::checks::ROOT,
    github::pr::checks::PROBE,
    cloudflare::ROOT,
    cloudflare::config::ROOT,
    cloudflare::config::GET,
    cloudflare::config::JSON,
    cloudflare::api::ROOT,
    cloudflare::api::REQUEST,
    cloudflare::zone::ROOT,
    cloudflare::zone::GET,
    cloudflare::zone::ruleset::ROOT,
    cloudflare::zone::ruleset::LIST,
    cloudflare::zone::ruleset::GET,
    cloudflare::zone::ruleset::CREATE,
    cloudflare::zone::ruleset::rule::ROOT,
    cloudflare::zone::ruleset::rule::ADD,
    cloudflare::zone::ruleset::rule::UPDATE,
    cloudflare::zone::record::ROOT,
    cloudflare::zone::record::LIST,
    cloudflare::zone::record::CREATE,
    cloudflare::zone::record::UPDATE,
    cloudflare::account::ROOT,
    cloudflare::account::GET,
    cloudflare::account::r2::ROOT,
    cloudflare::account::r2::bucket::ROOT,
    cloudflare::account::r2::bucket::LIST,
    cloudflare::redirect::ROOT,
    cloudflare::redirect::EXACT,
];

pub fn top() -> &'static str {
    "\
Usage: runseal @tool <namespace> <command> [args]

Run an atomic runseal tool command.

Tools:
  forgejo ...                            forgejo repository and actions helpers
  github ...                             github helpers
  cloudflare ...                         cloudflare helpers
"
}

mod forgejo {
    use super::{Entry, Section};

    pub const ROOT: Entry = Entry {
        key: "forgejo",
        usage: "runseal @tool forgejo <scope> <command> [args]",
        about: Some("Forgejo API helpers backed by Tea login or CI environment credentials."),
        sections: &[Section {
            title: "Forgejo helpers",
            items: &[
                ("repo get|create", "query or create a repository"),
                (
                    "pr find|create|get|merge|guard",
                    "manage pull requests and wait for guard",
                ),
                (
                    "secret upsert",
                    "create or update a repository Actions secret",
                ),
                (
                    "variable upsert",
                    "create or update a repository Actions variable",
                ),
                (
                    "workflow dispatch",
                    "dispatch one workflow and return its run",
                ),
                (
                    "run list|get|watch|cancel",
                    "inspect or wait for Actions runs",
                ),
            ],
        }],
        examples: &["runseal @tool forgejo repo get --repo PerishFire/runseal"],
    };

    pub mod repo {
        use crate::core::tool::help::Entry;

        pub const ROOT: Entry = Entry {
            key: "forgejo.repo",
            usage: "runseal @tool forgejo repo get|create [args]",
            about: None,
            sections: &[],
            examples: &[],
        };

        pub const GET: Entry = Entry {
            key: "forgejo.repo.get",
            usage: "runseal @tool forgejo repo get --repo <owner/name> [--login <name>]",
            about: None,
            sections: &[],
            examples: &[],
        };

        pub const CREATE: Entry = Entry {
            key: "forgejo.repo.create",
            usage: "runseal @tool forgejo repo create --owner <owner|@me> --name <name> [--private <true|false>] [--description <text>]",
            about: None,
            sections: &[],
            examples: &[],
        };
    }

    pub mod pr {
        use crate::core::tool::help::Entry;

        pub const ROOT: Entry = Entry {
            key: "forgejo.pr",
            usage: "runseal @tool forgejo pr find|create|get|merge|guard [args]",
            about: None,
            sections: &[],
            examples: &[],
        };

        pub const FIND: Entry = Entry {
            key: "forgejo.pr.find",
            usage: "runseal @tool forgejo pr find --repo <owner/name> --head <branch> --base <branch>",
            about: None,
            sections: &[],
            examples: &[],
        };

        pub const CREATE: Entry = Entry {
            key: "forgejo.pr.create",
            usage: "runseal @tool forgejo pr create --repo <owner/name> --head <branch> --base <branch> --title <text> [--body <text>]",
            about: None,
            sections: &[],
            examples: &[],
        };

        pub const GET: Entry = Entry {
            key: "forgejo.pr.get",
            usage: "runseal @tool forgejo pr get --repo <owner/name> --number <n>",
            about: None,
            sections: &[],
            examples: &[],
        };

        pub const MERGE: Entry = Entry {
            key: "forgejo.pr.merge",
            usage: "runseal @tool forgejo pr merge --repo <owner/name> --number <n> --head <guarded-sha> [--delete-branch <true|false>]",
            about: Some("Squash-merge one pull request after the caller has verified its guard."),
            sections: &[],
            examples: &[],
        };

        pub const GUARD: Entry = Entry {
            key: "forgejo.pr.guard",
            usage: "runseal @tool forgejo pr guard --repo <owner/name> --number <n> [--workflow guard.yml] [--interval <seconds>] [--timeout <seconds>]",
            about: Some(
                "Wait for the pull request head's Forgejo Actions guard run and fail closed.",
            ),
            sections: &[],
            examples: &[],
        };
    }

    pub mod secret {
        use crate::core::tool::help::Entry;

        pub const ROOT: Entry = Entry {
            key: "forgejo.secret",
            usage: "runseal @tool forgejo secret upsert [args]",
            about: None,
            sections: &[],
            examples: &[],
        };

        pub const UPSERT: Entry = Entry {
            key: "forgejo.secret.upsert",
            usage: "runseal @tool forgejo secret upsert --repo <owner/name> --name <name> (--value <text>|--value-env <name>|--value-file <path>)",
            about: Some(
                "Prefer --value-env or --value-file so secret data is not exposed in process arguments.",
            ),
            sections: &[],
            examples: &[],
        };
    }

    pub mod variable {
        use crate::core::tool::help::Entry;

        pub const ROOT: Entry = Entry {
            key: "forgejo.variable",
            usage: "runseal @tool forgejo variable upsert [args]",
            about: None,
            sections: &[],
            examples: &[],
        };

        pub const UPSERT: Entry = Entry {
            key: "forgejo.variable.upsert",
            usage: "runseal @tool forgejo variable upsert --repo <owner/name> --name <name> (--value <text>|--value-env <name>|--value-file <path>)",
            about: None,
            sections: &[],
            examples: &[],
        };
    }

    pub mod workflow {
        use crate::core::tool::help::Entry;

        pub const ROOT: Entry = Entry {
            key: "forgejo.workflow",
            usage: "runseal @tool forgejo workflow dispatch [args]",
            about: None,
            sections: &[],
            examples: &[],
        };

        pub const DISPATCH: Entry = Entry {
            key: "forgejo.workflow.dispatch",
            usage: "runseal @tool forgejo workflow dispatch --repo <owner/name> --workflow <file> --ref <ref> [--input <key=value>]...",
            about: Some("Dispatch a workflow with return_run_info enabled and print the run JSON."),
            sections: &[],
            examples: &[],
        };
    }

    pub mod run {
        use crate::core::tool::help::Entry;

        pub const ROOT: Entry = Entry {
            key: "forgejo.run",
            usage: "runseal @tool forgejo run list|get|watch|cancel [args]",
            about: None,
            sections: &[],
            examples: &[],
        };

        pub const LIST: Entry = Entry {
            key: "forgejo.run.list",
            usage: "runseal @tool forgejo run list --repo <owner/name> [--workflow <file>] [--event <event>] [--status <status>] [--sha <sha>] [--ref <ref>] [--number <n>] [--limit <n>]",
            about: None,
            sections: &[],
            examples: &[],
        };

        pub const GET: Entry = Entry {
            key: "forgejo.run.get",
            usage: "runseal @tool forgejo run get --repo <owner/name> --id <id>",
            about: None,
            sections: &[],
            examples: &[],
        };

        pub const WATCH: Entry = Entry {
            key: "forgejo.run.watch",
            usage: "runseal @tool forgejo run watch --repo <owner/name> --id <id> [--interval <seconds>] [--timeout <seconds>]",
            about: None,
            sections: &[],
            examples: &[],
        };

        pub const CANCEL: Entry = Entry {
            key: "forgejo.run.cancel",
            usage: "runseal @tool forgejo run cancel",
            about: Some(
                "Unsupported on Forgejo v15 because no workflow cancel REST endpoint exists.",
            ),
            sections: &[],
            examples: &[],
        };
    }
}

pub fn progressive(args: &[String]) -> Option<String> {
    let last = args.last()?;
    if !matches!(last.as_str(), "-h" | "--help" | "help") {
        return None;
    }
    let path = &args[..args.len() - 1];
    if path.is_empty() {
        return Some(top().to_string());
    }
    let key = path.join(".");
    let entry = ENTRIES.iter().find(|entry| entry.key == key)?;
    Some(render(entry))
}

fn render(entry: &Entry) -> String {
    if entry.about.is_none() && entry.sections.is_empty() && entry.examples.is_empty() {
        return format!("usage: {}", entry.usage);
    }
    let mut out = format!("Usage: {}\n", entry.usage);
    if let Some(about) = entry.about {
        out.push('\n');
        out.push_str(about);
        out.push('\n');
    }
    for section in entry.sections {
        out.push('\n');
        out.push_str(section.title);
        out.push_str(":\n");
        for (name, desc) in section.items {
            out.push_str("  ");
            out.push_str(name);
            let padding = 44usize.saturating_sub(name.len()).max(2);
            out.push_str(&" ".repeat(padding));
            out.push_str(desc);
            out.push('\n');
        }
    }
    if !entry.examples.is_empty() {
        out.push('\n');
        out.push_str("Examples:\n");
        for example in entry.examples {
            out.push_str("  ");
            out.push_str(example);
            out.push('\n');
        }
    }
    out
}
