use std::{
    fs::File,
    io::{self, BufRead, BufReader},
    path::Path,
    process,
};

pub fn get_entries(dir: &Path) -> Vec<String> {
    let mut entries: Vec<String> = Vec::new();

    for file_no in 1..=4 {
        let reader = get_jfile(dir, file_no).unwrap_or_else(|e| {
            eprintln!("Error opening jfile: {e}");
            process::exit(-1);
        });

        for line in reader.lines() {
            let line = line.unwrap_or_else(|e| {
                eprintln!("Invalid byte read in jfile: {e}");
                process::exit(-1);
            });

            let Some(hiragana) = extract_hiragana(&line) else {
                continue;
            };

            entries.push(hiragana);
        }
    }

    entries
}

fn extract_hiragana(line: &str) -> Option<String> {
    if line.starts_with("#") || line.is_empty() || line.contains("~") {
        return None;
    }

    // Remove parenthesized note
    let no_comment = line.split_once("（").map_or(line, |(before, _)| before);
    let fields: Vec<&str> = no_comment.split_whitespace().collect();

    match fields.len() {
        // Kanji isn't present, hiragana is first in line
        1 => Some(fields[0].to_string()),
        // Kanji is present, hiragana is second in line
        2 => Some(fields[1].to_string()),
        _ => {
            eprintln!("Error extracting hiragana:\n\t{line}");
            process::exit(-1);
        }
    }
}

fn get_jfile(path: &Path, file_no: u8) -> Result<BufReader<File>, io::Error> {
    let jlpt_path = path.join(format!("jlpt-voc-{file_no}.utf.txt"));
    Ok(BufReader::new(File::open(&jlpt_path)?))
}

// cat
