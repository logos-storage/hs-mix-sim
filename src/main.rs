mod adversary;
mod mixnet;
mod params;
mod path_sampler;
mod simulator;
mod summary;
mod time_based_path_sampler;
mod usermodel;

use adversary::configured::HiddenServiceAdversary;
use adversary::{Adversary, SybilAdversary};
use clap::{CommandFactory, Parser, ValueEnum, error::ErrorKind};
use mixnet::{Mixnet, MixnetConfig, MixnetGenerator};
use params::{DEFAULT_CSV_INTERVAL_SECONDS, DEFAULT_PATH_HOPS};
use path_sampler::alpha_sticky::AlphaStickyPathSampler;
use path_sampler::k_hops_fixed::KHopsFixedPathSampler;
use path_sampler::k_over_w::KOverWPathSampler;
use path_sampler::profiles::DOWNLOAD_PROFILES;
use path_sampler::random::RandomPathSampler;
use simulator::Simulator;
use std::path::PathBuf;
use summary::{SdlmStrategy, SdlmSummary, SimulationConfigSummary};
use time_based_path_sampler::TimeBasedPathSampler;
use time_based_path_sampler::fixed_path::FixedPathSampler;
use time_based_path_sampler::fixed_topology::{FPOFTSampler, FixedTopologySampler, profiles};
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
    FixedTopology,
    Fpoft,
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
    #[value(alias = "APT")]
    Apt,
    #[value(alias = "FVEY")]
    Fvey,
    Rubberhose1,
    Rubberhose2,
}

impl AdversaryChoice {
    fn create(self) -> HiddenServiceAdversary {
        match self {
            Self::Basic => HiddenServiceAdversary::basic(),
            Self::SybilOnly => HiddenServiceAdversary::sybil_only(),
            Self::Apt => HiddenServiceAdversary::apt(),
            Self::Fvey => HiddenServiceAdversary::fvey(),
            Self::Rubberhose1 => HiddenServiceAdversary::rubberhose1(),
            Self::Rubberhose2 => HiddenServiceAdversary::rubberhose2(),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Basic => "BasicAdversary",
            Self::SybilOnly => "SybilOnlyAdversary",
            Self::Apt => "APTAdversary",
            Self::Fvey => "FVEYAdversary",
            Self::Rubberhose1 => "Rubberhose1Adversary",
            Self::Rubberhose2 => "Rubberhose2Adversary",
        }
    }
}

#[derive(Debug, Parser)]
#[command(author, version, about = "Free-route mixnet simulator")]
struct Options {
    /// Path-selection mode (fixed-path for hidden-service, random otherwise).
    #[arg(long, value_enum)]
    mode: Option<Mode>,

    /// Hops per path (inferred for profiles, otherwise 3).
    #[arg(long)]
    hops: Option<usize>,

    /// Persistent hop positions in K-HF or K/W (K/W defaults to all hops).
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

    /// Vanguard-inspired profile; requires hidden-service and fixed-topology (default: vanguard1).
    #[arg(long, value_parser = clap::builder::PossibleValuesParser::new(
        profiles::ALL_PROFILES.iter().map(|profile| profile.name)
    ))]
    topology_profile: Option<String>,

    /// Active-path topology profile; requires hidden-service and fpoft (default: STANDARD).
    #[arg(long, value_parser = clap::builder::PossibleValuesParser::new(
        profiles::ALL_FPOFT_PROFILES.iter().map(|profile| profile.name)
    ))]
    fpoft_profile: Option<String>,

    /// Download K/W profile; sets hops, fixed hops, and K together.
    #[arg(long, conflicts_with_all = ["mode", "hops", "fixed_hops", "k", "alpha"],
        value_parser = clap::builder::PossibleValuesParser::new(
            DOWNLOAD_PROFILES.iter().map(|profile| profile.name)
        ))]
    download_profile: Option<String>,

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
    if options.download_profile.is_some() && !matches!(model, Model::DownloadSession) {
        Options::command()
            .error(
                ErrorKind::ArgumentConflict,
                "--download-profile requires --model download-session",
            )
            .exit();
    }
    let download_profile = options.download_profile.as_deref().map(|name| {
        *DOWNLOAD_PROFILES
            .iter()
            .find(|profile| profile.name == name)
            .unwrap()
    });
    let adversary = options.adversary.unwrap_or(AdversaryChoice::Basic);
    let mode = options.mode.unwrap_or(match model {
        Model::HiddenService => Mode::FixedPath,
        Model::DownloadSession if download_profile.is_some() => Mode::KOverW,
        _ => Mode::Random,
    });
    if options.topology_profile.is_some()
        && (!matches!(model, Model::HiddenService) || mode != Mode::FixedTopology)
    {
        Options::command()
            .error(
                ErrorKind::ArgumentConflict,
                "--topology-profile requires --model hidden-service --mode fixed-topology",
            )
            .exit();
    }
    if options.fpoft_profile.is_some()
        && (!matches!(model, Model::HiddenService) || mode != Mode::Fpoft)
    {
        Options::command()
            .error(
                ErrorKind::ArgumentConflict,
                "--fpoft-profile requires --model hidden-service --mode fpoft",
            )
            .exit();
    }
    let fpoft_profile =
        options
            .fpoft_profile
            .as_deref()
            .map_or(profiles::DEFAULT_FPOFT_PROFILE, |name| {
                *profiles::ALL_FPOFT_PROFILES
                    .iter()
                    .find(|profile| profile.name == name)
                    .unwrap()
            });
    // The CLI choices come from the same catalog, so a supplied name always resolves.
    let mut topology_profile =
        options
            .topology_profile
            .as_deref()
            .map_or(profiles::DEFAULT_PROFILE, |name| {
                *profiles::ALL_PROFILES
                    .iter()
                    .find(|profile| profile.name == name)
                    .unwrap()
            });
    if mode == Mode::Fpoft {
        topology_profile = fpoft_profile.topology_profile;
    }
    let hops = options.hops.unwrap_or_else(|| {
        if let Some(profile) = download_profile {
            profile.hops
        } else if matches!(mode, Mode::FixedTopology | Mode::Fpoft) {
            topology_profile.layers.len()
        } else {
            DEFAULT_PATH_HOPS
        }
    });
    assert!(
        matches!(model, Model::HiddenService)
            == matches!(mode, Mode::FixedPath | Mode::FixedTopology | Mode::Fpoft),
        "hidden-service requires fixed-path, fixed-topology, or fpoft; these support only hidden-service"
    );
    assert!(hops > 0, "--hops must be greater than zero");
    if matches!(mode, Mode::FixedTopology | Mode::Fpoft) {
        let required_hops = topology_profile.layers.len();
        if hops != required_hops {
            Options::command()
                .error(
                    ErrorKind::ArgumentConflict,
                    format!(
                        "profile {} has {} layers; set --hops {} or omit it",
                        if mode == Mode::Fpoft {
                            fpoft_profile.name
                        } else {
                            topology_profile.name
                        },
                        topology_profile.layers.len(),
                        required_hops
                    ),
                )
                .exit();
        }
        topology_profile
            .connections
            .validate_layers(topology_profile.layers);
    }

    assert!(
        options.csv_interval > 0,
        "--csv-interval must be greater than zero"
    );
    let fixed_hops = download_profile.map_or_else(
        || {
            options
                .fixed_hops
                .unwrap_or(if mode == Mode::KOverW { hops } else { 0 })
        },
        |profile| profile.fixed_hops,
    );
    if mode == Mode::KHopsFixed {
        assert!(
            matches!(model, Model::Simple | Model::DownloadSession),
            "K-HF mode currently supports only the simple and download-session models"
        );
        assert!(fixed_hops <= hops, "--fixed-hops must not exceed --hops");
    }
    let k = download_profile.map_or_else(|| options.k.unwrap_or(0), |profile| profile.k);
    if mode == Mode::KOverW {
        assert!(
            matches!(model, Model::Simple | Model::DownloadSession),
            "K/W mode supports only the simple and download-session models"
        );
        assert!(k > 0, "--k must be greater than zero");
        assert!(fixed_hops <= hops, "--fixed-hops must not exceed --hops");
    }
    let alpha = options.alpha.unwrap_or(0.0);
    if mode == Mode::AlphaSticky {
        assert!(
            matches!(model, Model::Simple | Model::DownloadSession),
            "alpha-sticky mode supports only the simple and download-session models"
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
        Mode::FixedTopology => topology_profile.name,
        Mode::Fpoft => fpoft_profile.name,
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
        (Model::HiddenService, _) => adversary.name(),
        _ => "SybilAdversary",
    };
    let sdlm_strategy = match (model, mode) {
        (Model::DownloadSession, Mode::Random) => Some(SdlmStrategy::Random),
        (Model::DownloadSession, Mode::KHopsFixed) => Some(SdlmStrategy::KHopsFixed { fixed_hops }),
        (Model::DownloadSession, Mode::KOverW) => Some(SdlmStrategy::KOverW { k, fixed_hops }),
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
            session_path_count(file_size, packet_size, hops),
            hops,
            total_nodes,
            malicious_nodes,
        )
    });
    let mut parameters = Vec::new();
    if let Some(profile) = download_profile {
        parameters.push(("download_profile", profile.name.to_owned()));
    }
    match mode {
        Mode::KHopsFixed => parameters.push(("fixed_hops", fixed_hops.to_string())),
        Mode::KOverW => {
            parameters.extend([("k", k.to_string()), ("fixed_hops", fixed_hops.to_string())])
        }
        Mode::AlphaSticky => parameters.push(("alpha", alpha.to_string())),
        Mode::FixedPath => {
            use time_based_path_sampler::fixed_path::{MAX_LIFETIME_SECONDS, MIN_LIFETIME_SECONDS};
            parameters.extend([
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
        Mode::FixedTopology | Mode::Fpoft => {
            use time_based_path_sampler::fixed_topology::ConnectionMode;
            let connections = match topology_profile.connections {
                ConnectionMode::Mesh => "mesh".to_owned(),
                ConnectionMode::Degree(d) => format!("degree({d})"),
            };
            // Layer order is service to exit; R marks a pool whose nodes rotate.
            let layers = topology_profile
                .layers
                .iter()
                .map(|layer| {
                    if layer.lifetime.bounds().is_some() {
                        format!("R{}", layer.node_count)
                    } else {
                        layer.node_count.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("_");
            parameters.extend([
                ("topology_connections", connections),
                ("topology_profile", topology_profile.name.to_owned()),
                ("topology_layers", layers),
            ]);
        }
        Mode::Random => {}
    }
    if mode == Mode::Fpoft {
        let lifetime = fpoft_profile.path_lifetime.bounds();
        parameters.extend([
            (
                "path_lifetime_min_seconds",
                lifetime.map_or_else(|| "never".to_owned(), |(min, _)| min.to_string()),
            ),
            (
                "path_lifetime_max_seconds",
                lifetime.map_or_else(|| "never".to_owned(), |(_, max)| max.to_string()),
            ),
            (
                "path_lifetime_distribution",
                if lifetime.is_some() {
                    "max_of_two_uniform_draws"
                } else {
                    "never"
                }
                .to_owned(),
            ),
        ]);
    }
    match model {
        Model::HiddenService => {
            let configured = adversary.create();
            parameters.push((
                "compromise_attempt_budget_per_layer",
                configured.walker().budget().to_string(),
            ));
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
        path_hops: hops,
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
                        RandomPathSampler::new(hops),
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
                        KHopsFixedPathSampler::new(hops, fixed_hops),
                        SybilAdversary,
                        simulator.limit_sec(),
                    ))
                })
                .collect();
            simulator.simulate(models);
        }
        (Model::Simple, Mode::KOverW) => {
            let models = (0..options.users)
                .map(|_| {
                    UserModelIterator(SimpleModel::new(
                        &mixnet,
                        KOverWPathSampler::new_with_fixed_hops(hops, k, fixed_hops),
                        SybilAdversary,
                        simulator.limit_sec(),
                    ))
                })
                .collect();
            simulator.simulate(models);
        }
        (Model::Simple, Mode::AlphaSticky) => {
            let models = (0..options.users)
                .map(|_| {
                    UserModelIterator(SimpleModel::new(
                        &mixnet,
                        AlphaStickyPathSampler::new(hops, alpha),
                        SybilAdversary,
                        simulator.limit_sec(),
                    ))
                })
                .collect();
            simulator.simulate(models);
        }
        (Model::HiddenService, Mode::FixedPath) => simulate_hidden_services(
            &mut simulator,
            &mixnet,
            options.users,
            || FixedPathSampler::new(hops, &mixnet),
            || adversary.create(),
        ),
        (Model::HiddenService, Mode::FixedTopology) => simulate_hidden_services(
            &mut simulator,
            &mixnet,
            options.users,
            || FixedTopologySampler::new(topology_profile, &mixnet),
            || adversary.create(),
        ),
        (Model::HiddenService, Mode::Fpoft) => simulate_hidden_services(
            &mut simulator,
            &mixnet,
            options.users,
            || FPOFTSampler::new(fpoft_profile, &mixnet),
            || adversary.create(),
        ),
        (Model::HiddenService, _) | (_, Mode::FixedPath | Mode::FixedTopology | Mode::Fpoft) => {
            unreachable!("time-based samplers require hidden-service")
        }
        (Model::DownloadSession, Mode::Random) => {
            let models = (0..options.users)
                .map(|_| {
                    UserModelIterator(DownloadSessionModel::new(
                        &mixnet,
                        RandomPathSampler::new(hops),
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
                        KHopsFixedPathSampler::new(hops, fixed_hops),
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
                        KOverWPathSampler::new_with_fixed_hops(hops, k, fixed_hops),
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
                        AlphaStickyPathSampler::new(hops, alpha),
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
fn simulate_hidden_services<S: TimeBasedPathSampler + Send, A: Adversary + Send>(
    simulator: &mut Simulator,
    mixnet: &Mixnet,
    users: u32,
    make_sampler: impl Fn() -> S,
    make_adversary: impl Fn() -> A,
) where
    S::Event: Send,
{
    let models = (0..users)
        .map(|_| {
            UserModelIterator(HiddenServiceModel::new(
                mixnet,
                make_sampler(),
                make_adversary(),
                simulator.limit_sec(),
            ))
        })
        .collect();
    simulator.simulate(models);
}
