use opcode_macro::make_op_code;
use std::ops::Index;

make_op_code!(OpCode {
    1 = Add(addend1: DereferencedAddress, addend2: DereferencedAddress, dest: Writable) { *dest = addend1 + addend2; },
    2 = Multiply(factor1: DereferencedAddress, factor2: DereferencedAddress, dest: Writable) { *dest = factor1 * factor2; },
    99 = End!
});

#[derive(Debug, Clone)]
pub enum EmulatorError {
    InvalidInstruction {
        value_found: usize,
        position: usize,
    },
    NotEnoughParametersForInstruction {
        instruction: usize,
        expected: usize,
        found: usize,
    },
    InvalidMemoryLocation {
        value_found: usize,
        position: usize,
    },
    InstructionPointerOutOfBounds {
        position: usize,
    },
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
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum EmulatorResult {
    Running,
    Done,
}

pub struct Emulator {
    memory: Vec<usize>,
    instruction_pointer: usize,
}

impl Emulator {
    pub fn new(initial_memory: &[usize]) -> Self {
        Emulator {
            memory: initial_memory.into(),
            instruction_pointer: 0,
        }
    }

    pub fn run_to_completion(&mut self) -> Result<usize, EmulatorError> {
        while self.step()? != EmulatorResult::Done {}
        Ok(self.memory[0])
    }

    pub fn step(&mut self) -> Result<EmulatorResult, EmulatorError> {
        OpCode::run(&mut self.memory, self.instruction_pointer).map(|run_result| match run_result {
            Some(instruction_pointer_offset) => {
                self.instruction_pointer += instruction_pointer_offset;
                EmulatorResult::Running
            }
            None => EmulatorResult::Done,
        })
    }
}

impl Index<usize> for Emulator {
    type Output = usize;

    fn index(&self, index: usize) -> &Self::Output {
        self.memory.index(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example() -> Result<(), EmulatorError> {
        let input = [1, 9, 10, 3, 2, 3, 11, 0, 99, 30, 40, 50];
        let mut emulator = Emulator::new(&input);
        assert_eq!(&input, emulator.memory.as_slice());

        assert_eq!(EmulatorResult::Running, emulator.step()?);
        assert_eq!(
            &[1, 9, 10, 70, 2, 3, 11, 0, 99, 30, 40, 50],
            emulator.memory.as_slice()
        );
        assert_eq!(EmulatorResult::Running, emulator.step()?);
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
        let input = [1, 0, 0, 0, 99];
        let mut emulator = Emulator::new(&input);
        assert_eq!(&input, emulator.memory.as_slice());

        assert_eq!(EmulatorResult::Running, emulator.step()?);
        assert_eq!(&[2, 0, 0, 0, 99], emulator.memory.as_slice());
        assert_eq!(EmulatorResult::Done, emulator.step()?);
        assert_eq!(EmulatorResult::Done, emulator.step()?);

        Ok(())
    }

    #[test]
    fn test_multiply_1() -> Result<(), EmulatorError> {
        let input = [1, 0, 0, 0, 99];
        let mut emulator = Emulator::new(&input);
        assert_eq!(&input, emulator.memory.as_slice());

        assert_eq!(EmulatorResult::Running, emulator.step()?);
        assert_eq!(&[2, 0, 0, 0, 99], emulator.memory.as_slice());
        assert_eq!(EmulatorResult::Done, emulator.step()?);
        assert_eq!(EmulatorResult::Done, emulator.step()?);

        Ok(())
    }

    #[test]
    fn test_multiply_2() -> Result<(), EmulatorError> {
        let input = [2, 4, 4, 5, 99, 0];
        let mut emulator = Emulator::new(&input);
        assert_eq!(&input, emulator.memory.as_slice());

        assert_eq!(EmulatorResult::Running, emulator.step()?);
        assert_eq!(&[2, 4, 4, 5, 99, 9801], emulator.memory.as_slice());
        assert_eq!(EmulatorResult::Done, emulator.step()?);
        assert_eq!(EmulatorResult::Done, emulator.step()?);

        Ok(())
    }

    #[test]
    fn test_overriding_future_instructions() -> Result<(), EmulatorError> {
        let input = [1, 1, 1, 4, 99, 5, 6, 0, 99];
        let mut emulator = Emulator::new(&input);
        assert_eq!(&input, emulator.memory.as_slice());

        assert_eq!(EmulatorResult::Running, emulator.step()?);
        assert_eq!(&[1, 1, 1, 4, 2, 5, 6, 0, 99], emulator.memory.as_slice());
        assert_eq!(EmulatorResult::Running, emulator.step()?);
        assert_eq!(&[30, 1, 1, 4, 2, 5, 6, 0, 99], emulator.memory.as_slice());
        assert_eq!(EmulatorResult::Done, emulator.step()?);
        assert_eq!(EmulatorResult::Done, emulator.step()?);

        Ok(())
    }
}
