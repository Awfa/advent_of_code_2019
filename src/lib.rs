#![deny(clippy::all)]

pub mod day1;

pub mod day2;
pub mod day5;
pub mod day7;
pub mod intcode;

use std::fs::File;
use std::io::BufRead;

pub fn get_intcode_memory_from_file(path: &str) -> Vec<i64> {
    let input_file = File::open(path).unwrap();
    let reader = std::io::BufReader::new(input_file);
    reader
        .split(b',')
        .map(|s| std::str::from_utf8(&s.unwrap()).unwrap().trim().parse())
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
}
