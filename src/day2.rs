use super::intcode::*;
use std::fs::File;
use std::io::BufRead;

pub fn run_part_1(path: &str) -> usize {
    let input_file = File::open(path).unwrap();
    let reader = std::io::BufReader::new(input_file);
    let mut initial_memory = reader
        .split(b',')
        .map(|s| std::str::from_utf8(&s.unwrap()).unwrap().trim().parse())
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    initial_memory[1] = 12;
    initial_memory[2] = 2;
    let mut emulator = Emulator::new(&initial_memory);
    emulator.run_to_completion().unwrap()
}

pub fn run_part_2(path: &str) -> Option<usize> {
    let input_file = File::open(path).unwrap();
    let reader = std::io::BufReader::new(input_file);
    let mut initial_memory = reader
        .split(b',')
        .map(|s| std::str::from_utf8(&s.unwrap()).unwrap().trim().parse())
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    for noun in 0..=99 {
        for verb in 0..=99 {
            initial_memory[1] = noun;
            initial_memory[2] = verb;
            let mut emulator = Emulator::new(&initial_memory);

            if emulator.run_to_completion().unwrap() == 19690720 {
                let answer = 100 * noun + verb;
                return Some(answer);
            }
        }
    }

    None
}
