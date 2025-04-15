use std::{
    collections::HashMap,
    error::Error,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::Path,
    process,
    rc::Rc,
};

use jplearnbot::dictionary::Entry;
use reqwest::blocking as req;

pub fn get_dict(file: &Path, no_cache: bool) -> HashMap<String, Rc<Entry>> {
    let entries: Vec<Rc<Entry>> = get_entries(file, no_cache)
        .into_iter()
        .map(Rc::new)
        .collect();

    let mut map: HashMap<String, Rc<Entry>> = HashMap::new();
    for entry in entries {
        for reading in &entry.readings {
            map.insert(reading.hiragana.clone(), entry.clone());
        }
    }

    map
}

fn get_entries(file: &Path, no_cache: bool) -> Vec<Entry> {
    let reader = get_dfile(file, no_cache).unwrap_or_else(|e| {
        eprintln!("Failed to open dfile: {e}");
        process::exit(-1);
    });

    let mut entries: Vec<Entry> = Vec::new();
    for line in reader.lines() {
        let line = line.unwrap_or_else(|e| {
            eprintln!("Invalid byte read in dfile: {e}");
            process::exit(-1);
        });

        let entry = serde_json::from_str(&line).unwrap_or_else(|e| {
            eprintln!("JSON Parse error: {e}");
            process::exit(-1);
        });

        entries.push(entry);
    }

    entries
}

fn get_dfile(file: &Path, no_cache: bool) -> Result<BufReader<File>, Box<dyn Error>> {
    // Download if missing or explicitly requested
    if !file.exists() || no_cache {
        download_dict(file)?;
    }

    Ok(BufReader::new(File::open(file)?))
}

fn download_dict(file: &Path) -> Result<(), Box<dyn Error>> {
    const URL: &str = "https://gitlab.com/jgrind/jmdict/-/raw/main/jmdict.jsonl?ref_type=heads";
    let content = req::get(URL)?.bytes()?;
    fs::write(file, content)?;
    Ok(())
}
