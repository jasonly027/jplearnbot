use std::{io::BufRead, path::Path};

use jplearnbot::open_reader;

pub type JLPTEntry = (String, Option<String>);

pub fn get_entries(dir: &Path) -> Vec<JLPTEntry> {
    let mut entries: Vec<JLPTEntry> = Vec::new();

    for file_no in 1..=4 {
        let path = dir.join(format!("jlpt-voc-{file_no}.utf.txt"));
        let reader = open_reader(&path);

        for line in reader.lines() {
            let line = line.unwrap_or_else(|e| panic!("Invalid byte read in jfile:\n{e}"));

            let Some(entry) = extract_entry(&line) else {
                continue;
            };

            entries.push(entry);
        }
    }

    entries
}

fn extract_entry(line: &str) -> Option<JLPTEntry> {
    if line.starts_with("#") || line.is_empty() || line.contains("~") {
        return None;
    }

    // Remove parenthesized note
    let no_note = line.split_once("（").map_or(line, |(left, _)| left);
    let fields: Vec<&str> = no_note.split_whitespace().collect();

    match fields.len() {
        // Kanji isn't present, hiragana is first in line
        1 => Some((fields[0].to_string(), None)),
        // Kanji is present, hiragana is second in line
        2 => Some((fields[1].to_string(), Some(fields[0].to_string()))),
        _ => panic!("Error extracting hiragana:\n\t{line}"),
    }
}

// cat
