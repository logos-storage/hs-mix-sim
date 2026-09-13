mod adversary;
mod mixnet;
mod params;
mod path_sampler;
mod simulator;
mod summary;
mod time_based_path_sampler;
mod usermodel;

use adversary::basic::BasicAdversary;
use adversary::sybil_only::SybilOnlyAdversary;
use adversary::{Adversary, SybilAdversary};
use clap::{CommandFactory, Parser, ValueEnum, error::ErrorKind};
use mixnet::{Mixnet, MixnetConfig, MixnetGenerator};
use params::{DEFAULT_CSV_INTERVAL_SECONDS, DEFAULT_PATH_HOPS};
use path_sampler::alpha_sticky::AlphaStickyPathSampler;
use path_sampler::k_hops_fixed::KHopsFixedPathSampler;
use path_sampler::k_over_w::KOverWPathSampler;
use path_sampler::random::RandomPathSampler;
use simulator::Simulator;
use std::path::PathBuf;
use summary::{SdlmStrategy, SdlmSummary, SimulationConfigSummary};
use time_based_path_sampler::fixed_path::FixedPathSampler;
use usermodel::{
    DownloadSessionModel, HiddenServiceModel, SimpleModel, UserModelIterator, session_path_count,
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
    #[value(name = "k-hf")]
    KHopsFixed,
    #[value(name = "k-w")]
    KOverW,
    AlphaSticky,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum AdversaryChoice {
    Basic,
    SybilOnly,
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

    /// Hidden-service adversary (default: basic); requires --model hidden-service.
    #[arg(long, value_enum)]
    adversary: Option<AdversaryChoice>,

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

    /// Seconds between CSV time-series rows; does not affect simulated events.
    #[arg(long, default_value_t = DEFAULT_CSV_INTERVAL_SECONDS)]
    csv_interval: u32,
}

fn main() {
    let options = Options::parse();
    let model = options.model;
    if options.adversary.is_some() && !matches!(model, Model::HiddenService) {
        Options::command()
            .error(
                ErrorKind::ArgumentConflict,
                "--adversary is only supported with --model hidden-service",
            )
            .exit();
    }
    let adversary = options.adversary.unwrap_or(AdversaryChoice::Basic);
    let mode = options.mode.unwrap_or(match model {
        Model::HiddenService => Mode::FixedPath,
        _ => Mode::Random,
    });
    assert!(
        matches!(model, Model::HiddenService) == (mode == Mode::FixedPath),
        "hidden-service requires --mode fixed-path; fixed-path supports only hidden-service"
    );
    assert!(options.hops > 0, "--hops must be greater than zero");
    assert!(
        options.csv_interval > 0,
        "--csv-interval must be greater than zero"
    );
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

    let mixnet_generator = MixnetGenerator::new(MixnetConfig::default());
    let mixnet = mixnet_generator.generate_mixnet();

    let sampler_type = match mode {
        Mode::FixedPath => "FixedPathSampler",
        Mode::Random => "RandomPathSampler",
        Mode::KHopsFixed => "KHopsFixedPathSampler",
        Mode::KOverW => "KOverWPathSampler",
        Mode::AlphaSticky => "AlphaStickyPathSampler",
    };
    let model_type = match model {
        Model::Simple => "SimpleModel",
        Model::HiddenService => "HiddenServiceModel",
        Model::DownloadSession => "DownloadSessionModel",
    };
    let adversary_type = match (model, adversary) {
        (Model::HiddenService, AdversaryChoice::Basic) => "BasicAdversary",
        (Model::HiddenService, AdversaryChoice::SybilOnly) => "SybilOnlyAdversary",
        _ => "SybilAdversary",
    };
    let sdlm_strategy = match (model, mode) {
        (Model::DownloadSession, Mode::Random) => Some(SdlmStrategy::Random),
        (Model::DownloadSession, Mode::KHopsFixed) => Some(SdlmStrategy::KHopsFixed { fixed_hops }),
        (Model::DownloadSession, Mode::KOverW) => Some(SdlmStrategy::KOverW { k }),
        (Model::DownloadSession, Mode::AlphaSticky) => Some(SdlmStrategy::AlphaSticky { alpha }),
        _ => None,
    };
    let sdlm = sdlm_strategy.map(|strategy| {
        let total_nodes = mixnet.nodes().len();
        let malicious_nodes = mixnet
            .nodes()
            .iter()
            .filter(|node| node.is_malicious)
            .count();

        SdlmSummary::new(
            strategy,
            session_path_count(file_size, packet_size, options.hops),
            options.hops,
            total_nodes,
            malicious_nodes,
        )
    });
    let mut parameters = Vec::new();
    match mode {
        Mode::KHopsFixed => parameters.push(("fixed_hops", fixed_hops.to_string())),
        Mode::KOverW => parameters.push(("k", k.to_string())),
        Mode::AlphaSticky => parameters.push(("alpha", alpha.to_string())),
        Mode::FixedPath => {
            use time_based_path_sampler::fixed_path::{
                FIXED_PATH_COUNT, MAX_LIFETIME_SECONDS, MIN_LIFETIME_SECONDS,
            };
            parameters.extend([
                ("stored_path_count", FIXED_PATH_COUNT.to_string()),
                (
                    "path_lifetime_min_seconds",
                    MIN_LIFETIME_SECONDS.to_string(),
                ),
                (
                    "path_lifetime_max_seconds",
                    MAX_LIFETIME_SECONDS.to_string(),
                ),
                (
                    "path_lifetime_distribution",
                    "max_of_two_uniform_draws".to_owned(),
                ),
            ]);
        }
        Mode::Random => {}
    }
    match model {
        Model::HiddenService => {
            use adversary::basic::{COMPROMISE_PROBABILITY, COMPROMISE_WINDOW_SECONDS};
            match adversary {
                AdversaryChoice::Basic => parameters.extend([
                    (
                        "node_compromise_probability",
                        COMPROMISE_PROBABILITY.to_string(),
                    ),
                    (
                        "node_compromise_window_seconds",
                        COMPROMISE_WINDOW_SECONDS.to_string(),
                    ),
                    (
                        "node_compromise_delay_distribution",
                        "uniform_1_to_window_otherwise_never".to_owned(),
                    ),
                ]),
                AdversaryChoice::SybilOnly => {
                    parameters.push(("node_compromise_probability", "0".to_owned()))
                }
            }
        }
        Model::Simple => {
            use usermodel::{INTERVAL_MAX, INTERVAL_MIN};
            parameters.extend([
                ("message_interval_min_seconds", INTERVAL_MIN.to_string()),
                (
                    "message_interval_max_exclusive_seconds",
                    INTERVAL_MAX.to_string(),
                ),
                ("message_interval_distribution", "uniform".to_owned()),
            ]);
        }
        Model::DownloadSession => parameters.push(("packet_size_bytes", packet_size.to_string())),
    }
    let malicious_nodes = mixnet
        .nodes()
        .iter()
        .filter(|node| node.is_malicious)
        .count();
    let config = SimulationConfigSummary {
        days: options.days,
        csv_interval_seconds: options.csv_interval,
        mix_nodes: mixnet.nodes().len(),
        malicious_nodes,
        path_hops: options.hops,
        path_sampler_type: sampler_type,
        user_model_type: model_type,
        adversary_type,
        malicious_node_fraction: malicious_nodes as f64 / mixnet.nodes().len() as f64,
        sdlm,
        parameters,
        download_size_bytes: matches!(model, Model::DownloadSession).then_some(file_size),
    };
    let mut simulator = Simulator::new(options.users, config, options.csv);

    match (model, mode) {
        (Model::Simple, Mode::Random) => {
            let models = (0..options.users)
                .map(|_| {
                    UserModelIterator(SimpleModel::new(
                        &mixnet,
                        RandomPathSampler::new(options.hops),
                        SybilAdversary,
                        simulator.limit_sec(),
                    ))
                })
                .collect();
            simulator.simulate(models);
        }
        (Model::Simple, Mode::KHopsFixed) => {
            let models = (0..options.users)
                .map(|_| {
                    UserModelIterator(SimpleModel::new(
                        &mixnet,
                        KHopsFixedPathSampler::new(options.hops, fixed_hops),
                        SybilAdversary,
                        simulator.limit_sec(),
                    ))
                })
                .collect();
            simulator.simulate(models);
        }
        (Model::Simple, Mode::KOverW | Mode::AlphaSticky) => {
            unreachable!("session path samplers require the download-session model")
        }
        (Model::HiddenService, Mode::FixedPath) => match adversary {
            AdversaryChoice::Basic => simulate_hidden_services(
                &mut simulator,
                &mixnet,
                options.users,
                options.hops,
                BasicAdversary::new,
            ),
            AdversaryChoice::SybilOnly => simulate_hidden_services(
                &mut simulator,
                &mixnet,
                options.users,
                options.hops,
                SybilOnlyAdversary::new,
            ),
        },
        (Model::HiddenService, _) | (_, Mode::FixedPath) => {
            unreachable!("fixed-path is the only time-based sampler and requires hidden-service")
        }
        (Model::DownloadSession, Mode::Random) => {
            let models = (0..options.users)
                .map(|_| {
                    UserModelIterator(DownloadSessionModel::new(
                        &mixnet,
                        RandomPathSampler::new(options.hops),
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
                        &mixnet,
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
                        &mixnet,
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
                        &mixnet,
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

/// Each user gets a fresh adversary, while sharing the run's static mixnet.
fn simulate_hidden_services<A: Adversary + Send>(
    simulator: &mut Simulator,
    mixnet: &Mixnet,
    users: u32,
    hops: usize,
    make_adversary: impl Fn() -> A,
) {
    let models = (0..users)
        .map(|_| {
            UserModelIterator(HiddenServiceModel::new(
                mixnet,
                FixedPathSampler::new(hops, mixnet),
                make_adversary(),
                simulator.limit_sec(),
            ))
        })
        .collect();
    simulator.simulate(models);
}
