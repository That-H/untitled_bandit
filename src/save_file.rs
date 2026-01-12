use super::puzzle_loader::{pzl_save, read_lines, LoadErr};
use pzl_save::get_save_path;
use std::{fs, io::{self, Write}};

const SCORE_FILE: &str = "high_score.txt";

/// Get the high score from the save file.
pub fn load_highscore() -> Result<f64, LoadErr> {
    for line in read_lines(get_save_path().join(SCORE_FILE)).map_err(|e| match e.kind() {
        io::ErrorKind::NotFound => LoadErr::NotFound,
        io::ErrorKind::ResourceBusy => LoadErr::Cant(String::from("the file is already in use")),
        e => LoadErr::Other(e),
    })?.map_while(Result::ok) {
        match line.parse() {
            Ok(sc) => return Ok(sc),
            Err(_) => return Err(LoadErr::IncorrectFormat(String::from("the score is not a number"))),
        }
    }

    // No score saved yet.
    return Ok(0.0)
}

/// Save the high score to the file.
pub fn save_highscore(new_score: f64) {
    let save = get_save_path();

    fs::create_dir_all(&save).expect("Couldn't create directories");
    let mut file = io::BufWriter::new(fs::File::create(save.join(SCORE_FILE)).expect("Unable to write save file"));

    file.write(new_score.to_string().as_bytes()).expect("Unable to write high score");
}
