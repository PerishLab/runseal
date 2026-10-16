use locus::{Candidate, Context, Engine, Role};
use plumb::observation::{Declaration, Run, view};
use serde_json::json;
use std::path::Path;

mod command;

pub(crate) fn start(cwd: &Path, command: &str) -> Option<Run> {
    let home = match runseal::core::config::Config::build(None, Vec::new(), cwd) {
        Ok(config) => config.home,
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

pub(crate) fn prepared(argv: &[String]) {
    let Some((engine, context)) = view() else {
        return;
    };
    if let Err(error) = record(engine, context, argv) {
        eprintln!("runseal locus: {error}");
    }
}

fn record(engine: &Engine, context: &Context, argv: &[String]) -> Result<(), String> {
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
        candidate = candidate.explicit(
            Role::new(role).map_err(|error| error.to_string())?,
            locus::Key::new(key).map_err(|error| error.to_string())?,
        );
    }
    engine
        .append(context, candidate)
        .map_err(|error| error.to_string())?;
    Ok(())
}
