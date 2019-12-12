#[deny(clippy::all)]

extern crate proc_macro;
extern crate syn;

use quote::{format_ident, quote};
use syn::{braced, Block, parenthesized, parse_macro_input, token, Ident, Result, Token, LitInt, Stmt};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry;

mod kw {
    syn::custom_keyword!(ReadOnly);
    syn::custom_keyword!(Writable);
}

struct OpCodeDeclaration {
    ident: Ident,
    variants: Punctuated<OpCodeVariants, Token![,]>
}

impl Parse for OpCodeDeclaration {
    fn parse(input: ParseStream) -> Result<Self> {
        let ident = input.parse()?;
        let content;
        braced!(content in input);
        let variants = content.parse_terminated(OpCodeVariants::parse)?;
        Ok(OpCodeDeclaration{
            ident,
            variants
        })
    }
}

struct OpCodeVariants {
    code: LitInt,
    ident: Ident,
    parameters: Punctuated<Parameter, Token![,]>,
    function: Vec<Stmt>,
    terminator: bool
}

impl Parse for OpCodeVariants {
    fn parse(input: ParseStream) -> Result<Self> {
        let code = input.parse()?;
        input.parse::<Token![=]>()?;
        let ident = input.parse()?;

        let mut parameters = Punctuated::new();
        let mut function = Vec::new();
        let mut terminator = false;

        if input.peek(token::Paren) {
            let content;
            parenthesized!(content in input);
            parameters = content.parse_terminated(Parameter::parse)?;

            let lookahead = input.lookahead1();
            if lookahead.peek(token::Brace) {
                let content;
                braced!(content in input);
                function = content.call(Block::parse_within)?;
            } else {
                return Err(lookahead.error());
            }
        } else if input.peek(token::Bang) {
            input.parse::<token::Bang>()?;
            terminator = true;
        }

        Ok(OpCodeVariants{
            code,
            ident,
            parameters,
            function,
            terminator
        })
    }
}

struct Parameter {
    ident: Ident,
    parameter_type: ParameterType
}

impl Parse for Parameter {
    fn parse(input: ParseStream) -> Result<Self> {
        let ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let parameter_type = input.parse()?;

        Ok(Parameter{
            ident,
            parameter_type
        })
    }
}

enum ParameterType {
    ReadOnly,
    Writable
}

impl Parse for ParameterType {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::ReadOnly) {
            input.parse::<kw::ReadOnly>()?;
            Ok(ParameterType::ReadOnly)
        } else if lookahead.peek(kw::Writable) {
            input.parse::<kw::Writable>()?;
            Ok(ParameterType::Writable)
        } else {
            Err(lookahead.error())
        }
    }
}

#[proc_macro]
pub fn make_op_code(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as OpCodeDeclaration);

    let enum_name = input.ident;
    let variant_idents = input.variants.iter().map(|variant| &variant.ident);

    let mut unique : HashMap<String, String> = HashMap::new();
    let compile_errors_opcode_names = variant_idents.clone().map(Clone::clone).map(|x| x.to_string()).filter_map(|ident| {
        let lowercased_ident = ident.to_lowercase();
        match unique.entry(lowercased_ident) {
            Entry::Occupied(entry) => {
                let error = format!("opcode '{}' and '{}' conflict case-insensitively - please change one of them", entry.get(), ident);
                Some(quote!{
                    compile_error!(#error);
                })
            }
            Entry::Vacant(entry) =>  {
                entry.insert(ident);
                None
            }
        }
    });

    let mut unique : HashSet<&str> = HashSet::new();
    let compile_errors_opcodes = input.variants.iter().map(|variant| &variant.code).filter_map(|code| {
        if !unique.insert(code.base10_digits()) {
            let error = format!("there is more than one definition of opcode {}", code.base10_digits());
            Some(quote!{
                compile_error!(#error);
            })
        } else {
            None
        }
    });

    let compile_errors: Vec<_> = compile_errors_opcode_names.chain(compile_errors_opcodes).collect();

    let get_current_instruction_fn = {
        let translation_from_code_match_arms = input.variants.iter().map(|variant| {
            let ident = &variant.ident;
            let code = &variant.code;
            quote!{#code => Ok(#enum_name::#ident)}
        });
        quote!{
            fn get_current_instruction(memory: &[usize], instruction_pointer: usize) -> Result<(#enum_name, impl Iterator<Item = Result<ParameterMode, EmulatorError>>), EmulatorError> {
                let instruction_value = *memory.get(instruction_pointer).ok_or(
                    EmulatorError::InstructionPointerOutOfBounds {
                        position: instruction_pointer,
                    })?;
                let instruction = match (instruction_value % 100) {
                    #(#translation_from_code_match_arms),*,
                    _ => Err(EmulatorError::InvalidInstruction{value_found: instruction_value, position: instruction_pointer})
                }?;

                let parameter_mode_iterator = {
                    let mut parameter_mode_digits = instruction_value / 100;
                    std::iter::from_fn(move || {
                        let parameter_mode_digit = parameter_mode_digits % 10;
                        let result = match parameter_mode_digit {
                            0 => Ok(ParameterMode::Position),
                            1 => Ok(ParameterMode::Immediate),
                            _ => Err(EmulatorError::InvalidParameterMode{value_found: parameter_mode_digit, position: instruction_pointer}),
                        };
                        parameter_mode_digits /= 10;
                        Some(result)
                    })
                };

                Ok((instruction, parameter_mode_iterator))
            }
        }
    };

    let to_opcode_fn = {
        let translation_to_code_match_arms = input.variants.iter().map(|variant| {
            let ident = &variant.ident;
            let code = &variant.code;
            quote!{#enum_name::#ident => #code}
        });
        quote!{
            fn to_opcode(&self) -> usize {
                match self {
                    #(#translation_to_code_match_arms),*,
                }
            }
        }
    };

    let variant_handler_functions = input.variants.iter().map(|variant| {
        let ident = &variant.ident;
        let stmts = &variant.function;

        let fn_param_list = variant.parameters.iter().map(|parameter| {
            let param_ident = &parameter.ident;
            match parameter.parameter_type {
                ParameterType::ReadOnly => quote!{
                    #param_ident: usize
                },
                ParameterType::Writable => quote!{
                    #param_ident: &mut usize
                }
            }
        });

        let handler_name = format_ident!("handle_{}", ident.to_string().to_lowercase());
        quote!{
            fn #handler_name(#(#fn_param_list),*) {
                #(#stmts)*
            }
        }
    });

    let variant_handler_dispatchers = input.variants.iter().map(|variant| {
        let ident = &variant.ident;
        let parens = variant.parameters.iter().enumerate().map(|(idx, parameter)| {
            let param_ident = &parameter.ident;
            match parameter.parameter_type {
                ParameterType::ReadOnly => quote!{
                    let #param_ident = match parameter_mode_iterator.next().unwrap()? {
                        ParameterMode::Position => {
                            let parameter_location = instruction_pointer + #idx + 1;
                            let address = memory[parameter_location];
                            *memory.get(address)
                                .ok_or(EmulatorError::InvalidMemoryLocation {
                                    value_found: address,
                                    position: parameter_location,
                                })?
                        },
                        ParameterMode::Immediate => {
                            memory[instruction_pointer + #idx + 1]
                        }
                    };
                },
                ParameterType::Writable => quote!{
                    let #param_ident = match parameter_mode_iterator.next().unwrap()? {
                        ParameterMode::Position => {
                            let parameter_location = instruction_pointer + #idx + 1;
                            let address = memory[parameter_location];
                            memory.get_mut(address)
                                .ok_or(EmulatorError::InvalidMemoryLocation {
                                    value_found: address,
                                    position: parameter_location,
                                })?
                        },
                        ParameterMode::Immediate => {
                            return Err(EmulatorError::UnexpectedParameterModeForWritable {
                                value_found: 1,
                                position: instruction_pointer + #idx + 1,
                            })
                        }
                    };
                }
            }
        });
        let parameter_amt = variant.parameters.len();
        let paren_guard = if parameter_amt > 0 {
            quote!{
                if instruction_pointer + 1 + #parameter_amt >= memory.len() {
                    return Err(EmulatorError::NotEnoughParametersForInstruction {
                        instruction: instruction.to_opcode(),
                        expected: #parameter_amt,
                        found: instruction_pointer + 1 + #parameter_amt - memory.len(),
                    })
                }
            }
        } else {
            quote!{}
        };
        let handler_name = format_ident!("handle_{}", ident.to_string().to_lowercase());
        let statement_runner = {
            let param_list = variant.parameters.iter().map(|parameter| &parameter.ident);

            quote!{
                #enum_name::#handler_name(#(#param_list),*);
            }
        };
        let next_action = if variant.terminator {
            quote!{Ok(None)}
        } else {
            let instruction_offset = parameter_amt + 1; // + 1 for the instruction itself
            quote!(Ok(Some(#instruction_offset)))
        };
        quote!{
            #enum_name::#ident => {
                #paren_guard
                #(#parens)*
                #statement_runner
                #next_action
            }
        }
    });

    let output = if compile_errors.len() > 0 {
        quote! {
            #(#compile_errors)*
        }
    } else {
        quote! {
            pub enum #enum_name {
                #(#variant_idents),*
            }

            impl #enum_name {
                #get_current_instruction_fn
                #to_opcode_fn

                #(#variant_handler_functions)*

                fn run(memory: &mut [usize], instruction_pointer: usize) -> Result<Option<usize>, EmulatorError> {
                    let (instruction, mut parameter_mode_iterator) = #enum_name::get_current_instruction(memory, instruction_pointer)?;
                    match instruction {
                        #(#variant_handler_dispatchers),*
                    }
                }
            }
        }
    };

    println!("{}", output);

    proc_macro::TokenStream::from(output)
}
