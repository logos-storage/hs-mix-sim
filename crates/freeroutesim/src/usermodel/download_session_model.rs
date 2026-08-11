//! Single anonymous-download session model.
//!
//! Time and topology churn are intentionally not considered. Each user performs one
//! download through a single topology snapshot. Session traffic includes erasure-coded
//! data chunks, packets that supply the corresponding SURBs, and control overhead.

use crate::path_sampler::PathSampler;
use crate::topologygen::MixNode;
use crate::usermodel::{RouteEvent, UserModel, UserModelInfo};

// Mix kappa = 128 bits = 16 bytes.
const SPHINX_KAPPA_BYTES: u64 = 16;

// Mix alpha size in bytes.
const SPHINX_ALPHA_BYTES: u64 = 32;

// Mix spec recommends t = 6 kappa-sized blocks for the
// combined address-and-delay field, so 6 * 16 = 96 bytes.
const SPHINX_ADDRESS_WIDTH: u64 = 6;

// Transport layer overhead: 16-byte Sphinx integrity prefix + 24-byte
// metadata (16-byte session id + 4-byte message index + 2-byte K + 2-byte N) +
// 2-byte chunk index = 42 bytes.
const TRANSPORT_CHUNK_OVERHEAD_BYTES: u64 = 42;

// erasure N value
const ERASURE_REDUNDANCY_N: u64 = 3;

// erasure K value
const ERASURE_REDUNDANCY_K: u64 = 2;

// model assumption for requests, acknowledgements, session control,
// and SURB-management overhead traffic.
const CONTROL_OVERHEAD_PERCENT: u64 = 5;

pub struct DownloadSessionModel<'a, S: PathSampler> {
    model_info: UserModelInfo<'a>,
    paths_remaining: u64,
    path_sampler: S,
}

impl<'a, S: PathSampler> DownloadSessionModel<'a, S> {
    pub fn new(
        model_info: UserModelInfo<'a>,
        path_sampler: S,
        file_size: u64,
        packet_size: u64,
    ) -> Self {
        assert!(file_size > 0, "file size must be greater than zero");
        assert!(packet_size > 0, "packet size must be greater than zero");

        let paths_remaining = session_path_count(file_size, packet_size, path_sampler.hops());

        Self {
            model_info,
            paths_remaining,
            path_sampler,
        }
    }
}

pub(crate) fn session_path_count(file_size: u64, packet_size: u64, hops: usize) -> u64 {
    assert!(hops > 0, "path length must be greater than zero");

    // P(L) = packet_size - |SphinxHeader(L)|
    let payload_size = sphinx_payload_size(packet_size, hops);

    // R(L) = P(L) - H_T: file bytes carried by one transport data chunk.
    let raw_chunk_size = payload_size
        .checked_sub(TRANSPORT_CHUNK_OVERHEAD_BYTES)
        .expect("Sphinx payload is too small for the transport chunk overhead");
    assert!(
        raw_chunk_size > 0,
        "Sphinx payload must have space for transport chunk data"
    );

    // K(F, L) = ceil(F / R(L)): original chunks required for the file.
    let original_chunks = file_size.div_ceil(raw_chunk_size);

    // N_D(F, L, r) = ceil(r * K(F, L)): data packets after erasure redundancy.
    let data_packets = checked_ceil_ratio(
        original_chunks,
        ERASURE_REDUNDANCY_N,
        ERASURE_REDUNDANCY_K,
        "erasure-coded data packet count overflowed",
    );

    // S_SURB(L) = (t*kappa - 2) + |SphinxHeader(L)| + kappa.
    let surb_size = surb_size(hops);

    // C_SURB(L) = floor(R(L) / S_SURB(L)): SURBs carried by one forward packet.
    let surbs_per_packet = raw_chunk_size / surb_size;
    assert!(
        surbs_per_packet > 0,
        "transport packet cannot fit a single SURB"
    );

    // N_F = ceil(N_D / C_SURB(L)): forward packets needed to supply all SURBs.
    let forward_packets = data_packets.div_ceil(surbs_per_packet);

    // Count both provider-to-downloader data packets and downloader-to-provider
    // SURB-supply packets before applying the remaining control-traffic estimate.
    let packets_before_control_overhead = data_packets
        .checked_add(forward_packets)
        .expect("download session packet count overflowed");

    // N_session = ceil((1 + control_overhead / 100) * (N_D + N_F))
    //           = ceil((100 + control_overhead) * (N_D + N_F) / 100).
    checked_ceil_ratio(
        packets_before_control_overhead,
        100 + CONTROL_OVERHEAD_PERCENT,
        100,
        "download session control overhead overflowed",
    )
}

fn sphinx_payload_size(packet_size: u64, max_hops: usize) -> u64 {
    packet_size
        .checked_sub(sphinx_header_size(max_hops))
        .expect("Mix packet is too small for its Sphinx header")
}

fn sphinx_header_size(max_hops: usize) -> u64 {
    let max_hops = u64::try_from(max_hops).expect("path length does not fit in u64");

    // |beta(L)| = (L * (t + 1) + 1) * kappa.
    let beta_units = max_hops
        .checked_mul(SPHINX_ADDRESS_WIDTH + 1)
        .and_then(|units| units.checked_add(1))
        .expect("Sphinx header size overflowed");
    let beta_size = beta_units
        .checked_mul(SPHINX_KAPPA_BYTES)
        .expect("Sphinx header size overflowed");

    // |SphinxHeader(L)| = |alpha| + |beta(L)| + |gamma|, where |gamma| = kappa.
    SPHINX_ALPHA_BYTES
        .checked_add(beta_size)
        .and_then(|size| size.checked_add(SPHINX_KAPPA_BYTES))
        .expect("Sphinx header size overflowed")
}

fn surb_size(max_hops: usize) -> u64 {
    // |hop_0| = t * kappa - 2; the two delay bytes are not part of the address.
    let first_hop_address_size = SPHINX_ADDRESS_WIDTH
        .checked_mul(SPHINX_KAPPA_BYTES)
        .and_then(|size| size.checked_sub(2))
        .expect("SURB first-hop address size overflowed");

    // |SURB(L)| = |hop_0| + |SphinxHeader(L)| + |reply_key|,
    // where the reply key is kappa bytes.
    first_hop_address_size
        .checked_add(sphinx_header_size(max_hops))
        .and_then(|size| size.checked_add(SPHINX_KAPPA_BYTES))
        .expect("SURB size overflowed")
}

fn checked_ceil_ratio(value: u64, numerator: u64, denominator: u64, message: &str) -> u64 {
    assert!(
        denominator > 0,
        "ratio denominator must be greater than zero"
    );
    value
        .checked_mul(numerator)
        .unwrap_or_else(|| panic!("{message}"))
        .div_ceil(denominator)
}

impl<S: PathSampler> UserModel for DownloadSessionModel<'_, S> {
    fn fetch_next(&mut self) -> Option<RouteEvent> {
        if self.paths_remaining == 0 {
            return None;
        }

        let (topology_index, topology) = self
            .model_info
            .topology_at(0)
            .expect("download session needs one topology snapshot");
        let path = self.path_sampler.sample_path(topology_index, topology);
        self.paths_remaining -= 1;

        Some((0, Self::is_path_malicious(&path)))
    }

    fn is_path_malicious(path: &[MixNode]) -> bool {
        !path.is_empty() && path.iter().all(|node| node.is_malicious)
    }
}
