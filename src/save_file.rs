use super::puzzle_loader::{LoadErr, pzl_save, read_lines};
use pzl_save::get_save_path;
use std::{
    fs,
    io::{self, Write},
    collections::HashMap,
};

const SCORE_FILE: &str = "high_score.txt";
const KILLS_FILE: &str = "kills.txt";
const WON_YET_FILE: &str = "won_yet.txt";

/// Get the high score from the save file.
pub fn load_highscore() -> Result<f64, LoadErr> {
    if let Some(ln) = read_lines(get_save_path().join(SCORE_FILE))
        .map_err(|e| match e.kind() {
            io::ErrorKind::NotFound => LoadErr::NotFound,
            io::ErrorKind::ResourceBusy => {
                LoadErr::Cant(String::from("the file is already in use"))
            }
            e => LoadErr::Other(e),
        })?
        .map_while(Result::ok)
        .next()
    {
        match ln.parse() {
            Ok(sc) => return Ok(sc),
            Err(_) => {
                return Err(LoadErr::IncorrectFormat(String::from(
                    "the score is not a number",
                )));
            }
        }
    }

    // No score saved yet.
    Ok(0.0)
}

/// Save the high score to the file.
pub fn save_highscore(new_score: f64) {
    let save = get_save_path();

    fs::create_dir_all(&save).expect("Couldn't create directories");
    let mut file = io::BufWriter::new(
        fs::File::create(save.join(SCORE_FILE)).expect("Unable to write save file"),
    );

    file.write_all(new_score.to_string().as_bytes())
        .expect("Unable to write high score");
}

/// Get whether we have won yet from the save file.
pub fn load_won() -> Result<bool, LoadErr> {
    if let Some(ln) = read_lines(get_save_path().join(WON_YET_FILE))
        .map_err(|e| match e.kind() {
            io::ErrorKind::NotFound => LoadErr::NotFound,
            io::ErrorKind::ResourceBusy => {
                LoadErr::Cant(String::from("the file is already in use"))
            }
            e => LoadErr::Other(e),
        })?
        .map_while(Result::ok)
        .next()
    {
        return Ok(match &ln as &str {
            "yes" => true,
            "no" => false,
            _ => false,
        });
    }

    // No data saved yet.
    Ok(false)
}

/// Save the high score to the file.
pub fn save_won(new_status: bool) {
    let save = get_save_path();

    fs::create_dir_all(&save).expect("Couldn't create directories");
    let mut file = io::BufWriter::new(
        fs::File::create(save.join(WON_YET_FILE)).expect("Unable to write save file"),
    );

    let txt = if new_status { "yes" } else { "no" };
    file.write_all(txt.as_bytes())
        .expect("Unable to write won status");
}
/// Get the kill counts from the file.
pub fn load_kills() -> Result<HashMap<char, u32>, LoadErr> {
    let mut kills = HashMap::new();
    
    for ln in read_lines(get_save_path().join(KILLS_FILE))
        .map_err(|e| match e.kind() {
            io::ErrorKind::NotFound => LoadErr::NotFound,
            io::ErrorKind::ResourceBusy => {
                LoadErr::Cant(String::from("the file is already in use"))
            }
            e => LoadErr::Other(e),
        })?
        .map_while(Result::ok)
    {
        let mut this_char = None;
        for (n, val) in ln.split(':').enumerate() {
            // Place character in to map.
            if n == 0 {
                this_char = Some(val.chars().next().ok_or(LoadErr::IncorrectFormat(String::from("No character")))?);
            // Read number of kills.
            } else {
                let Some(ch) = this_char else { unreachable!() };
                kills.insert(ch, val.parse().map_err(|_| LoadErr::IncorrectFormat(format!("No value for '{ch}'")))?);
            }
        }
    }

    Ok(kills)
}

/// Save the kill counts to the file.
pub fn save_kills(kills: &HashMap<char, u32>) {
    let save = get_save_path();

    fs::create_dir_all(&save).expect("Couldn't create directories");
    let mut file = io::BufWriter::new(
        fs::File::create(save.join(KILLS_FILE)).expect("Unable to write save file"),
    );

    for (ch, count) in kills {
        file.write(format!("{ch}:{count}\n").as_bytes())
            .expect("Unable to write kill count");
    }

    file.flush().expect("Couldn't write to kills file");
}

