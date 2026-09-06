use clap::Subcommand;
use plumb::skill::{Action, Ask, Depot, Done, Kit, Report};
use std::path::PathBuf;

const RELEASES: &str = "https://releases.runseal.perish.uk";
const DEPOT: &str = "https://depot.runseal.perish.uk";

#[derive(Debug, PartialEq)]
struct Rig {
    home: String,
    releases: String,
}

impl Default for Rig {
    fn default() -> Self {
        Self {
            home: plumb::config::data("runseal")
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            releases: RELEASES.to_string(),
        }
    }
}

impl Rig {
    fn resolve() -> Self {
        let default = Self::default();
        Self {
            home: plumb::config::value("RUNSEAL_HOME").unwrap_or(default.home),
            releases: plumb::config::value("RUNSEAL_RELEASES").unwrap_or(default.releases),
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum Deed {
    Install {
        #[arg(long, default_value = "stable")]
        channel: String,
        #[arg(long)]
        version: Option<String>,
        #[arg(long)]
        path: Option<PathBuf>,
        #[arg(long)]
        force: bool,
    },
    Upgrade {
        #[arg(long, default_value = "stable")]
        channel: String,
        #[arg(long)]
        version: Option<String>,
        #[arg(long = "dry-run")]
        dry: bool,
        #[arg(long)]
        json: bool,
    },
    Status {
        #[arg(long, default_value = "stable")]
        channel: String,
        #[arg(long)]
        version: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Stage {
        #[arg(long)]
        channel: String,
        #[arg(long)]
        version: String,
        #[arg(long)]
        path: PathBuf,
    },
    List,
    Uninstall,
}

pub fn run(deed: Deed) -> i32 {
    let rig = Rig::resolve();
    if rig.home.is_empty() {
        return sour("no data home; set RUNSEAL_HOME");
    }
    let kit = Kit {
        name: "runseal".to_string(),
        home: plumb::config::home().unwrap_or_else(|| PathBuf::from(".")),
        state: PathBuf::from(&rig.home).join("state").join("skills.json"),
        url: rig.releases,
    };
    let depot = kit.depot(DEPOT, "runseal", plumb::version!("RUNSEAL"));
    act(&depot, deed)
}

fn act(depot: &Depot<'_>, deed: Deed) -> i32 {
    match deed {
        Deed::Install {
            channel,
            version,
            path,
            force,
        } => told(
            "installed",
            depot.install(&Ask {
                channel,
                version,
                path,
                force,
            }),
            false,
        ),
        Deed::Upgrade {
            channel,
            version,
            dry,
            json,
        } => {
            let ask = Ask {
                channel,
                version,
                ..Ask::default()
            };
            if dry {
                report("upgrade_dry_run", depot.status(&ask), json)
            } else {
                told("upgraded", depot.upgrade(&ask), json)
            }
        }
        Deed::Status {
            channel,
            version,
            json,
        } => report(
            "status",
            depot.status(&Ask {
                channel,
                version,
                ..Ask::default()
            }),
            json,
        ),
        Deed::Stage {
            channel,
            version,
            path,
        } => told(
            "staged",
            depot.stage(&Ask {
                channel,
                version: Some(version),
                path: Some(path),
                ..Ask::default()
            }),
            false,
        ),
        Deed::List => tell(depot),
        Deed::Uninstall => told("removed", depot.uninstall(), false),
    }
}

fn tell(depot: &Depot<'_>) -> i32 {
    match depot.list() {
        Ok(records) => {
            for record in &records {
                println!(
                    "  {} {} {}",
                    record.agent,
                    record.version,
                    record.path.display()
                );
            }
            if records.is_empty() {
                println!("  no managed skill");
            }
            0
        }
        Err(error) => sour(&error.to_string()),
    }
}

fn told(action: &str, held: Result<Done, plumb::skill::Error>, json: bool) -> i32 {
    let done = match held {
        Ok(done) => done,
        Err(error) => return sour(&error.to_string()),
    };
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "action": action,
                "changed": done.kept.iter().map(|seat| serde_json::json!({
                    "agent": seat.agent,
                    "path": seat.path,
                })).collect::<Vec<_>>(),
                "unchanged": done.same.iter().map(|seat| serde_json::json!({
                    "agent": seat.agent,
                    "path": seat.path,
                })).collect::<Vec<_>>(),
                "skipped": done.left.iter().map(|skip| serde_json::json!({
                    "path": skip.path,
                    "reason": skip.note,
                })).collect::<Vec<_>>(),
            }))
            .expect("skill result should encode")
        );
    }
    for seat in &done.kept {
        if !json {
            println!("  {action} {} {}", seat.agent, seat.path.display());
        }
    }
    for seat in &done.same {
        if !json {
            println!("  unchanged {} {}", seat.agent, seat.path.display());
        }
    }
    for skip in &done.left {
        if !json {
            println!("  skipped {}: {}", skip.path.display(), skip.note);
        }
    }
    i32::from(done.kept.is_empty() && (done.same.is_empty() || !done.left.is_empty()))
}

fn report(operation: &str, held: Result<Report, plumb::skill::Error>, json: bool) -> i32 {
    let report = match held {
        Ok(report) => report,
        Err(error) => return sour(&error.to_string()),
    };
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "operation": operation,
                "channel": report.channel,
                "explicit": report.explicit,
                "target": report.target,
                "seats": report.seats,
            }))
            .expect("skill report should encode")
        );
    } else {
        println!("  target {} {}", report.channel, report.target.version);
        if report.seats.is_empty() {
            println!("  unmanaged");
        }
        for status in &report.seats {
            println!(
                "  {} {} -> {} {} {} {}",
                status.agent,
                status.installed,
                report.target.version,
                status.state,
                status.action,
                status.path.display()
            );
        }
    }
    i32::from(
        operation == "upgrade_dry_run"
            && (report.seats.is_empty()
                || report
                    .seats
                    .iter()
                    .any(|status| status.action == Action::Refuse)),
    )
}

fn sour(note: &str) -> i32 {
    eprintln!("runseal skill: {note}");
    1
}
