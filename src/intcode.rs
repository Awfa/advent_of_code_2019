use std::convert::{TryFrom, TryInto};
use std::ops::Index;

#[derive(Debug, Clone)]
struct OpCodeParseError(usize);

macro_rules! associated_enum {
    ($enum_name:ident($associated_type:ty, $error_type:ident) { $($variant:ident = $value:expr),+ }) => {
        #[derive(Debug, Copy, Clone)]
        enum $enum_name {
            $($variant),+
        }

        impl TryFrom<$associated_type> for $enum_name {
            type Error = $error_type;

            fn try_from(v: $associated_type) -> Result<Self, Self::Error> {
                match v {
                    $($value => Ok($enum_name::$variant)),+,
                    _ => Err( $error_type(v) )
                }
            }
        }
    }
}

associated_enum!(OpCode(usize, OpCodeParseError) {
    Add = 1,
    Multiply = 2,
    End = 99
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
        let instruction = self.get_current_instruction()?;
        match instruction {
            OpCode::Add => {
                let (parameters, parameter_start_location) = self.get_parameters(&instruction)?;
                let sum = self.try_dereference(parameters[0], parameter_start_location)?
                    + self.try_dereference(parameters[1], parameter_start_location + 1)?;
                let destination =
                    self.try_dereference_mut(parameters[2], parameter_start_location + 2)?;
                *destination = sum;
                self.instruction_pointer += 4;
                Ok(EmulatorResult::Running)
            }
            OpCode::Multiply => {
                let (parameters, parameter_start_location) = self.get_parameters(&instruction)?;
                let product = self.try_dereference(parameters[0], parameter_start_location)?
                    * self.try_dereference(parameters[1], parameter_start_location + 1)?;
                let destination =
                    self.try_dereference_mut(parameters[2], parameter_start_location + 2)?;
                *destination = product;
                self.instruction_pointer += 4;
                Ok(EmulatorResult::Running)
            }
            OpCode::End => Ok(EmulatorResult::Done),
        }
    }

    fn get_current_instruction(&self) -> Result<OpCode, EmulatorError> {
        (*self.memory.get(self.instruction_pointer).ok_or(
            EmulatorError::InstructionPointerOutOfBounds {
                position: self.instruction_pointer,
            },
        )?)
        .try_into()
        .map_err(|err: OpCodeParseError| EmulatorError::InvalidInstruction {
            value_found: err.0,
            position: self.instruction_pointer,
        })
    }

    fn get_parameters(&self, instruction: &OpCode) -> Result<(Vec<usize>, usize), EmulatorError> {
        let expected = match instruction {
            OpCode::Add | OpCode::Multiply => 3,
            OpCode::End => {
                unreachable!("Emulator::get_parameters shouldn't be called with OpCode::End")
            }
        };
        if self.instruction_pointer + expected + 1 < self.memory.len() {
            Ok((
                self.memory[self.instruction_pointer + 1..self.instruction_pointer + expected + 1]
                    .into(),
                self.instruction_pointer + 1,
            ))
        } else {
            dbg!(self.instruction_pointer);
            dbg!(expected);
            Err(EmulatorError::NotEnoughParametersForInstruction {
                instruction: *instruction as usize,
                expected,
                found: self.instruction_pointer + expected + 1 - self.memory.len(),
            })
        }
    }

    fn try_dereference(
        &self,
        index: usize,
        location_of_index: usize,
    ) -> Result<&usize, EmulatorError> {
        self.memory
            .get(index)
            .ok_or(EmulatorError::InvalidMemoryLocation {
                value_found: index,
                position: location_of_index,
            })
    }

    fn try_dereference_mut(
        &mut self,
        index: usize,
        location_of_index: usize,
    ) -> Result<&mut usize, EmulatorError> {
        self.memory
            .get_mut(index)
            .ok_or(EmulatorError::InvalidMemoryLocation {
                value_found: index,
                position: location_of_index,
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
