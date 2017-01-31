use std::collections::{HashMap, HashSet};
use serde_json;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct History {
    handled: HashMap<String, HashSet<String>>,
}

impl History {
    #[inline]
    pub fn new() -> History {
        History { handled: HashMap::new() }
    }

    pub fn from_json(json: &str) -> serde_json::Result<History> {
        serde_json::from_str(json)
    }

    pub fn insert(&mut self, term: String, meaning: String) {
        self.handled.entry(term).or_insert(HashSet::new()).insert(meaning);
    }

    pub fn contains(&mut self, term: &str, meaning: &str) -> bool {
        self.handled.get(term).map(|m| m.contains(meaning)).unwrap_or(false)
    }
}
