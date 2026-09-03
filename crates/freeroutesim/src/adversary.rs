use crate::topologygen::MixNode;

/// Defines when an adversary wins on a sampled path.
pub trait Adversary {
    fn wins(&mut self, path: &[MixNode]) -> bool;
}

/// Wins when every node on a non-empty path is malicious.
#[derive(Debug, Default)]
pub struct SybilAdversary;

impl Adversary for SybilAdversary {
    fn wins(&mut self, path: &[MixNode]) -> bool {
        !path.is_empty() && path.iter().all(|node| node.is_malicious)
    }
}

#[cfg(test)]
mod tests {
    use super::{Adversary, SybilAdversary};
    use crate::topologygen::MixNode;

    fn node(mix_id: u32, is_malicious: bool) -> MixNode {
        MixNode {
            weight: 1.0,
            mix_id,
            is_malicious,
            tags: Vec::new(),
        }
    }

    #[test]
    fn wins_only_when_every_node_is_malicious() {
        let mut adversary = SybilAdversary;

        assert!(adversary.wins(&[node(1, true), node(2, true)]));
        assert!(!adversary.wins(&[node(1, true), node(2, false)]));
        assert!(!adversary.wins(&[]));
    }
}
