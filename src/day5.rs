use super::get_intcode_memory_from_file;
use super::intcode::*;
use std::iter::once;

pub fn run_part_1(path: &str) -> EmulatorMemoryType {
    let initial_memory = get_intcode_memory_from_file(path);

    let emulator = Emulator::new(&initial_memory, once(Ok(1)));
    let outputs = emulator
        .into_output_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    outputs[outputs.len() - 1]
}

pub fn run_part_2(path: &str) -> EmulatorMemoryType {
    let initial_memory = get_intcode_memory_from_file(path);

    let emulator = Emulator::new(&initial_memory, once(Ok(5)));
    let outputs = emulator
        .into_output_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    outputs[outputs.len() - 1]
}
