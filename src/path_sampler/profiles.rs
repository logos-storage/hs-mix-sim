//! download-session profiles from the session path-selection spec.
#[derive(Clone, Copy, Debug)]
pub struct DownloadProfile {
    pub name: &'static str,
    pub hops: usize,
    pub fixed_hops: usize,
    pub k: usize,
}

pub const DOWNLOAD_PROFILES: &[DownloadProfile] = &[
    DownloadProfile {
        name: "LITE",
        hops: 3,
        fixed_hops: 1,
        k: 5,
    },
    DownloadProfile {
        name: "STANDARD",
        hops: 3,
        fixed_hops: 2,
        k: 5,
    },
    DownloadProfile {
        name: "STRICT",
        hops: 4,
        fixed_hops: 3,
        k: 3,
    },
];
