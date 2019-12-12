#![deny(clippy::all)]

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
        Ok(None)
    },
    2 = Multiply(factor1: ReadOnly, factor2: ReadOnly, dest: Writable) {
        *dest = factor1 * factor2;
        Ok(None)
    },
    3 = Input(dest: Writable) {
        *dest = input_iter.next().ok_or(EmulatorError::InputNonExistent)?;
        Ok(None)
    },
    4 = Output(value: ReadOnly) {
        Ok(Some(value))
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

pub struct Emulator<I: Iterator<Item = EmulatorMemoryType>> {
    memory: Vec<EmulatorMemoryType>,
    instruction_pointer: usize,
    input_iter: I,
}

impl<I: Iterator<Item = EmulatorMemoryType>> Emulator<I> {
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
                Some(next_instruction_offset) => {
                    self.instruction_pointer += next_instruction_offset;
                }
            }

            if let Some(output) = output {
                return EmulatorResult::SuccessWithValue(output);
            }

            EmulatorResult::Success
        })
    }

    pub fn into_output_iter(mut self) -> impl Iterator<Item = Result<EmulatorMemoryType, EmulatorError>> {
        std::iter::from_fn(move || {
            while match self.step() {
                Ok(EmulatorResult::Done) => false,
                Ok(EmulatorResult::Success) => true,
                Ok(EmulatorResult::SuccessWithValue(value)) => {
                    return Some(Ok(value))
                }
                Err(e) => {
                    return Some(Err(e))
                }
            } {}
            None
        })
    }
}

pub fn emulator_with_empty_input(
    initial_memory: &[EmulatorMemoryType],
) -> Emulator<impl Iterator<Item = EmulatorMemoryType>> {
    Emulator::new(initial_memory, std::iter::empty())
}

impl<I: Iterator<Item = EmulatorMemoryType>> Index<usize> for Emulator<I> {
    type Output = EmulatorMemoryType;

    fn index(&self, index: usize) -> &Self::Output {
        self.memory.index(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let initial_address = [3,0,4,0,99];
        let mut emulator = Emulator::new(&initial_address, std::iter::once(1337));
        assert_eq!(&initial_address, emulator.memory.as_slice());

        assert_eq!(EmulatorResult::Success, emulator.step()?);
        assert_eq!(&[1337,0,4,0,99], emulator.memory.as_slice());

        assert_eq!(EmulatorResult::SuccessWithValue(1337), emulator.step()?);
        assert_eq!(&[1337,0,4,0,99], emulator.memory.as_slice());
        assert_eq!(EmulatorResult::Done, emulator.step()?);
        assert_eq!(EmulatorResult::Done, emulator.step()?);

        Ok(())
    }

    #[test]
    fn test_output_iterator() -> Result<(), EmulatorError> {
        let initial_address = [3,0,4,0,99];
        let emulator = Emulator::new(&initial_address, std::iter::once(1337));
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
}
