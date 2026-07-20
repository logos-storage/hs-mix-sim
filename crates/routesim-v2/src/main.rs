//! Simulating behavioural activity within a Mixnet
//!
//! This tool evaluates the probability of deanonymization through time,
//! assuming some level of adversarial activity among the mixes.
//!
//! We expect to take in input mixnet [`topologies`](config::TopologyConfig).
//! The simple model samples one message per user at a time and virtually sends
//! those messages through the mixnet.
//!
//! The simulator appliess a Monte Carlo method to draw paths and outputs path
//! information for each message sent by each sample.
//!

mod config;
mod simulation;
mod simplehiddenservicemodel;
mod simplemodel;
mod usermodel;

use clap::{ArgEnum, Parser};
use config::TopologyConfig;
use simulation::Runable;
use simplehiddenservicemodel::*;
use simplemodel::*;

/// currently supported models
#[derive(Copy, Clone, Debug, ArgEnum)]
enum Model {
    Simple,
    SimpleHiddenService,
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Opts {
    #[clap(
        short = 'i',
        long,
        required = true,
        parse(from_os_str),
        help = "MTG layout CSV containing one or more topology epochs"
    )]
    topology_file: std::path::PathBuf,
    #[clap(long, default_value = "1", help = "Number of simulated days")]
    days: u32,
    #[clap(long, default_value = "5000", help = "Number of users to simulate")]
    users: u32,
    #[clap(
        short,
        long,
        default_value = "86401",
        help = "Validity period for a given topologies"
    )]
    epoch: u32,
    #[clap(
        short,
        long,
        help = "Print human-readable route output, one route per line"
    )]
    to_console: bool,
    #[clap(long, help = "Print aggregate compromise summary after simulation")]
    summary: bool,
    #[clap(
        long,
        arg_enum,
        default_value = "simple",
        help = "User/service traffic model to simulate"
    )]
    model: Model,
    #[clap(short, help = "Disable persistent guard and vanguard positions")]
    disable_guards: bool,
    #[clap(
        long,
        help = "Disable the persistent vanguard position while keeping guards enabled"
    )]
    disable_vanguards: bool,
}

fn main() {
    let opts: Opts = Opts::parse();

    let mut topologies: Vec<TopologyConfig> = config::load(&opts.topology_file);
    // We need sorting the topologies for accessing the right one depending
    // on the current epoch
    topologies.sort_by(|a, b| a.epoch.cmp(&b.epoch));
    let n = topologies.len();

    let mut epoch = opts.epoch;
    // check whether the parameters days; config and epoch make sense
    if epoch * n as u32 <= opts.days * 24 * 60 * 60 {
        eprintln!("Make sure you have enough configuration files, and that the epoch and days value make sense!");
        epoch = 86400 * opts.days + 1;
        eprintln!("Setting epoch to {epoch}. Maybe you want to change that");
    }
    let mut runner = Runable::new(opts.users, topologies, opts.days, epoch);

    if !opts.disable_guards {
        runner.with_guards();
    }
    if !opts.disable_guards && !opts.disable_vanguards {
        runner.with_vanguards();
    }
    if opts.to_console {
        runner.with_console();
    }
    if opts.summary {
        runner.with_summary();
    }

    match opts.model {
        Model::Simple => {
            let usermodels = runner.init::<SimpleSynchronousModel>();
            runner.run(usermodels);
        }
        Model::SimpleHiddenService => {
            let usermodels = runner.init::<SimpleHiddenServiceModel>();
            runner.run(usermodels);
        }
    }
}
