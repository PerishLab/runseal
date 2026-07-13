use super::super::help::{Entry, Section};

pub const ROOT: Entry = Entry {
    key: "cloudflare",
    usage: "runseal @tool cloudflare <scope> <command> [args]",
    about: None,
    sections: &[Section {
        title: "Cloudflare helpers",
        items: &[
            (
                "config get|json",
                "inspect configured account/zone defaults",
            ),
            ("api request", "send one authenticated API request"),
            ("zone get|ruleset|dns-record", "manage zone resources"),
            (
                "account get|r2 bucket list",
                "manage account-level resources",
            ),
            ("redirect-rule exact", "build one redirect rule payload"),
        ],
    }],
    examples: &[],
};

pub mod config {
    use crate::core::tool::help::{Entry, Section};

    pub const ROOT: Entry = Entry {
        key: "cloudflare.config",
        usage: "runseal @tool cloudflare config <command> [args]",
        about: None,
        sections: &[Section {
            title: "Cloudflare config helpers",
            items: &[
                ("get <key>", "print one configured value"),
                ("json", "print configured values as JSON"),
            ],
        }],
        examples: &[],
    };

    pub const GET: Entry = Entry {
        key: "cloudflare.config.get",
        usage: "runseal @tool cloudflare config get <key>",
        about: Some("Print one configured Cloudflare value loaded from `cloudflare.env`."),
        sections: &[Section {
            title: "Keys",
            items: &[
                ("account_id", "Cloudflare account identifier"),
                ("zone_name", "default zone name"),
                ("manage_host", "host used for release manager redirects"),
                (
                    "manage_origin_host",
                    "origin host used for release manager redirects",
                ),
                ("manage_redirect_prefix", "optional redirect prefix"),
            ],
        }],
        examples: &["runseal @tool cloudflare config get zone_name"],
    };

    pub const JSON: Entry = Entry {
        key: "cloudflare.config.json",
        usage: "runseal @tool cloudflare config json",
        about: Some("Print configured Cloudflare values as a JSON object."),
        sections: &[],
        examples: &["runseal @tool cloudflare config json"],
    };
}

pub mod api {
    use crate::core::tool::help::{Entry, Section};

    pub const ROOT: Entry = Entry {
        key: "cloudflare.api",
        usage: "runseal @tool cloudflare api request <method> <path> [--query <k=v>]... [--json <json>]",
        about: Some("Send one authenticated Cloudflare API request."),
        sections: &[Section {
            title: "Flags",
            items: &[
                ("--query <k=v>", "append one query pair; repeatable"),
                ("--json <json>", "send a JSON request body"),
            ],
        }],
        examples: &["runseal @tool cloudflare api request GET /zones --query name=perish.uk"],
    };

    pub const REQUEST: Entry = Entry {
        key: "cloudflare.api.request",
        usage: "runseal @tool cloudflare api request <method> <path> [--query <k=v>]... [--json <json>]",
        about: Some("Send one authenticated Cloudflare API request."),
        sections: &[Section {
            title: "Flags",
            items: &[
                ("--query <k=v>", "append one query pair; repeatable"),
                ("--json <json>", "send a JSON request body"),
            ],
        }],
        examples: &[
            "runseal @tool cloudflare api request PATCH /zones/ZONE_ID/dns_records/REC_ID --json '{\"ttl\":120}'",
        ],
    };
}

pub mod zone {
    use crate::core::tool::help::{Entry, Section};

    pub const ROOT: Entry = Entry {
        key: "cloudflare.zone",
        usage: "runseal @tool cloudflare zone <command> [args]",
        about: None,
        sections: &[Section {
            title: "Cloudflare zone helpers",
            items: &[
                (
                    "get --name <zone>",
                    "fetch one zone object by exact zone name",
                ),
                ("ruleset <command> [args]", "manage zone rulesets"),
                ("dns-record <command> [args]", "manage zone DNS records"),
            ],
        }],
        examples: &[
            "runseal @tool cloudflare zone get --name perish.uk",
            "runseal @tool cloudflare zone dns-record list --zone-id ZONE_ID",
        ],
    };

    pub const GET: Entry = Entry {
        key: "cloudflare.zone.get",
        usage: "runseal @tool cloudflare zone get --name <zone>",
        about: Some("Fetch one zone object by exact zone name."),
        sections: &[Section {
            title: "Flags",
            items: &[("--name <zone>", "target zone name")],
        }],
        examples: &["runseal @tool cloudflare zone get --name perish.uk"],
    };

    pub mod ruleset {
        use crate::core::tool::help::{Entry, Section};

        pub const ROOT: Entry = Entry {
            key: "cloudflare.zone.ruleset",
            usage: "runseal @tool cloudflare zone ruleset <command> [args]",
            about: None,
            sections: &[Section {
                title: "Cloudflare zone ruleset helpers",
                items: &[
                    ("list --zone-id <id>", "list rulesets in one zone"),
                    ("get --zone-id <id> --ruleset-id <id>", "fetch one ruleset"),
                    (
                        "create --zone-id <id> --phase <phase> --name <name>",
                        "create an empty zone ruleset",
                    ),
                    ("rule <command> [args]", "add or update one ruleset rule"),
                ],
            }],
            examples: &[],
        };

        pub const LIST: Entry = Entry {
            key: "cloudflare.zone.ruleset.list",
            usage: "runseal @tool cloudflare zone ruleset list --zone-id <id>",
            about: Some("Print zone rulesets as a JSON array."),
            sections: &[Section {
                title: "Flags",
                items: &[("--zone-id <id>", "target zone identifier")],
            }],
            examples: &["runseal @tool cloudflare zone ruleset list --zone-id ZONE_ID"],
        };

        pub const GET: Entry = Entry {
            key: "cloudflare.zone.ruleset.get",
            usage: "runseal @tool cloudflare zone ruleset get --zone-id <id> --ruleset-id <id>",
            about: Some("Fetch one zone ruleset as JSON."),
            sections: &[Section {
                title: "Flags",
                items: &[
                    ("--zone-id <id>", "target zone identifier"),
                    ("--ruleset-id <id>", "target ruleset identifier"),
                ],
            }],
            examples: &[
                "runseal @tool cloudflare zone ruleset get --zone-id ZONE_ID --ruleset-id RULESET_ID",
            ],
        };

        pub const CREATE: Entry = Entry {
            key: "cloudflare.zone.ruleset.create",
            usage: "runseal @tool cloudflare zone ruleset create --zone-id <id> --phase <phase> --name <name>",
            about: Some("Create an empty zone ruleset and print the created object as JSON."),
            sections: &[Section {
                title: "Flags",
                items: &[
                    ("--zone-id <id>", "target zone identifier"),
                    ("--phase <phase>", "Cloudflare ruleset phase"),
                    ("--name <name>", "ruleset name"),
                ],
            }],
            examples: &[
                "runseal @tool cloudflare zone ruleset create --zone-id ZONE_ID --phase http_request_dynamic_redirect --name manage-redirects",
            ],
        };

        pub mod rule {
            use crate::core::tool::help::{Entry, Section};

            pub const ROOT: Entry = Entry {
                key: "cloudflare.zone.ruleset.rule",
                usage: "runseal @tool cloudflare zone ruleset rule <command> [args]",
                about: None,
                sections: &[Section {
                    title: "Cloudflare ruleset rule helpers",
                    items: &[
                        (
                            "add --zone-id <id> --ruleset-id <id> --json <json>",
                            "add one rule",
                        ),
                        (
                            "update --zone-id <id> --ruleset-id <id> --rule-id <id> --json <json>",
                            "update one existing rule",
                        ),
                    ],
                }],
                examples: &[],
            };

            pub const ADD: Entry = Entry {
                key: "cloudflare.zone.ruleset.rule.add",
                usage: "runseal @tool cloudflare zone ruleset rule add --zone-id <id> --ruleset-id <id> --json <json>",
                about: Some("Add one rule to an existing ruleset."),
                sections: &[Section {
                    title: "Flags",
                    items: &[
                        ("--zone-id <id>", "target zone identifier"),
                        ("--ruleset-id <id>", "target ruleset identifier"),
                        ("--json <json>", "Cloudflare ruleset rule payload"),
                    ],
                }],
                examples: &[
                    "runseal @tool cloudflare zone ruleset rule add --zone-id ZONE_ID --ruleset-id RULESET_ID --json '{\"action\":\"redirect\"}'",
                ],
            };

            pub const UPDATE: Entry = Entry {
                key: "cloudflare.zone.ruleset.rule.update",
                usage: "runseal @tool cloudflare zone ruleset rule update --zone-id <id> --ruleset-id <id> --rule-id <id> --json <json>",
                about: Some("Update one existing ruleset rule."),
                sections: &[Section {
                    title: "Flags",
                    items: &[
                        ("--zone-id <id>", "target zone identifier"),
                        ("--ruleset-id <id>", "target ruleset identifier"),
                        ("--rule-id <id>", "target rule identifier"),
                        ("--json <json>", "Cloudflare ruleset rule payload"),
                    ],
                }],
                examples: &[
                    "runseal @tool cloudflare zone ruleset rule update --zone-id ZONE_ID --ruleset-id RULESET_ID --rule-id RULE_ID --json '{\"description\":\"updated\"}'",
                ],
            };
        }
    }

    pub mod record {
        use crate::core::tool::help::{Entry, Section};

        pub const ROOT: Entry = Entry {
            key: "cloudflare.zone.dns-record",
            usage: "runseal @tool cloudflare zone dns-record <command> [args]",
            about: None,
            sections: &[Section {
                title: "Cloudflare DNS record helpers",
                items: &[
                    (
                        "list --zone-id <id> [--name <name>]",
                        "list records in one zone",
                    ),
                    ("create --zone-id <id> --json <json>", "create one record"),
                    (
                        "update --zone-id <id> --record-id <id> --json <json>",
                        "update one record",
                    ),
                ],
            }],
            examples: &[],
        };

        pub const LIST: Entry = Entry {
            key: "cloudflare.zone.dns-record.list",
            usage: "runseal @tool cloudflare zone dns-record list --zone-id <id> [--name <name>]",
            about: Some("Print DNS records in one zone as a JSON array."),
            sections: &[Section {
                title: "Flags",
                items: &[
                    ("--zone-id <id>", "target zone identifier"),
                    ("--name <name>", "optional exact DNS record name filter"),
                ],
            }],
            examples: &[
                "runseal @tool cloudflare zone dns-record list --zone-id ZONE_ID",
                "runseal @tool cloudflare zone dns-record list --zone-id ZONE_ID --name runseal.perish.uk",
            ],
        };

        pub const CREATE: Entry = Entry {
            key: "cloudflare.zone.dns-record.create",
            usage: "runseal @tool cloudflare zone dns-record create --zone-id <id> --json <json>",
            about: Some(
                "Create one DNS record from the JSON payload and print the created record.",
            ),
            sections: &[Section {
                title: "Flags",
                items: &[
                    ("--zone-id <id>", "target zone identifier"),
                    ("--json <json>", "Cloudflare DNS record payload"),
                ],
            }],
            examples: &[
                "runseal @tool cloudflare zone dns-record create --zone-id ZONE_ID --json '{\"type\":\"CNAME\",\"name\":\"runseal\",\"content\":\"example.com\",\"proxied\":true}'",
            ],
        };

        pub const UPDATE: Entry = Entry {
            key: "cloudflare.zone.dns-record.update",
            usage: "runseal @tool cloudflare zone dns-record update --zone-id <id> --record-id <id> --json <json>",
            about: Some(
                "Update one DNS record from the JSON payload and print the updated record.",
            ),
            sections: &[Section {
                title: "Flags",
                items: &[
                    ("--zone-id <id>", "target zone identifier"),
                    ("--record-id <id>", "target DNS record identifier"),
                    (
                        "--json <json>",
                        "partial or full Cloudflare DNS record payload",
                    ),
                ],
            }],
            examples: &[
                "runseal @tool cloudflare zone dns-record update --zone-id ZONE_ID --record-id RECORD_ID --json '{\"ttl\":120}'",
            ],
        };
    }
}

pub mod account {
    use crate::core::tool::help::{Entry, Section};

    pub const ROOT: Entry = Entry {
        key: "cloudflare.account",
        usage: "runseal @tool cloudflare account <command> [args]",
        about: None,
        sections: &[Section {
            title: "Cloudflare account helpers",
            items: &[
                ("get --account-id <id>", "fetch one account object"),
                ("r2 <command> [args]", "manage account-level R2 resources"),
            ],
        }],
        examples: &[],
    };

    pub const GET: Entry = Entry {
        key: "cloudflare.account.get",
        usage: "runseal @tool cloudflare account get --account-id <id>",
        about: Some("Fetch one account object as JSON."),
        sections: &[Section {
            title: "Flags",
            items: &[("--account-id <id>", "target account identifier")],
        }],
        examples: &["runseal @tool cloudflare account get --account-id ACCOUNT_ID"],
    };

    pub mod r2 {
        use crate::core::tool::help::{Entry, Section};

        pub const ROOT: Entry = Entry {
            key: "cloudflare.account.r2",
            usage: "runseal @tool cloudflare account r2 <command> [args]",
            about: None,
            sections: &[Section {
                title: "Cloudflare account R2 helpers",
                items: &[("bucket list --account-id <id>", "list account buckets")],
            }],
            examples: &[],
        };

        pub mod bucket {
            use crate::core::tool::help::{Entry, Section};

            pub const ROOT: Entry = Entry {
                key: "cloudflare.account.r2.bucket",
                usage: "runseal @tool cloudflare account r2 bucket <command> [args]",
                about: None,
                sections: &[Section {
                    title: "Cloudflare account R2 bucket helpers",
                    items: &[("list --account-id <id>", "list account buckets")],
                }],
                examples: &[],
            };

            pub const LIST: Entry = Entry {
                key: "cloudflare.account.r2.bucket.list",
                usage: "runseal @tool cloudflare account r2 bucket list --account-id <id>",
                about: Some("Print R2 buckets as a JSON array."),
                sections: &[Section {
                    title: "Flags",
                    items: &[("--account-id <id>", "target account identifier")],
                }],
                examples: &[
                    "runseal @tool cloudflare account r2 bucket list --account-id ACCOUNT_ID",
                ],
            };
        }
    }
}

pub mod redirect {
    use crate::core::tool::help::{Entry, Section};

    pub const ROOT: Entry = Entry {
        key: "cloudflare.redirect-rule",
        usage: "runseal @tool cloudflare redirect-rule <command> [args]",
        about: None,
        sections: &[Section {
            title: "Cloudflare redirect-rule helpers",
            items: &[(
                "exact --ref <ref> --description <text> --host <host> --path <path> --target-url <url> [--status-code <code>]",
                "build one exact-match redirect rule payload",
            )],
        }],
        examples: &[],
    };

    pub const EXACT: Entry = Entry {
        key: "cloudflare.redirect-rule.exact",
        usage: "runseal @tool cloudflare redirect-rule exact --ref <ref> --description <text> --host <host> --path <path> --target-url <url> [--status-code <code>]",
        about: Some("Build one exact-match redirect rule payload as JSON."),
        sections: &[Section {
            title: "Flags",
            items: &[
                ("--ref <ref>", "stable rule reference name"),
                ("--description <text>", "human-readable rule description"),
                ("--host <host>", "matched request host"),
                ("--path <path>", "matched request path"),
                ("--target-url <url>", "redirect target URL"),
                ("--status-code <code>", "HTTP status code; default `302`"),
            ],
        }],
        examples: &[
            "runseal @tool cloudflare redirect-rule exact --ref manage-sh --description \"manage.sh\" --host runseal.perish.uk --path /manage.sh --target-url https://releases.runseal.perish.uk/manage.sh",
        ],
    };
}
