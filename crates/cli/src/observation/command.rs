pub(super) struct Shape {
    pub program: &'static str,
    pub target: &'static str,
    pub wrapper: &'static str,
    pub verb: &'static str,
    pub overrides: Vec<&'static str>,
}

impl Shape {
    pub fn read(argv: &[String]) -> Self {
        if argv.len() > 32 || argv.iter().take(32).any(|arg| arg.len() > 128) {
            return Self::opaque("other");
        }
        let Some((first, rest)) = argv.split_first() else {
            return Self::opaque("other");
        };
        if first == "env" {
            return Self::wrapped(rest);
        }
        let program = program(first);
        if program == "other" {
            return Self::opaque(program);
        }
        Self {
            program,
            target: program,
            wrapper: "none",
            verb: verb(rest),
            overrides: Vec::new(),
        }
    }

    fn wrapped(mut argv: &[String]) -> Self {
        let mut overrides = Vec::new();
        while let Some((first, rest)) = argv.split_first() {
            let Some((key, _)) = first.split_once('=') else {
                break;
            };
            if !assignment(key) {
                return Self::opaque("env");
            }
            overrides.push(if first == "GODEBUG=http2client=0" {
                "GODEBUG=http2client=0"
            } else {
                "other"
            });
            argv = rest;
        }
        let Some((first, rest)) = argv.split_first() else {
            return Self::opaque("env");
        };
        let target = program(first);
        if target == "other" {
            return Self::opaque("env");
        }
        Self {
            program: "env",
            target,
            wrapper: "env",
            verb: verb(rest),
            overrides,
        }
    }

    fn opaque(program: &'static str) -> Self {
        Self {
            program,
            target: "other",
            wrapper: "opaque",
            verb: "other",
            overrides: Vec::new(),
        }
    }
}

fn program(value: &str) -> &'static str {
    match value {
        "gh" => "gh",
        "concord" => "concord",
        "plumb" => "plumb",
        _ => "other",
    }
}

fn verb(argv: &[String]) -> &'static str {
    match argv.first().map(String::as_str) {
        Some("api") => "api",
        Some("issue") => "issue",
        Some("pr") => "pr",
        Some("acceptance") => "acceptance",
        Some("member") => "member",
        Some("release") => "release",
        Some("land") => "land",
        Some("metadata") => "metadata",
        Some("--version") => "--version",
        _ => "other",
    }
}

fn assignment(key: &str) -> bool {
    let mut bytes = key.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}
