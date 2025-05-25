use std::{
    io::{BufRead, Cursor},
    sync::Arc,
};

use jplearnbot::dictionary::{DictEntry, NLevel, Pos};
use rand::seq::SliceRandom;

/// Contains [`DictEntry`]'s.
pub struct Dictionary {
    /// Contains all of the entries.
    entries: Vec<Arc<DictEntry>>,
}

impl Default for Dictionary {
    fn default() -> Self {
        let mut dict = Dictionary {
            entries: Vec::new(),
        };

        static DICT_FILE: &[u8] = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/content/dictionary.jsonl"
        ));

        // Deserialize entries and append
        for line in Cursor::new(DICT_FILE).lines() {
            let entry: DictEntry = serde_json::from_str(&line.expect("failed to read line"))
                .expect("failed to deserialize entry");

            dict.entries.push(entry.into());
        }

        dict
    }
}

impl Dictionary {
    pub fn new() -> Self {
        Dictionary::default()
    }

    /// Creates a randomized subset of the entries based on the parameter filters.
    pub async fn sample(&self, levels: &[NLevel], pos: &[Pos]) -> Vec<Arc<DictEntry>> {
        let mut sample = Vec::new();

        for entry in &self.entries {
            // Add only if at least one matching NLevel or part of speech.
            if entry.levels().iter().any(|lvl| levels.contains(lvl))
                && entry
                    .senses
                    .iter()
                    .any(|sense| sense.pos.iter().any(|p| pos.contains(p)))
            {
                sample.push(entry.clone());
            }
        }
        sample.shuffle(&mut rand::rng());

        sample
    }
}
