use super::{ENABLED, LINK, configure, lines, records, run, spans};

#[test]
fn member() {
    let edge = "界".repeat(21) + "A";
    let excess = edge.clone() + "B";
    let cases = [
        (None, None),
        (Some(""), None),
        (Some(excess.as_str()), None),
        (Some("Control\u{85}sentinel"), None),
        (Some(edge.as_str()), Some(edge.as_str())),
        (Some("Example Member"), Some("Example Member")),
    ];
    for (declared, expected) in cases {
        let home = tempfile::tempdir().unwrap();
        let root = home.path();
        let mut profile = "[env.vars]\nPRIVATE_TOKEN = 'credential-sentinel'\n".to_string();
        if let Some(name) = declared {
            profile.push_str(&format!("GROK_BOT_IDENTITY = '{name}'\n"));
        }
        let path = root.join("runseal.private-profile-sentinel.toml");
        std::fs::write(&path, profile).unwrap();
        let args = [
            ":private-profile-sentinel",
            "sh",
            "-c",
            "printf '%s\\n' \"$GROK_BOT_IDENTITY\" \"$PRIVATE_TOKEN\" \"$1\"; exit 17",
            "sh",
            "private-argument-sentinel",
        ];
        let baseline = run(root, &args);
        configure(root, ENABLED);
        let observed = run(root, &args);
        assert_eq!(baseline.status.code(), Some(17));
        assert_eq!(observed.status.code(), baseline.status.code());
        assert_eq!(observed.stdout, baseline.stdout);
        assert_eq!(lines(&observed.stderr), lines(&baseline.stderr));
        let applied = declared.unwrap_or("inherited-member-sentinel");
        assert_eq!(
            observed.stdout,
            format!("{applied}\ncredential-sentinel\nprivate-argument-sentinel\n").as_bytes()
        );
        let atoms = records(root);
        let prepared: Vec<_> = atoms
            .iter()
            .filter(|atom| atom["context"]["runseal.exec"] == "prepared.v1")
            .collect();
        assert_eq!(prepared.len(), 1);
        assert_eq!(prepared[0]["context"]["runseal.member"].as_str(), expected);
        assert_eq!(
            prepared[0]["context"]["locus.span"].as_str(),
            Some(spans(&atoms)[0].as_str())
        );
        assert_eq!(
            atoms
                .iter()
                .filter(|atom| atom["context"].get("runseal.member").is_some())
                .count(),
            usize::from(expected.is_some())
        );
        let text = serde_json::to_string(&atoms).unwrap();
        for forbidden in [
            "inherited-member-sentinel",
            "credential-sentinel",
            "PRIVATE_TOKEN",
            "private-argument-sentinel",
            "private-profile-sentinel",
            root.to_str().unwrap(),
        ] {
            assert!(!text.contains(forbidden), "{forbidden}");
        }
        if let (Some(invalid), None) = (declared.filter(|name| !name.is_empty()), expected) {
            assert!(!text.contains(invalid));
        }

        configure(root, "[locus]\nenabled = 'malformed'\n");
        let refused = run(root, &args);
        assert_eq!(refused.status.code(), baseline.status.code());
        assert_eq!(refused.stdout, baseline.stdout);
    }
}

#[test]
fn nested() {
    let home = tempfile::tempdir().expect("home");
    configure(home.path(), ENABLED);
    let inner = env!("CARGO_BIN_EXE_runseal");
    std::fs::write(
        home.path().join("runseal.outer.toml"),
        "[env.vars]\nGROK_BOT_IDENTITY = 'Example Member'\n",
    )
    .unwrap();
    let result = run(home.path(), &[":outer", inner, ":", "sh", "-c", LINK]);
    assert_eq!(result.status.code(), Some(17));
    let atoms = records(home.path());
    let spans = spans(&atoms);
    assert_eq!(spans.len(), 2);
    let written = String::from_utf8(result.stdout).unwrap();
    assert!(spans.contains(&written));
    let outer = spans.iter().find(|span| **span != written).unwrap();
    let labels: Vec<_> = atoms
        .iter()
        .filter(|atom| atom["context"].get("runseal.member").is_some())
        .collect();
    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0]["context"]["runseal.member"], "Example Member");
    assert_eq!(labels[0]["context"]["locus.span"], outer.as_str());
    assert!(atoms.iter().any(|atom| {
        atom["context"]["locus.parent"] == outer.as_str()
            && atom["choices"][0]["role"] == "locus.parent"
    }));
}
