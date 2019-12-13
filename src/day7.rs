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

    let mut highest_output_settings = None;
    let initial_input = 0;
    let mut permutator = Permutator::new((0..=4).collect());
    while let Some(x) = permutator.next() {
        let (phase_a, phase_b, phase_c, phase_d, phase_e) = (x[0], x[1], x[2], x[3], x[4]);
        let emulator_a = Emulator::new(&initial_memory, std::iter::once(Ok(phase_a)).chain(std::iter::once(Ok(initial_input))));
        let emulator_b = Emulator::new(&initial_memory, std::iter::once(Ok(phase_b)).chain(emulator_a.into_output_iter()));
        let emulator_c = Emulator::new(&initial_memory, std::iter::once(Ok(phase_c)).chain(emulator_b.into_output_iter()));
        let emulator_d = Emulator::new(&initial_memory, std::iter::once(Ok(phase_d)).chain(emulator_c.into_output_iter()));
        let emulator_e = Emulator::new(&initial_memory, std::iter::once(Ok(phase_e)).chain(emulator_d.into_output_iter()));

        let thrust_output = emulator_e.into_output_iter().next().unwrap().unwrap();
        let candidate_output_settings = Some(((phase_a, phase_b, phase_c, phase_d, phase_e), thrust_output));
        highest_output_settings = highest_output_settings.map_or(candidate_output_settings, |current| {
            let (_, prev_thrust_output) = current;
            if prev_thrust_output > thrust_output {
                Some(current)
            } else {
                candidate_output_settings
            }
        });
    }
    println!("{:?}", highest_output_settings);
    highest_output_settings.map(|(_, highest_output)| highest_output).unwrap()
}

struct Permutator {
    array: Vec<EmulatorMemoryType>,
    recursion_stack: Vec<(usize, usize, bool)>
}

impl Permutator {
    fn new(array: Vec<EmulatorMemoryType>) -> Permutator {
        Permutator {
            array,
            recursion_stack: vec![(0, 0, false)]
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
                        self.recursion_stack.push((start+1, start+1, false));
                        continue;
                    } else {
                        self.array.swap(start, swap_index);
                        self.recursion_stack.push((start, swap_index+1, false));
                        continue;
                    }
                }
            } else {
                return None;
            }
        }
    }
}
