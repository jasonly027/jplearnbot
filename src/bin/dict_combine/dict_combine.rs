use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    path::Path,
    process,
};

use crate::{dict, jlpt};

pub fn run(dir: &Path, overwrite: bool, no_cache: bool) {
    let dict = dict::get_dict(&dir.join("jmdict.jsonl"), no_cache);
    let entries = jlpt::get_entries(dir);
    let writer = get_writer(dir, overwrite);

    for entry in &entries {

    }
}

fn get_writer(dir: &Path, overwrite: bool) -> BufWriter<File> {
    let file = OpenOptions::new()
        .write(true)
        .create_new(!overwrite)
        .open(dir.join("dictionary.jsonl"))
        .unwrap_or_else(|e| {
            eprintln!("Error writing output:\n\t{e}");
            process::exit(-1);
        });

    BufWriter::new(file)
}
