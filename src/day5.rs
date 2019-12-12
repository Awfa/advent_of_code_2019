use super::intcode::*;
use std::fs::File;
use std::io::BufRead;

pub fn run_part_1(path: &str) -> EmulatorMemoryType {
    let input_file = File::open(path).unwrap();
    let reader = std::io::BufReader::new(input_file);
    let initial_memory = reader
        .split(b',')
        .map(|s| std::str::from_utf8(&s.unwrap()).unwrap().trim().parse())
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    let emulator = Emulator::new(&initial_memory, std::iter::once(1));
    let outputs = emulator.into_output_iter().collect::<Result<Vec<_>, _>>().unwrap();
    outputs[outputs.len()-1]
}