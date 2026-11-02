use crate::tencent::tat;
use clap::{Args, Subcommand};
use std::{io::Read, path::PathBuf};

#[derive(Args)]
#[command(
    about = "One atomic TAT operation",
    after_help = "Run sends one public SHELL script to one Lighthouse, without saved command, parameters or COS output. Persist exact intent before dispatch; no ClientToken or automatic replay exists. Accepted is not execution success. Task requires expected invocation/command/instance and optional exact task ID; --output explicitly returns bounded public output, base64 in JSON. SDK callers receive decoded bytes. Scripts and outputs must contain no credentials/private material; provider history may persist even with SaveCommand=false. Hardrig owns polling, policy and SSH trust."
)]
pub(in crate::tencent) struct Request {
    #[arg(long)]
    region: String,
    #[command(subcommand)]
    operation: Operation,
}

#[derive(Subcommand)]
enum Operation {
    Run {
        #[arg(long)]
        instance: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        username: String,
        #[arg(long)]
        directory: String,
        #[arg(long)]
        timeout: u64,
        #[arg(long = "script-file")]
        script: PathBuf,
    },
    Task {
        #[arg(long)]
        invocation: String,
        #[arg(long)]
        command: String,
        #[arg(long)]
        instance: String,
        #[arg(long)]
        task: Option<String>,
        #[arg(long)]
        output: bool,
    },
    Regions,
    Agent {
        #[arg(long)]
        id: String,
    },
}

impl Request {
    pub(in crate::tencent) fn convert(&self) -> Result<tat::Request, crate::tencent::Failure> {
        Ok(tat::Request {
            region: self.region.clone(),
            operation: match &self.operation {
                Operation::Run {
                    instance,
                    name,
                    username,
                    directory,
                    timeout,
                    script,
                } => tat::Operation::Run(tat::Script {
                    instance: instance.clone(),
                    name: name.clone(),
                    user: username.clone(),
                    directory: directory.clone(),
                    timeout: *timeout,
                    content: content(script)?,
                }),
                Operation::Task {
                    invocation,
                    command,
                    instance,
                    task,
                    output,
                } => tat::Operation::Task(tat::Query {
                    target: tat::Target {
                        invocation: invocation.clone(),
                        command: command.clone(),
                        instance: instance.clone(),
                    },
                    id: task.clone(),
                    output: *output,
                }),
                Operation::Regions => tat::Operation::Regions,
                Operation::Agent { id } => tat::Operation::Lighthouse { id: id.clone() },
            },
        })
    }
}

fn content(path: &PathBuf) -> Result<String, crate::tencent::Failure> {
    let refused = || {
        crate::tencent::Failure::new(
            crate::tencent::Class::Input,
            "TAT script file must be bounded public UTF-8 content; never send private material",
        )
    };
    let file = std::fs::File::open(path).map_err(|_| refused())?;
    if !file.metadata().map_err(|_| refused())?.is_file() {
        return Err(refused());
    }
    let mut content = String::new();
    file.take(49153)
        .read_to_string(&mut content)
        .map_err(|_| refused())?;
    if content.len() > 49152 {
        return Err(refused());
    }
    Ok(content)
}
