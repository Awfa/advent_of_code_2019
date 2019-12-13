use opcode_macro::make_op_code;
use std::ops::Index;

pub type EmulatorMemoryType = i64;

enum ParameterMode {
    Position,  // = Position(memory: Memory, parameter_value: ParameterValue) {},
    Immediate, // = Immediate(parameter_value: ParameterValue) {},
}

// 0 = Position for ReadOnly, Writable
// 1 = Immediate for ReadOnly
// 2 = Relative for ReadOnly

make_op_code!(OpCode {
    1 = Add(addend1: ReadOnly, addend2: ReadOnly, dest: Writable) {
        *dest = addend1 + addend2;
    },
    2 = Multiply(factor1: ReadOnly, factor2: ReadOnly, dest: Writable) {
        *dest = factor1 * factor2;
    },
    3 = Input(dest: Writable) [input_iter: Input] {
        *dest = input_iter.next().ok_or(EmulatorError::InputNonExistent)??;
    },
    4 = Output(value: ReadOnly) [Output] {
        value
    },
    5 = JumpIfTrue(value: ReadOnly, new_address: ReadOnly) [new_instruction_pointer: InstructionPointerOverride] {
        if value != 0 {
            *new_instruction_pointer = Some(new_address);
        }
    },
    6 = JumpIfFalse(value: ReadOnly, new_address: ReadOnly) [new_instruction_pointer: InstructionPointerOverride] {
        if value == 0 {
            *new_instruction_pointer = Some(new_address);
        }
    },
    7 = LessThan(left_side: ReadOnly, right_side: ReadOnly, dest: Writable) {
        *dest = match left_side < right_side {
            true => 1,
            false => 0
        };
    },
    8 = Equals(left_side: ReadOnly, right_side: ReadOnly, dest: Writable) {
        *dest = match left_side == right_side {
            true => 1,
            false => 0
        };
    },
    99 = End!
});

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EmulatorError {
    InvalidInstruction {
        value_found: EmulatorMemoryType,
        position: usize,
    },
    NotEnoughParametersForInstruction {
        instruction: EmulatorMemoryType,
        expected: usize,
        found: usize,
    },
    InvalidMemoryLocation {
        value_found: EmulatorMemoryType,
        position: usize,
    },
    InstructionPointerOutOfBounds {
        position: usize,
    },
    InvalidParameterMode {
        value_found: EmulatorMemoryType,
        position: usize,
    },
    UnexpectedParameterModeForWritable {
        value_found: EmulatorMemoryType,
        position: usize,
    },
    InputNonExistent,
}

impl std::fmt::Display for EmulatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            EmulatorError::InvalidInstruction {
                value_found,
                position,
            } => write!(
                f,
                "Invalid instruction {} referenced at {}",
                value_found, position
            ),
            EmulatorError::NotEnoughParametersForInstruction {
                instruction,
                expected,
                found,
            } => write!(
                f,
                "Not enough parameters for instruction: {:?}. Expected {}, but found {}",
                instruction, expected, found
            ),
            EmulatorError::InvalidMemoryLocation {
                value_found,
                position,
            } => write!(
                f,
                "Invalid memory location {} referenced at {}.",
                value_found, position
            ),
            EmulatorError::InstructionPointerOutOfBounds { position } => write!(
                f,
                "Location pointer is at {} which is out of bounds",
                position
            ),
            EmulatorError::InvalidParameterMode { value_found, position } => write!(
                f,
                "Invalid parameter mode {} referenced at {}",
                value_found, position
            ),
            EmulatorError::UnexpectedParameterModeForWritable { value_found, position } => write!(
                f,
                "Writable parameter at {} has invalid parameter mode {}. The parameter mode must be 0",
                position, value_found
            ),
            EmulatorError::InputNonExistent => write!(
                f,
                "Input non existent"
            ),
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum EmulatorResult {
    Success,
    SuccessWithValue(EmulatorMemoryType),
    Done,
}

pub struct Emulator<I: Iterator<Item = Result<EmulatorMemoryType, EmulatorError>>> {
    memory: Vec<EmulatorMemoryType>,
    instruction_pointer: usize,
    input_iter: I,
}

impl<I: Iterator<Item = Result<EmulatorMemoryType, EmulatorError>>> Emulator<I> {
    pub fn new(initial_memory: &[EmulatorMemoryType], input_iter: I) -> Emulator<I> {
        Emulator {
            memory: initial_memory.into(),
            instruction_pointer: 0,
            input_iter,
        }
    }

    pub fn run_to_completion(&mut self) -> Result<EmulatorMemoryType, EmulatorError> {
        while self.step()? != EmulatorResult::Done {}
        Ok(self.memory[0])
    }

    pub fn step(&mut self) -> Result<EmulatorResult, EmulatorError> {
        OpCode::run(
            &mut self.memory,
            self.instruction_pointer,
            &mut self.input_iter,
        )
        .map(|run_result| {
            let (next_instruction_offset, output) = run_result;
            match next_instruction_offset {
                None => {
                    return EmulatorResult::Done;
                }
                Some(next_instruction_pointer) => {
                    self.instruction_pointer = next_instruction_pointer;
                }
            }

            if let Some(output) = output {
                return EmulatorResult::SuccessWithValue(output);
            }

            EmulatorResult::Success
        })
    }

    pub fn into_output_iter(
        mut self,
    ) -> impl Iterator<Item = Result<EmulatorMemoryType, EmulatorError>> {
        std::iter::from_fn(move || {
            while match self.step() {
                Ok(EmulatorResult::Done) => false,
                Ok(EmulatorResult::Success) => true,
                Ok(EmulatorResult::SuccessWithValue(value)) => return Some(Ok(value)),
                Err(e) => return Some(Err(e)),
            } {}
            None
        })
    }
}

pub fn emulator_with_empty_input(
    initial_memory: &[EmulatorMemoryType],
) -> Emulator<impl Iterator<Item = Result<EmulatorMemoryType, EmulatorError>>> {
    Emulator::new(initial_memory, std::iter::empty())
}

impl<I: Iterator<Item = Result<EmulatorMemoryType, EmulatorError>>> Index<usize> for Emulator<I> {
    type Output = EmulatorMemoryType;

    fn index(&self, index: usize) -> &Self::Output {
        self.memory.index(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::iter::once;

    #[test]
    fn test_example() -> Result<(), EmulatorError> {
        let initial_address = [1, 9, 10, 3, 2, 3, 11, 0, 99, 30, 40, 50];
        let mut emulator = emulator_with_empty_input(&initial_address);
        assert_eq!(&initial_address, emulator.memory.as_slice());

        assert_eq!(EmulatorResult::Success, emulator.step()?);
        assert_eq!(
            &[1, 9, 10, 70, 2, 3, 11, 0, 99, 30, 40, 50],
            emulator.memory.as_slice()
        );
        assert_eq!(EmulatorResult::Success, emulator.step()?);
        assert_eq!(
            &[3500, 9, 10, 70, 2, 3, 11, 0, 99, 30, 40, 50],
            emulator.memory.as_slice()
        );
        assert_eq!(EmulatorResult::Done, emulator.step()?);
        assert_eq!(EmulatorResult::Done, emulator.step()?);

        Ok(())
    }

    #[test]
    fn test_add() -> Result<(), EmulatorError> {
        let initial_address = [1, 0, 0, 0, 99];
        let mut emulator = emulator_with_empty_input(&initial_address);
        assert_eq!(&initial_address, emulator.memory.as_slice());

        assert_eq!(EmulatorResult::Success, emulator.step()?);
        assert_eq!(&[2, 0, 0, 0, 99], emulator.memory.as_slice());
        assert_eq!(EmulatorResult::Done, emulator.step()?);
        assert_eq!(EmulatorResult::Done, emulator.step()?);

        Ok(())
    }

    #[test]
    fn test_multiply_1() -> Result<(), EmulatorError> {
        let initial_address = [1, 0, 0, 0, 99];
        let mut emulator = emulator_with_empty_input(&initial_address);
        assert_eq!(&initial_address, emulator.memory.as_slice());

        assert_eq!(EmulatorResult::Success, emulator.step()?);
        assert_eq!(&[2, 0, 0, 0, 99], emulator.memory.as_slice());
        assert_eq!(EmulatorResult::Done, emulator.step()?);
        assert_eq!(EmulatorResult::Done, emulator.step()?);

        Ok(())
    }

    #[test]
    fn test_multiply_2() -> Result<(), EmulatorError> {
        let initial_address = [2, 4, 4, 5, 99, 0];
        let mut emulator = emulator_with_empty_input(&initial_address);
        assert_eq!(&initial_address, emulator.memory.as_slice());

        assert_eq!(EmulatorResult::Success, emulator.step()?);
        assert_eq!(&[2, 4, 4, 5, 99, 9801], emulator.memory.as_slice());
        assert_eq!(EmulatorResult::Done, emulator.step()?);
        assert_eq!(EmulatorResult::Done, emulator.step()?);

        Ok(())
    }

    #[test]
    fn test_overriding_future_instructions() -> Result<(), EmulatorError> {
        let initial_address = [1, 1, 1, 4, 99, 5, 6, 0, 99];
        let mut emulator = emulator_with_empty_input(&initial_address);
        assert_eq!(&initial_address, emulator.memory.as_slice());

        assert_eq!(EmulatorResult::Success, emulator.step()?);
        assert_eq!(&[1, 1, 1, 4, 2, 5, 6, 0, 99], emulator.memory.as_slice());
        assert_eq!(EmulatorResult::Success, emulator.step()?);
        assert_eq!(&[30, 1, 1, 4, 2, 5, 6, 0, 99], emulator.memory.as_slice());
        assert_eq!(EmulatorResult::Done, emulator.step()?);
        assert_eq!(EmulatorResult::Done, emulator.step()?);

        Ok(())
    }

    #[test]
    fn test_parameter_modes() -> Result<(), EmulatorError> {
        let initial_address = [1002, 4, 3, 4, 33];
        let mut emulator = emulator_with_empty_input(&initial_address);
        assert_eq!(&initial_address, emulator.memory.as_slice());

        assert_eq!(EmulatorResult::Success, emulator.step()?);
        assert_eq!(&[1002, 4, 3, 4, 99], emulator.memory.as_slice());
        assert_eq!(EmulatorResult::Done, emulator.step()?);
        assert_eq!(EmulatorResult::Done, emulator.step()?);

        Ok(())
    }

    #[test]
    fn test_input_output() -> Result<(), EmulatorError> {
        let initial_address = [3, 0, 4, 0, 99];
        let mut emulator = Emulator::new(&initial_address, once(Ok(1337)));
        assert_eq!(&initial_address, emulator.memory.as_slice());

        assert_eq!(EmulatorResult::Success, emulator.step()?);
        assert_eq!(&[1337, 0, 4, 0, 99], emulator.memory.as_slice());

        assert_eq!(EmulatorResult::SuccessWithValue(1337), emulator.step()?);
        assert_eq!(&[1337, 0, 4, 0, 99], emulator.memory.as_slice());
        assert_eq!(EmulatorResult::Done, emulator.step()?);
        assert_eq!(EmulatorResult::Done, emulator.step()?);

        Ok(())
    }

    #[test]
    fn test_output_iterator() -> Result<(), EmulatorError> {
        let initial_address = [3, 0, 4, 0, 99];
        let emulator = Emulator::new(&initial_address, once(Ok(1337)));
        assert_eq!(&initial_address, emulator.memory.as_slice());

        let mut iterator = emulator.into_output_iter();
        assert_eq!(Some(Ok(1337)), iterator.next());
        assert_eq!(None, iterator.next());

        Ok(())
    }

    #[test]
    fn test_negatives() -> Result<(), EmulatorError> {
        let initial_address = [1101, 100, -1, 4, 0];
        let mut emulator = emulator_with_empty_input(&initial_address);
        assert_eq!(&initial_address, emulator.memory.as_slice());

        assert_eq!(EmulatorResult::Success, emulator.step()?);
        assert_eq!(&[1101, 100, -1, 4, 99], emulator.memory.as_slice());
        assert_eq!(EmulatorResult::Done, emulator.step()?);
        assert_eq!(EmulatorResult::Done, emulator.step()?);

        Ok(())
    }

    #[test]
    fn test_equals_with_position_mode() -> Result<(), EmulatorError> {
        let initial_address = [3, 9, 8, 9, 10, 9, 4, 9, 99, -1, 8];
        {
            let emulator = Emulator::new(&initial_address, once(Ok(7)));
            assert_eq!(
                0,
                emulator.into_output_iter().collect::<Result<Vec<_>, _>>()?[0]
            );
        }
        {
            let emulator = Emulator::new(&initial_address, once(Ok(8)));
            assert_eq!(
                1,
                emulator.into_output_iter().collect::<Result<Vec<_>, _>>()?[0]
            );
        }
        {
            let emulator = Emulator::new(&initial_address, once(Ok(9)));
            assert_eq!(
                0,
                emulator.into_output_iter().collect::<Result<Vec<_>, _>>()?[0]
            );
        }

        Ok(())
    }

    #[test]
    fn test_less_than_with_position_mode() -> Result<(), EmulatorError> {
        let initial_address = [3, 9, 7, 9, 10, 9, 4, 9, 99, -1, 8];
        {
            let emulator = Emulator::new(&initial_address, once(Ok(7)));
            assert_eq!(
                1,
                emulator.into_output_iter().collect::<Result<Vec<_>, _>>()?[0]
            );
        }
        {
            let emulator = Emulator::new(&initial_address, once(Ok(8)));
            assert_eq!(
                0,
                emulator.into_output_iter().collect::<Result<Vec<_>, _>>()?[0]
            );
        }
        {
            let emulator = Emulator::new(&initial_address, once(Ok(9)));
            assert_eq!(
                0,
                emulator.into_output_iter().collect::<Result<Vec<_>, _>>()?[0]
            );
        }

        Ok(())
    }

    #[test]
    fn test_equals_with_immediate_mode() -> Result<(), EmulatorError> {
        let initial_address = [3, 3, 1108, -1, 8, 3, 4, 3, 99];
        {
            let emulator = Emulator::new(&initial_address, once(Ok(7)));
            assert_eq!(
                0,
                emulator.into_output_iter().collect::<Result<Vec<_>, _>>()?[0]
            );
        }
        {
            let emulator = Emulator::new(&initial_address, once(Ok(8)));
            assert_eq!(
                1,
                emulator.into_output_iter().collect::<Result<Vec<_>, _>>()?[0]
            );
        }
        {
            let emulator = Emulator::new(&initial_address, once(Ok(9)));
            assert_eq!(
                0,
                emulator.into_output_iter().collect::<Result<Vec<_>, _>>()?[0]
            );
        }

        Ok(())
    }

    #[test]
    fn test_less_than_with_immediate_mode() -> Result<(), EmulatorError> {
        let initial_address = [3, 3, 1107, -1, 8, 3, 4, 3, 99];
        {
            let emulator = Emulator::new(&initial_address, once(Ok(7)));
            assert_eq!(
                1,
                emulator.into_output_iter().collect::<Result<Vec<_>, _>>()?[0]
            );
        }
        {
            let emulator = Emulator::new(&initial_address, once(Ok(8)));
            assert_eq!(
                0,
                emulator.into_output_iter().collect::<Result<Vec<_>, _>>()?[0]
            );
        }
        {
            let emulator = Emulator::new(&initial_address, once(Ok(9)));
            assert_eq!(
                0,
                emulator.into_output_iter().collect::<Result<Vec<_>, _>>()?[0]
            );
        }

        Ok(())
    }

    #[test]
    fn test_jumps_with_position_mode() -> Result<(), EmulatorError> {
        let initial_address = [3, 12, 6, 12, 15, 1, 13, 14, 13, 4, 13, 99, -1, 0, 1, 9];
        {
            let emulator = Emulator::new(&initial_address, once(Ok(-1)));
            assert_eq!(
                1,
                emulator.into_output_iter().collect::<Result<Vec<_>, _>>()?[0]
            );
        }
        {
            let emulator = Emulator::new(&initial_address, once(Ok(0)));
            assert_eq!(
                0,
                emulator.into_output_iter().collect::<Result<Vec<_>, _>>()?[0]
            );
        }
        {
            let emulator = Emulator::new(&initial_address, once(Ok(2)));
            assert_eq!(
                1,
                emulator.into_output_iter().collect::<Result<Vec<_>, _>>()?[0]
            );
        }

        Ok(())
    }

    #[test]
    fn test_jumps_with_immediate_mode() -> Result<(), EmulatorError> {
        let initial_address = [3, 3, 1105, -1, 9, 1101, 0, 0, 12, 4, 12, 99, 1];
        {
            let emulator = Emulator::new(&initial_address, once(Ok(-1)));
            assert_eq!(
                1,
                emulator.into_output_iter().collect::<Result<Vec<_>, _>>()?[0]
            );
        }
        {
            let emulator = Emulator::new(&initial_address, once(Ok(0)));
            assert_eq!(
                0,
                emulator.into_output_iter().collect::<Result<Vec<_>, _>>()?[0]
            );
        }
        {
            let emulator = Emulator::new(&initial_address, once(Ok(2)));
            assert_eq!(
                1,
                emulator.into_output_iter().collect::<Result<Vec<_>, _>>()?[0]
            );
        }

        Ok(())
    }

    #[test]
    fn test_long_example_with_jumps() -> Result<(), EmulatorError> {
        let initial_address = [
            3, 21, 1008, 21, 8, 20, 1005, 20, 22, 107, 8, 21, 20, 1006, 20, 31, 1106, 0, 36, 98, 0,
            0, 1002, 21, 125, 20, 4, 20, 1105, 1, 46, 104, 999, 1105, 1, 46, 1101, 1000, 1, 20, 4,
            20, 1105, 1, 46, 98, 99,
        ];
        {
            let emulator = Emulator::new(&initial_address, once(Ok(7)));
            assert_eq!(
                999,
                emulator.into_output_iter().collect::<Result<Vec<_>, _>>()?[0]
            );
        }
        {
            let emulator = Emulator::new(&initial_address, once(Ok(8)));
            assert_eq!(
                1000,
                emulator.into_output_iter().collect::<Result<Vec<_>, _>>()?[0]
            );
        }
        {
            let emulator = Emulator::new(&initial_address, once(Ok(9)));
            assert_eq!(
                1001,
                emulator.into_output_iter().collect::<Result<Vec<_>, _>>()?[0]
            );
        }

        Ok(())
    }
}
