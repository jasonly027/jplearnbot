use std::{
    env,
    error::Error,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::Path,
};

use reqwest::blocking as req;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct DictEntry {
    #[serde(rename = "ent_seq")]
    id: u32,

    #[serde(rename = "keb")]
    k_ele: Option<KanjiElement>,
}

#[derive(Debug, Deserialize)]
struct KanjiElement {
    #[serde(rename = "keb")]
    kanji: String,
}

pub fn get_dict(overwrite: bool) -> Result<(), Box<dyn Error>> {
    let reader = get_dict_reader(overwrite)?;

    let mut entries: Vec<DictEntry> = Vec::new();
    for line in reader.lines() {
        let line = line?;
        entries.push(serde_json::from_str(&line)?);
    }

    Ok(())
}

fn get_dict_reader(overwrite: bool) -> Result<BufReader<File>, Box<dyn Error>> {
    let jmdict_path = env::current_exe()?
        .parent()
        .ok_or("Executable's directory couldn't be found")?
        .join("jmdict.jsonl");

    // Download if missing or explicitly requested
    if !jmdict_path.exists() || overwrite {
        download_dict(&jmdict_path)?;
    }

    Ok(BufReader::new(fs::File::open(jmdict_path)?))
}

fn download_dict(destination: &Path) -> Result<(), Box<dyn Error>> {
    const URL: &str = "https://gitlab.com/jgrind/jmdict/-/raw/main/jmdict.jsonl?ref_type=heads";
    let content = req::get(URL)?.bytes()?;
    fs::write(destination, content)?;
    Ok(())
}
