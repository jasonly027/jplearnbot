use std::{
    collections::HashMap,
    error::Error,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::Path,
    rc::Rc,
};

use jplearnbot::{dictionary::Entry, open_reader};
use reqwest::blocking as req;

pub fn dict(file: &Path, no_cache: bool) -> HashMap<String, Vec<Rc<Entry>>> {
    let entries: Vec<Rc<Entry>> = entries(file, no_cache)
        .into_iter()
        .map(Rc::new)
        .collect();

    let mut map: HashMap<String, Vec<Rc<Entry>>> = HashMap::new();
    for entry in entries {
        for reading in &entry.readings {
            map.entry(reading.hiragana.clone())
                .or_default()
                .push(entry.clone());
        }
    }

    map
}

fn entries(file: &Path, no_cache: bool) -> Vec<Entry> {
    let reader = dfile(file, no_cache);

    let mut entries: Vec<Entry> = Vec::new();
    for line in reader.lines() {
        let line = line.unwrap_or_else(|e| panic!("Invalid byte read in dfile:\n{e}"));

        let entry =
            serde_json::from_str(&line).unwrap_or_else(|e| panic!("JSON Parse error:\n{e}"));

        entries.push(entry);
    }

    entries
}

fn dfile(path: &Path, no_cache: bool) -> BufReader<File> {
    // Download if missing or explicitly requested
    if !path.exists() || no_cache {
        download_dict(path)
            .unwrap_or_else(|e| panic!("Failed to download dfile to {}\n{e}", path.display()));
    }

    open_reader(path)
}

fn download_dict(destination: &Path) -> Result<(), Box<dyn Error>> {
    const URL: &str = "https://gitlab.com/jgrind/jmdict/-/raw/main/jmdict.jsonl?ref_type=heads";
    let content = req::get(URL)?.bytes()?;
    fs::write(destination, content)?;
    Ok(())
}
