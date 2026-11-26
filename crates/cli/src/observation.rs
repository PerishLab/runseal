use locus::{Candidate, Context, Key, Role};
use plumb::observation::{Declaration, Run, view};
use serde_json::json;
use std::ffi::{OsStr, OsString};
use std::path::Path;

mod command;

pub(crate) fn start(cwd: &Path, command: &str) -> Option<Run> {
    let home = match runseal::core::config::Config::home(cwd) {
        Ok(home) => home,
        Err(error) => {
            eprintln!("runseal locus: {error}");
            return None;
        }
    };
    Run::start(
        Declaration {
            producer: "runseal",
            file: &home.join("config.toml"),
            buffer: home.join("state/locus"),
        },
        command,
    )
}

pub(crate) fn parent(frame: Option<&Context>) -> Vec<(OsString, Option<OsString>)> {
    let mut command = std::process::Command::new("runseal");
    command.env_remove(locus::collector::PARENT);
    if let Some(frame) = frame {
        plumb::observation::parent(&mut command, frame);
    }
    command
        .get_envs()
        .map(|(key, value)| (key.to_owned(), value.map(OsStr::to_owned)))
        .collect()
}

pub(crate) fn prepared(frame: &Context, argv: &[String], member: Option<&str>) {
    let Some((engine, _)) = view() else {
        return;
    };
    if let Err(error) =
        preparation(argv, member).and_then(|candidate| engine.append(frame, candidate))
    {
        eprintln!("runseal locus: {error}");
    }
}

fn preparation(argv: &[String], member: Option<&str>) -> Result<Candidate, locus::Error> {
    let shape = command::Shape::read(argv);
    let mut candidate = Candidate::event(json!([
        shape.overrides,
        shape.program,
        shape.target,
        shape.verb
    ]));
    for (role, key) in [
        ("runseal.exec", "prepared.v1"),
        ("runseal.program", shape.program),
        ("runseal.target", shape.target),
        ("runseal.wrapper", shape.wrapper),
    ] {
        candidate = candidate.explicit(Role::new(role)?, Key::new(key)?);
    }
    if let Some(key) = member
        .filter(|name| name.len() <= 64)
        .and_then(|name| Key::new(name).ok())
    {
        candidate = candidate.explicit(Role::new("runseal.member")?, key);
    }
    Ok(candidate)
}

pub(crate) fn provider(observation: &runseal::github::Observation) {
    let Some((engine, context)) = view() else {
        return;
    };
    let candidate = Candidate::event(json!({
        "event": "provider.call",
        "provider": observation.provider,
        "operation": observation.operation,
        "transport": observation.transport,
        "outcome": observation.outcome,
        "cause": observation.cause,
        "status": observation.status,
        "attempts": observation.attempts,
        "duration_ms": observation.elapsed,
        "partial": observation.partial,
        "wait": observation.wait,
    }))
    .ensure(Role::trace())
    .ensure(Role::span());
    if let Err(error) = engine.append(context, candidate) {
        eprintln!("runseal locus: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::{provider, view};
    use plumb::observation::{Declaration, Run};
    use serde_json::Value;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::thread;

    #[locus::trace(with = view())]
    fn invoke(address: std::net::SocketAddr) {
        let client = runseal::github::Client::loopback("github_pat_secret", address)
            .unwrap()
            .observer(provider);
        client.repository("SecretOrg", "PrivateRepo").unwrap();
    }

    #[test]
    fn github() {
        let home = tempfile::tempdir().unwrap();
        let file = home.path().join("config.toml");
        std::fs::write(
            &file,
            "[locus]\nenabled = true\nendpoint = 'http://127.0.0.1:0'\n",
        )
        .unwrap();
        let buffer = home.path().join("state/locus");
        let run = Run::start(
            Declaration {
                producer: "runseal",
                file: &file,
                buffer: buffer.clone(),
            },
            "tool",
        )
        .unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let body = r#"{"id":14,"full_name":"SecretOrg/PrivateRepo","private":true,"default_branch":"main"}"#;
        let reply = format!(
            "HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        let thread = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(&stream);
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
                    break;
                }
            }
            stream.write_all(reply.as_bytes()).unwrap();
        });

        invoke(address);
        thread.join().unwrap();
        run.finish(0);

        let mut atoms = Vec::new();
        for entry in std::fs::read_dir(&buffer).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy();
            if name == "active.jsonl" || name.starts_with("sealed-") {
                let content = std::fs::read_to_string(path).unwrap();
                atoms.extend(
                    content
                        .lines()
                        .map(|line| serde_json::from_str::<Value>(line).unwrap()),
                );
            }
        }
        let calls: Vec<&Value> = atoms
            .iter()
            .filter(|atom| atom["payload"]["event"] == "provider.call")
            .collect();
        assert_eq!(calls.len(), 1);
        let payload = &calls[0]["payload"];
        assert_eq!(calls[0]["producer"], "runseal");
        assert_eq!(payload["provider"], "github");
        assert_eq!(payload["operation"], "rest.get.repository");
        assert_eq!(payload["transport"], "rest");
        assert_eq!(payload["outcome"], "observed");
        assert_eq!(payload["status"], 200);
        assert_eq!(payload["attempts"], 1);
        assert!(payload["duration_ms"].is_u64());
        assert!(calls[0]["context"]["locus.trace"].is_string());
        assert!(calls[0]["context"]["locus.span"].is_string());
        let text = serde_json::to_string(&calls).unwrap();
        for forbidden in ["github_pat_secret", "SecretOrg", "PrivateRepo"] {
            assert!(!text.contains(forbidden), "{forbidden}");
        }
    }
}
