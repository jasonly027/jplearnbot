use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    path::Path,
    process,
    rc::Rc,
};

use jplearnbot::dictionary::Entry;

use crate::{dict, jlpt};

pub fn run(dir: &Path, overwrite: bool, no_cache: bool) {
    let dict = dict::dict(&dir.join("jmdict.jsonl"), no_cache);
    let entries = jlpt::entries(dir);

    let mut writer = writer(dir, overwrite);

    let mut write_entry = |entry: &Entry| {
        let mut str = serde_json::to_string(entry).unwrap();
        str.push('\n');

        writer
            .write_all(str.as_bytes())
            .unwrap_or_else(|e| panic!("Failed to write to output:\n{e}"));
    };

    for (hiragana, kanji) in &entries {
        let Some(matches) = dict.get(hiragana) else {
            continue;
        };

        // No definition ambiguity, just write the exact match
        if matches.len() == 1 {
            write_entry(&matches[0]);
            continue;
        }

        // Multiple definitions match the entry, try
        // narrowing to one by cross referencing kanji
        if let Some(kanji) = kanji {
            // Entry has kanji, write the match that is the only one
            // with the same kanji, if it exists

            let matches: Vec<&Rc<Entry>> = matches
                .iter()
                .filter(|m| m.kanjis.iter().any(|k| *kanji == k.kanji))
                .collect();

            if matches.len() == 1 {
                write_entry(matches[0]);
            }
        } else {
            // Entry has no kanji, write the match that is the only one
            // without kanji too, if it exists

            let matches: Vec<&Rc<Entry>> = matches.iter().filter(|m| m.kanjis.is_empty()).collect();

            if matches.len() == 1 {
                write_entry(matches[0]);
            }
        }
    }

    writer.flush().expect("Failed to flush to output");
}

fn writer(dir: &Path, overwrite: bool) -> BufWriter<File> {
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(overwrite)
        .open(dir.join("dictionary.jsonl"))
        .unwrap_or_else(|e| {
            eprintln!("Error writing output:\n\t{e}");
            process::exit(-1);
        });

    BufWriter::new(file)
}
