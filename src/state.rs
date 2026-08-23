use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use crate::types::Peer;

#[derive(Default)]
pub struct PeerTable {
    inner: Mutex<HashMap<String, Peer>>,
    paired: Mutex<HashSet<String>>,
}

impl PeerTable {
    pub fn upsert(&self, peer: Peer) {
        self.inner.lock().unwrap().insert(peer.id.clone(), peer);
    }

    pub fn get(&self, id: &str) -> Option<Peer> {
        self.inner.lock().unwrap().get(id).cloned()
    }

    pub fn snapshot(&self) -> Vec<Peer> {
        self.inner.lock().unwrap().values().cloned().collect()
    }

    pub fn mark_paired(&self, id: &str) {
        self.paired.lock().unwrap().insert(id.to_string());
    }

    pub fn is_paired(&self, id: &str) -> bool {
        self.paired.lock().unwrap().contains(id)
    }

    /// Discovered peers that are also paired.
    pub fn paired_peers(&self) -> Vec<Peer> {
        let paired = self.paired.lock().unwrap();
        self.inner
            .lock()
            .unwrap()
            .values()
            .filter(|p| paired.contains(&p.id))
            .cloned()
            .collect()
    }
}
