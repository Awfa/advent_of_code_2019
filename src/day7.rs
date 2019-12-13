use super::get_intcode_memory_from_file;
use super::intcode::*;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::iter::once;

pub fn run_part_1(path: &str) -> EmulatorMemoryType {
    let initial_memory = get_intcode_memory_from_file(path);

    let mut highest_thrust = None;
    let initial_input = 0;
    let mut permutator = Permutator::new((0..=4).collect());
    while let Some(x) = permutator.next() {
        let (phase_a, phase_b, phase_c, phase_d, phase_e) = (x[0], x[1], x[2], x[3], x[4]);
        let emulator_a = Emulator::new(
            &initial_memory,
            once(Ok(phase_a)).chain(once(Ok(initial_input))),
        );
        let emulator_b = Emulator::new(
            &initial_memory,
            once(Ok(phase_b)).chain(emulator_a.into_output_iter()),
        );
        let emulator_c = Emulator::new(
            &initial_memory,
            once(Ok(phase_c)).chain(emulator_b.into_output_iter()),
        );
        let emulator_d = Emulator::new(
            &initial_memory,
            once(Ok(phase_d)).chain(emulator_c.into_output_iter()),
        );
        let emulator_e = Emulator::new(
            &initial_memory,
            once(Ok(phase_e)).chain(emulator_d.into_output_iter()),
        );

        let thrust_output = emulator_e.into_output_iter().last().unwrap().unwrap();
        highest_thrust = highest_thrust.map_or(Some(thrust_output), |current| {
            Some(std::cmp::max(thrust_output, current))
        });
    }

    highest_thrust.unwrap()
}

pub fn run_part_2(path: &str) -> EmulatorMemoryType {
    let initial_memory = get_intcode_memory_from_file(path);

    let mut highest_thrust = None;
    let initial_input = 0;
    let mut permutator = Permutator::new((5..=9).collect());
    while let Some(x) = permutator.next() {
        let (phase_a, phase_b, phase_c, phase_d, phase_e) = (x[0], x[1], x[2], x[3], x[4]);

        let emulator_e_a_loopback_pipe =
            RefCell::new(VecDeque::<Result<EmulatorMemoryType, EmulatorError>>::new());
        let loopback_iter =
            std::iter::from_fn(|| emulator_e_a_loopback_pipe.borrow_mut().pop_front());
        let emulator_a_iter = once(Ok(phase_a))
            .chain(once(Ok(initial_input)))
            .chain(loopback_iter);
        let emulator_a = Emulator::new(&initial_memory, emulator_a_iter);
        let emulator_b = Emulator::new(
            &initial_memory,
            once(Ok(phase_b)).chain(emulator_a.into_output_iter()),
        );
        let emulator_c = Emulator::new(
            &initial_memory,
            once(Ok(phase_c)).chain(emulator_b.into_output_iter()),
        );
        let emulator_d = Emulator::new(
            &initial_memory,
            once(Ok(phase_d)).chain(emulator_c.into_output_iter()),
        );
        let emulator_e = Emulator::new(
            &initial_memory,
            once(Ok(phase_e)).chain(emulator_d.into_output_iter()),
        );
        let output_iterator = emulator_e.into_output_iter().map(|value| {
            emulator_e_a_loopback_pipe.borrow_mut().push_back(value);
            value
        });

        let thrust_output = output_iterator.last().unwrap().unwrap();
        highest_thrust = highest_thrust.map_or(Some(thrust_output), |current| {
            Some(std::cmp::max(thrust_output, current))
        })
    }

    highest_thrust.unwrap()
}

struct Permutator {
    array: Vec<EmulatorMemoryType>,
    recursion_stack: Vec<(usize, usize, bool)>,
}

impl Permutator {
    fn new(array: Vec<EmulatorMemoryType>) -> Permutator {
        Permutator {
            array,
            recursion_stack: vec![(0, 0, false)],
        }
    }

    fn next<'a>(&'a mut self) -> Option<&'a [EmulatorMemoryType]> {
        loop {
            if let Some((start, swap_index, explored)) = self.recursion_stack.pop() {
                // let tab: String = std::iter::repeat(" ").take(self.recursion_stack.len()).collect();
                // println!("{}start: {}, swap_index: {}, explored: {}", tab, start, swap_index, explored);
                if start + 1 >= self.array.len() {
                    return Some(self.array.as_slice());
                } else {
                    if swap_index >= self.array.len() {
                        continue;
                    } else if !explored {
                        self.array.swap(start, swap_index);
                        self.recursion_stack.push((start, swap_index, true));
                        self.recursion_stack.push((start + 1, start + 1, false));
                        continue;
                    } else {
                        self.array.swap(start, swap_index);
                        self.recursion_stack.push((start, swap_index + 1, false));
                        continue;
                    }
                }
            } else {
                return None;
            }
        }
    }
}
