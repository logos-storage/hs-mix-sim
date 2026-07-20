mod params;
mod path_sampler;
mod simulator;
mod summary;
mod topologygen;
mod usermodel;

use clap::{Parser, ValueEnum};
use params::DEFAULT_PATH_HOPS;
use path_sampler::bandwidth_random::BandwidthRandomPathSampler;
use path_sampler::guard::PathSamplerWithGuards;
use path_sampler::random::RandomPathSampler;
use path_sampler::vanguard::PathSamplerWithVanguards;
use simulator::Simulator;
use std::path::PathBuf;
use topologygen::{TopologyConfig, TopologyGenerator};
use usermodel::{HiddenServiceModel, SimpleModel, UserModelInfo, UserModelIterator};

/// currently supported models
#[derive(Copy, Clone, Debug, ValueEnum)]
enum Model {
    Simple,
    HiddenService,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum Mode {
    Random,
    BandwidthRandom,
    Guard,
    Vanguard,
}

#[derive(Debug, Parser)]
#[command(author, version, about = "Free-route mixnet simulator")]
struct Options {
    /// Path-selection mode.
    #[arg(long, value_enum, default_value = "random")]
    mode: Mode,

    /// Number of hops in each sampled path.
    #[arg(long, default_value_t = DEFAULT_PATH_HOPS)]
    hops: usize,

    /// Number of vanguards used in vanguard mode.
    #[arg(long, required_if_eq("mode", "vanguard"))]
    vanguards: Option<usize>,

    /// User model.
    #[arg(long, value_enum, default_value = "simple")]
    model: Model,

    /// Write the output to CSV file.
    #[arg(long)]
    csv: Option<PathBuf>,

    /// Number of days to simulate.
    #[arg(long, default_value_t = 1)]
    days: u32,

    /// Number of users to simulate.
    #[arg(long, default_value_t = 5000)]
    users: u32,

    /// Duration of one topology epoch in seconds.
    #[arg(long, default_value_t = 3600)]
    epoch: u32,
}

fn main() {
    let options = Options::parse();
    assert!(options.hops > 0, "--hops must be greater than zero");
    assert!(options.epoch > 0, "--epoch must be greater than zero");
    let vanguards = options.vanguards.unwrap_or(0);
    if options.mode == Mode::Vanguard {
        assert!(vanguards > 0, "--vanguards must be greater than zero");
        assert!(
            options.hops > 1 && vanguards < options.hops - 1,
            "--vanguards must be less than --hops - 1"
        );
    }

    let topology_config = TopologyConfig {
        guard_mode: matches!(options.mode, Mode::Guard | Mode::Vanguard),
        epochs: epochs_needed(options.days, options.epoch),
        ..TopologyConfig::default()
    };
    let topology_generator = TopologyGenerator::new(topology_config);
    let topologies = topology_generator.generate_topologies();

    let sampler_type = match options.mode {
        Mode::Random => "RandomPathSampler",
        Mode::BandwidthRandom => "BandwidthRandomPathSampler",
        Mode::Guard => "PathSamplerWithGuards",
        Mode::Vanguard => "PathSamplerWithVanguards",
    };
    let model_type = match options.model {
        Model::Simple => "SimpleModel",
        Model::HiddenService => "HiddenServiceModel",
    };
    let mut simulator = Simulator::new(
        options.users,
        topology_generator,
        options.days,
        options.epoch,
        options.hops,
        options.csv,
        sampler_type,
        model_type,
    );

    match (options.model, options.mode) {
        (Model::Simple, Mode::Random) => {
            let models = (0..options.users)
                .map(|_| {
                    UserModelIterator(SimpleModel::new(
                        UserModelInfo::new(&topologies, options.epoch),
                        RandomPathSampler::new(options.hops),
                    ))
                })
                .collect();
            simulator.simulate(models);
        }
        (Model::Simple, Mode::BandwidthRandom) => {
            let models = (0..options.users)
                .map(|_| {
                    UserModelIterator(SimpleModel::new(
                        UserModelInfo::new(&topologies, options.epoch),
                        BandwidthRandomPathSampler::new(options.hops),
                    ))
                })
                .collect();
            simulator.simulate(models);
        }
        (Model::Simple, Mode::Guard) => {
            let models = (0..options.users)
                .map(|_| {
                    UserModelIterator(SimpleModel::new(
                        UserModelInfo::new(&topologies, options.epoch),
                        PathSamplerWithGuards::new(options.hops),
                    ))
                })
                .collect();
            simulator.simulate(models);
        }
        (Model::Simple, Mode::Vanguard) => {
            let models = (0..options.users)
                .map(|_| {
                    UserModelIterator(SimpleModel::new(
                        UserModelInfo::new(&topologies, options.epoch),
                        PathSamplerWithVanguards::new(options.hops, vanguards),
                    ))
                })
                .collect();
            simulator.simulate(models);
        }
        (Model::HiddenService, Mode::Random) => {
            let models = (0..options.users)
                .map(|_| {
                    UserModelIterator(HiddenServiceModel::new(
                        UserModelInfo::new(&topologies, options.epoch),
                        RandomPathSampler::new(options.hops),
                    ))
                })
                .collect();
            simulator.simulate(models);
        }
        (Model::HiddenService, Mode::BandwidthRandom) => {
            let models = (0..options.users)
                .map(|_| {
                    UserModelIterator(HiddenServiceModel::new(
                        UserModelInfo::new(&topologies, options.epoch),
                        BandwidthRandomPathSampler::new(options.hops),
                    ))
                })
                .collect();
            simulator.simulate(models);
        }
        (Model::HiddenService, Mode::Guard) => {
            let models = (0..options.users)
                .map(|_| {
                    UserModelIterator(HiddenServiceModel::new(
                        UserModelInfo::new(&topologies, options.epoch),
                        PathSamplerWithGuards::new(options.hops),
                    ))
                })
                .collect();
            simulator.simulate(models);
        }
        (Model::HiddenService, Mode::Vanguard) => {
            let models = (0..options.users)
                .map(|_| {
                    UserModelIterator(HiddenServiceModel::new(
                        UserModelInfo::new(&topologies, options.epoch),
                        PathSamplerWithVanguards::new(options.hops, vanguards),
                    ))
                })
                .collect();
            simulator.simulate(models);
        }
    }
}

fn epochs_needed(days: u32, epoch_seconds: u32) -> u32 {
    let duration_seconds = u64::from(days) * 24 * 60 * 60;
    let epochs = duration_seconds / u64::from(epoch_seconds) + 1;
    u32::try_from(epochs).expect("requested simulation duration needs too many topology epochs")
}
