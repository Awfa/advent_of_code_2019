use super::get_intcode_memory_from_file;
use super::intcode::*;

pub fn run_part_1(path: &str) -> EmulatorMemoryType {
    let mut initial_memory = get_intcode_memory_from_file(path);

    initial_memory[1] = 12;
    initial_memory[2] = 2;
    let mut emulator = emulator_with_empty_input(&initial_memory);
    emulator.run_to_completion().unwrap()
}

pub fn run_part_2(path: &str) -> Option<EmulatorMemoryType> {
    let mut initial_memory = get_intcode_memory_from_file(path);

    for noun in 0..=99 {
        for verb in 0..=99 {
            initial_memory[1] = noun;
            initial_memory[2] = verb;
            let mut emulator = emulator_with_empty_input(&initial_memory);

            if emulator.run_to_completion().unwrap() == 19_690_720 {
                let answer = 100 * noun + verb;
                return Some(answer);
            }
        }
    }

    None
}
