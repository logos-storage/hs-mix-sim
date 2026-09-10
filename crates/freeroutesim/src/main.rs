mod adversary;
mod params;
mod path_sampler;
mod simulator;
mod summary;
mod time_based_path_sampler;
mod topologygen;
mod usermodel;

use adversary::SybilAdversary;
use adversary::basic::BasicAdversary;
use clap::{Parser, ValueEnum};
use params::DEFAULT_PATH_HOPS;
use path_sampler::alpha_sticky::AlphaStickyPathSampler;
use path_sampler::bandwidth_random::BandwidthRandomPathSampler;
use path_sampler::guard::PathSamplerWithGuards;
use path_sampler::k_hops_fixed::KHopsFixedPathSampler;
use path_sampler::k_over_w::KOverWPathSampler;
use path_sampler::random::RandomPathSampler;
use path_sampler::vanguard::PathSamplerWithVanguards;
use simulator::Simulator;
use std::path::PathBuf;
use summary::{SdlmStrategy, SdlmSummary};
use time_based_path_sampler::fixed_path::FixedPathSampler;
use topologygen::{TopologyConfig, TopologyGenerator};
use usermodel::{
    DownloadSessionModel, HiddenServiceModel, SimpleModel, UserModelInfo, UserModelIterator,
    session_path_count,
};

/// currently supported models
#[derive(Copy, Clone, Debug, ValueEnum)]
enum Model {
    Simple,
    HiddenService,
    DownloadSession,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum Mode {
    FixedPath,
    Random,
    BandwidthRandom,
    Guard,
    Vanguard,
    #[value(name = "k-hf")]
    KHopsFixed,
    #[value(name = "k-w")]
    KOverW,
    AlphaSticky,
}

#[derive(Debug, Parser)]
#[command(author, version, about = "Free-route mixnet simulator")]
struct Options {
    /// Path-selection mode (fixed-path for hidden-service, random otherwise).
    #[arg(long, value_enum)]
    mode: Option<Mode>,

    /// Number of hops in each sampled path.
    #[arg(long, default_value_t = DEFAULT_PATH_HOPS)]
    hops: usize,

    /// Number of vanguards used in vanguard mode.
    #[arg(long, required_if_eq("mode", "vanguard"))]
    vanguards: Option<usize>,

    /// Number of persistent hop positions used in K-HF mode.
    #[arg(long, required_if_eq("mode", "k-hf"))]
    fixed_hops: Option<usize>,

    /// Number of candidates per logical hop in K/W mode.
    #[arg(long, required_if_eq("mode", "k-w"))]
    k: Option<usize>,

    /// Probability of reusing a previous path in alpha-sticky mode.
    #[arg(long, required_if_eq("mode", "alpha-sticky"))]
    alpha: Option<f64>,

    /// User model.
    #[arg(long, value_enum, default_value = "simple")]
    model: Model,

    /// File size in bytes, required by the download-session model.
    #[arg(long, required_if_eq("model", "download-session"))]
    file_size: Option<u64>,

    /// Total serialized Mix-packet size in bytes, required by the download-session model.
    #[arg(long, required_if_eq("model", "download-session"))]
    packet_size: Option<u64>,

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
    let model = options.model;
    let mode = options.mode.unwrap_or(match model {
        Model::HiddenService => Mode::FixedPath,
        _ => Mode::Random,
    });
    assert!(
        matches!(model, Model::HiddenService) == (mode == Mode::FixedPath),
        "hidden-service requires --mode fixed-path; fixed-path supports only hidden-service"
    );
    assert!(options.hops > 0, "--hops must be greater than zero");
    assert!(options.epoch > 0, "--epoch must be greater than zero");
    let vanguards = options.vanguards.unwrap_or(0);
    if mode == Mode::Vanguard {
        assert!(vanguards > 0, "--vanguards must be greater than zero");
        assert!(
            options.hops > 1 && vanguards < options.hops - 1,
            "--vanguards must be less than --hops - 1"
        );
    }
    let fixed_hops = options.fixed_hops.unwrap_or(0);
    if mode == Mode::KHopsFixed {
        assert!(
            matches!(model, Model::Simple | Model::DownloadSession),
            "K-HF mode currently supports only the simple and download-session models"
        );
        assert!(
            fixed_hops <= options.hops,
            "--fixed-hops must not exceed --hops"
        );
    }
    let k = options.k.unwrap_or(0);
    if mode == Mode::KOverW {
        assert!(
            matches!(model, Model::DownloadSession),
            "K/W mode supports only the download-session model"
        );
        assert!(k > 0, "--k must be greater than zero");
    }
    let alpha = options.alpha.unwrap_or(0.0);
    if mode == Mode::AlphaSticky {
        assert!(
            matches!(model, Model::DownloadSession),
            "alpha-sticky mode supports only the download-session model"
        );
        assert!(
            (0.0..=1.0).contains(&alpha),
            "--alpha must be between 0 and 1"
        );
    }
    let file_size = options.file_size.unwrap_or(0);
    let packet_size = options.packet_size.unwrap_or(0);
    if matches!(model, Model::DownloadSession) {
        assert!(file_size > 0, "--file-size must be greater than zero");
        assert!(packet_size > 0, "--packet-size must be greater than zero");
    }

    let topology_config = TopologyConfig {
        guard_mode: matches!(mode, Mode::Guard | Mode::Vanguard),
        epochs: if matches!(model, Model::DownloadSession) {
            1
        } else {
            epochs_needed(options.days, options.epoch)
        },
        ..TopologyConfig::default()
    };
    let topology_generator = TopologyGenerator::new(topology_config);
    let topologies = topology_generator.generate_topologies();

    let sampler_type = match mode {
        Mode::FixedPath => "FixedPathSampler",
        Mode::Random => "RandomPathSampler",
        Mode::BandwidthRandom => "BandwidthRandomPathSampler",
        Mode::Guard => "PathSamplerWithGuards",
        Mode::Vanguard => "PathSamplerWithVanguards",
        Mode::KHopsFixed => "KHopsFixedPathSampler",
        Mode::KOverW => "KOverWPathSampler",
        Mode::AlphaSticky => "AlphaStickyPathSampler",
    };
    let model_type = match model {
        Model::Simple => "SimpleModel",
        Model::HiddenService => "HiddenServiceModel",
        Model::DownloadSession => "DownloadSessionModel",
    };
    let sdlm_strategy = match (model, mode) {
        (Model::DownloadSession, Mode::Random) => Some(SdlmStrategy::Random),
        (Model::DownloadSession, Mode::KHopsFixed) => Some(SdlmStrategy::KHopsFixed { fixed_hops }),
        (Model::DownloadSession, Mode::KOverW) => Some(SdlmStrategy::KOverW { k }),
        (Model::DownloadSession, Mode::AlphaSticky) => Some(SdlmStrategy::AlphaSticky { alpha }),
        _ => None,
    };
    let sdlm = sdlm_strategy.map(|strategy| {
        let topology = topologies
            .first()
            .expect("download-session S-DLM needs one topology snapshot");
        let active_nodes = topology.active().len();
        let malicious_nodes = topology
            .active()
            .iter()
            .filter(|node| node.is_malicious)
            .count();

        SdlmSummary::new(
            strategy,
            session_path_count(file_size, packet_size, options.hops),
            options.hops,
            active_nodes,
            malicious_nodes,
        )
    });
    let mut simulator = Simulator::new(
        options.users,
        topology_generator,
        options.days,
        options.epoch,
        options.hops,
        options.csv,
        sampler_type,
        model_type,
        sdlm,
    );

    match (model, mode) {
        (Model::Simple, Mode::Random) => {
            let models = (0..options.users)
                .map(|_| {
                    UserModelIterator(SimpleModel::new(
                        UserModelInfo::new(&topologies, options.epoch),
                        RandomPathSampler::new(options.hops),
                        SybilAdversary,
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
                        SybilAdversary,
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
                        SybilAdversary,
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
                        SybilAdversary,
                    ))
                })
                .collect();
            simulator.simulate(models);
        }
        (Model::Simple, Mode::KHopsFixed) => {
            let models = (0..options.users)
                .map(|_| {
                    UserModelIterator(SimpleModel::new(
                        UserModelInfo::new(&topologies, options.epoch),
                        KHopsFixedPathSampler::new(options.hops, fixed_hops),
                        SybilAdversary,
                    ))
                })
                .collect();
            simulator.simulate(models);
        }
        (Model::Simple, Mode::KOverW | Mode::AlphaSticky) => {
            unreachable!("session path samplers require the download-session model")
        }
        (Model::HiddenService, Mode::FixedPath) => {
            let initial_topology = topologies
                .first()
                .expect("hidden service needs an initial topology");
            let models = (0..options.users)
                .map(|_| {
                    UserModelIterator(HiddenServiceModel::new(
                        UserModelInfo::new(&topologies, options.epoch),
                        FixedPathSampler::new(options.hops, initial_topology),
                        BasicAdversary::new(),
                        simulator.limit_sec(),
                    ))
                })
                .collect();
            simulator.simulate(models);
        }
        (Model::HiddenService, _) | (_, Mode::FixedPath) => {
            unreachable!("fixed-path is the only time-based sampler and requires hidden-service")
        }
        (Model::DownloadSession, Mode::Random) => {
            let models = (0..options.users)
                .map(|_| {
                    UserModelIterator(DownloadSessionModel::new(
                        UserModelInfo::new(&topologies, options.epoch),
                        RandomPathSampler::new(options.hops),
                        SybilAdversary,
                        file_size,
                        packet_size,
                    ))
                })
                .collect();
            simulator.simulate(models);
        }
        (Model::DownloadSession, Mode::BandwidthRandom) => {
            let models = (0..options.users)
                .map(|_| {
                    UserModelIterator(DownloadSessionModel::new(
                        UserModelInfo::new(&topologies, options.epoch),
                        BandwidthRandomPathSampler::new(options.hops),
                        SybilAdversary,
                        file_size,
                        packet_size,
                    ))
                })
                .collect();
            simulator.simulate(models);
        }
        (Model::DownloadSession, Mode::Guard) => {
            let models = (0..options.users)
                .map(|_| {
                    UserModelIterator(DownloadSessionModel::new(
                        UserModelInfo::new(&topologies, options.epoch),
                        PathSamplerWithGuards::new(options.hops),
                        SybilAdversary,
                        file_size,
                        packet_size,
                    ))
                })
                .collect();
            simulator.simulate(models);
        }
        (Model::DownloadSession, Mode::Vanguard) => {
            let models = (0..options.users)
                .map(|_| {
                    UserModelIterator(DownloadSessionModel::new(
                        UserModelInfo::new(&topologies, options.epoch),
                        PathSamplerWithVanguards::new(options.hops, vanguards),
                        SybilAdversary,
                        file_size,
                        packet_size,
                    ))
                })
                .collect();
            simulator.simulate(models);
        }
        (Model::DownloadSession, Mode::KHopsFixed) => {
            let models = (0..options.users)
                .map(|_| {
                    UserModelIterator(DownloadSessionModel::new(
                        UserModelInfo::new(&topologies, options.epoch),
                        KHopsFixedPathSampler::new(options.hops, fixed_hops),
                        SybilAdversary,
                        file_size,
                        packet_size,
                    ))
                })
                .collect();
            simulator.simulate(models);
        }
        (Model::DownloadSession, Mode::KOverW) => {
            let models = (0..options.users)
                .map(|_| {
                    UserModelIterator(DownloadSessionModel::new(
                        UserModelInfo::new(&topologies, options.epoch),
                        KOverWPathSampler::new(options.hops, k),
                        SybilAdversary,
                        file_size,
                        packet_size,
                    ))
                })
                .collect();
            simulator.simulate(models);
        }
        (Model::DownloadSession, Mode::AlphaSticky) => {
            let models = (0..options.users)
                .map(|_| {
                    UserModelIterator(DownloadSessionModel::new(
                        UserModelInfo::new(&topologies, options.epoch),
                        AlphaStickyPathSampler::new(options.hops, alpha),
                        SybilAdversary,
                        file_size,
                        packet_size,
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
