//! Handles saving and loading puzzle saves.

use std::collections::HashMap;
use std::io::{self, ErrorKind, Write};
use std::fs;

const QUAL: &str = "";
const ORGANISATION: &str = "Uranium Productions";
const APP: &str = "Untitled Bandit";
const PZLS_FILE: &str = "completed_pzls.txt";

/// Load all puzzle saves.
pub fn load_pzl_save() -> HashMap<u128, u8> {
    let mut map = HashMap::new();
    let lines = match super::read_lines(get_pzl_path()) {
        Ok(lns) => lns,
        Err(e) => match e.kind() {
            // Must not have a save file yet, so no completion to read.
            ErrorKind::NotFound => return map,
            e => panic!("Error reading save file: {e:?}"),
        }
    };

    for line in lines.map_while(Result::ok) {
        let mut id = None;
        for (n, num) in line.split(":").enumerate() {
            if n == 0 {
                id = Some(num.parse().expect("Improper puzzle id"));
            } else if n == 1 {
                map.insert(id.take().unwrap(), num.parse().expect("Improper completion status"));
                id = None;
            } else {
                break;
            }
        }
    }

    map
}

/// Write the current state of completion to the save file.
pub fn write_pzl_save(data: HashMap<u128, u8>) {
    let mut p = get_pzl_path();
    p.pop();
    fs::create_dir_all(&p).expect("Can't create the directories");
    let mut file = io::BufWriter::new(fs::File::create(get_pzl_path()).expect("Unable to write save file"));

    for (hash, stars) in data {
        file.write(format!("{hash}:{stars}\n").as_bytes()).expect("Unable to write save file");
    }

    file.flush().expect("Unable to flush save file");
}

// Get the path to the puzzle save file.
fn get_pzl_path() -> std::path::PathBuf {
    let pro_dirs = directories::ProjectDirs::from(QUAL, ORGANISATION, APP).unwrap();

    pro_dirs.data_local_dir().join(PZLS_FILE)
}
