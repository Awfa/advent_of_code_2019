#[deny(clippy::all)]

extern crate proc_macro;
extern crate syn;

use quote::{format_ident, quote};
use syn::{braced, bracketed, Block, parenthesized, parse_macro_input, token, Ident, Result, Token, LitInt, Stmt};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry;

mod kw {
    syn::custom_keyword!(ReadOnly);
    syn::custom_keyword!(Writable);
    syn::custom_keyword!(Input);
    syn::custom_keyword!(Output);
    syn::custom_keyword!(InstructionPointerOverride);
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
    input_ident: Option<Ident>,
    outputs_value: bool,
    instruction_pointer_override_ident: Option<Ident>,
    function: Vec<Stmt>,
    terminator: bool
}

impl Parse for OpCodeVariants {
    fn parse(input: ParseStream) -> Result<Self> {
        let code = input.parse()?;
        input.parse::<Token![=]>()?;
        let ident = input.parse()?;

        let mut parameters = Punctuated::new();
        if input.peek(token::Paren) {
            let content;
            parenthesized!(content in input);
            parameters = content.parse_terminated(Parameter::parse)?;
        }

        let mut input_ident = None;
        let mut outputs_value = false;
        let mut instruction_pointer_override_ident = None;
        if input.peek(token::Bracket) {
            let content;
            bracketed!(content in input);
            let io_declarations: Punctuated<IoDeclaration, Token![,]> = content.parse_terminated(IoDeclaration::parse)?;
            let mut input_declaration: Option<kw::Input> = None;
            let mut out_declaration: Option<kw::Output> = None;
            let mut instruction_pointer_override_declaration: Option<kw::InstructionPointerOverride> = None;
            for declaration in io_declarations.into_iter() {
                match declaration {
                    IoDeclaration::Input{keyword, ident, ..} => {
                        if let Some(_) = input_declaration {
                            return Err(syn::Error::new_spanned(keyword, "io declaration can only be declared once"));
                        } else {
                            input_declaration = Some(keyword);
                            input_ident = Some(ident);
                        }
                    },
                    IoDeclaration::Output{keyword} => {
                        if let Some(_) = out_declaration {
                            return Err(syn::Error::new_spanned(keyword, "io declaration can only be declared once"));
                        } else {
                            out_declaration = Some(keyword);
                            outputs_value = true;
                        }
                    },
                    IoDeclaration::InstructionPointerOverride{keyword, ident, ..} => {
                        if let Some(_) = instruction_pointer_override_declaration {
                            return Err(syn::Error::new_spanned(keyword, "instruction pointer override declaration can only be declared once"));
                        } else {
                            instruction_pointer_override_declaration = Some(keyword);
                            instruction_pointer_override_ident = Some(ident);
                        }
                    }
                }
            }
        }

        let mut function = Vec::new();
        if input.peek(token::Brace) {
            let content;
            braced!(content in input);
            function = content.call(Block::parse_within)?;
        }


        let mut terminator = false;
        if input.peek(token::Bang) {
            input.parse::<token::Bang>()?;
            terminator = true;
        }

        Ok(OpCodeVariants{
            code,
            ident,
            parameters,
            input_ident,
            outputs_value,
            instruction_pointer_override_ident,
            function,
            terminator
        })
    }
}

struct Parameter {
    ident: Ident,
    separator: Token![:],
    parameter_type: ParameterType
}

impl Parse for Parameter {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Parameter{
            ident: input.parse()?,
            separator: input.parse()?,
            parameter_type: input.parse()?
        })
    }
}

enum ParameterType {
    ReadOnly {
        keyword: kw::ReadOnly
    },
    Writable {
        keyword: kw::Writable
    }
}

impl Parse for ParameterType {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::ReadOnly) {
            Ok(ParameterType::ReadOnly {
                keyword: input.parse()?
            })
        } else if lookahead.peek(kw::Writable) {
            Ok(ParameterType::Writable {
                keyword: input.parse()?
            })
        } else {
            Err(lookahead.error())
        }
    }
}

enum IoDeclaration {
    Input {
        ident: Ident,
        separator: Token![:],
        keyword: kw::Input
    },
    Output {
        keyword: kw::Output
    },
    InstructionPointerOverride {
        ident: Ident,
        separator: Token![:],
        keyword: kw::InstructionPointerOverride
    }
}

impl Parse for IoDeclaration {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::Output) {
            Ok(IoDeclaration::Output {
                keyword: input.parse()?
            })
        } else if lookahead.peek(Ident) {
            let ident = input.parse()?;
            let separator = input.parse()?;
            let lookahead = input.lookahead1();
            if lookahead.peek(kw::Input) {
                Ok(IoDeclaration::Input{
                    ident,
                    separator,
                    keyword: input.parse()?
                })
            } else if lookahead.peek(kw::InstructionPointerOverride) {
                Ok(IoDeclaration::InstructionPointerOverride {
                    ident,
                    separator,
                    keyword: input.parse()?
                })
            } else {
                Err(lookahead.error())
            }
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
            fn get_current_instruction(memory: &[EmulatorMemoryType], instruction_pointer: usize) -> Result<(#enum_name, impl Iterator<Item = Result<ParameterMode, EmulatorError>>), EmulatorError> {
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
            fn to_opcode(&self) -> EmulatorMemoryType {
                match self {
                    #(#translation_to_code_match_arms),*,
                }
            }
        }
    };

    let variant_handler_functions = input.variants.iter().map(|variant: &OpCodeVariants| {
        let ident = &variant.ident;
        let stmts = &variant.function;

        let fn_param_list = variant.parameters.iter().map(|parameter| {
            let param_ident = &parameter.ident;
            match parameter.parameter_type {
                ParameterType::ReadOnly{..} => quote!{
                    #param_ident: EmulatorMemoryType
                },
                ParameterType::Writable{..} => quote!{
                    #param_ident: &mut EmulatorMemoryType
                }
            }
        });

        let mut parameters = Vec::new();
        let iterator_bound = if let Some(ident) = &variant.input_ident {
            parameters.push(quote!{#ident: &mut I});
            quote!{<I: Iterator<Item = Result<EmulatorMemoryType, EmulatorError>>>}
        } else {
            quote!{}
        };

        if let Some(ident) = &variant.instruction_pointer_override_ident {
            parameters.push(quote!{#ident: &mut Option<EmulatorMemoryType>});
        };

        parameters.extend(fn_param_list);
        let parameters = quote!{(#(#parameters),*)};

        let okay_type = if variant.outputs_value {
            quote!{EmulatorMemoryType}
        } else {
            quote!{()}
        };

        let handler_name = format_ident!("handle_{}", ident.to_string().to_lowercase());
        quote!{
            fn #handler_name#iterator_bound#parameters -> Result<#okay_type, EmulatorError> {
                Ok({#(#stmts)*})
            }
        }
    });

    let variant_handler_dispatchers = input.variants.iter().map(|variant| {
        let ident = &variant.ident;
        let parameter_amt = variant.parameters.len();
        let parameter_bounds_guard = if parameter_amt > 0 {
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

        let parameter_initializers = variant.parameters.iter().enumerate().map(|(idx, parameter)| {
            let param_ident = &parameter.ident;
            match parameter.parameter_type {
                ParameterType::ReadOnly{..} => quote!{
                    let #param_ident: EmulatorMemoryType = match parameter_mode_iterator.next().unwrap()? {
                        ParameterMode::Position => {
                            let parameter_location = instruction_pointer + #idx + 1;
                            let address = memory[parameter_location];
                            let error = EmulatorError::InvalidMemoryLocation {
                                value_found: address,
                                position: parameter_location,
                            };
                            let address_converted = std::convert::TryInto::<usize>::try_into(address).or(Err(error))?;
                            *memory.get(address_converted)
                                .ok_or(error)?
                        },
                        ParameterMode::Immediate => {
                            memory[instruction_pointer + #idx + 1]
                        }
                    };
                },
                ParameterType::Writable{..} => quote!{
                    let #param_ident: &mut EmulatorMemoryType = match parameter_mode_iterator.next().unwrap()? {
                        ParameterMode::Position => {
                            let parameter_location = instruction_pointer + #idx + 1;
                            let address = memory[parameter_location];
                            let error = EmulatorError::InvalidMemoryLocation {
                                value_found: address,
                                position: parameter_location,
                            };
                            let address_converted = std::convert::TryInto::<usize>::try_into(address).or(Err(error))?;
                            memory.get_mut(address_converted)
                                .ok_or(error)?
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

        let (output_binding, output) = if variant.outputs_value {
            (quote!{let output: EmulatorMemoryType}, quote!{Some(output)})
        } else {
            (quote!{let _: ()}, quote!{None})
        };

        let handler_name = format_ident!("handle_{}", ident.to_string().to_lowercase());

        let mut parameters = Vec::new();
        if variant.input_ident.is_some() {
            parameters.push(quote!{input_iter});
        }

        if variant.instruction_pointer_override_ident.is_some() {
            parameters.push(quote!{&mut new_instruction_pointer});
        }

        parameters.extend(variant.parameters.iter().map(|parameter| &parameter.ident).map(|ident| quote!{#ident}));

        let statement_runner = quote!{
            #output_binding = #enum_name::#handler_name(#(#parameters),*)?;
        };

        let instruction_offset = parameter_amt + 1; // + 1 for the instruction itself
        let instruction_pointer_update = if variant.terminator {
            quote!{
                None
            }
        } else if variant.instruction_pointer_override_ident.is_some() {
            quote!{
                Some(new_instruction_pointer.map(|value| std::convert::TryInto::<usize>::try_into(value)
                                                    .or(Err(EmulatorError::InvalidMemoryLocation{value_found: value, position: 0})))
                .unwrap_or(Ok(instruction_pointer + #instruction_offset))?)
            }
        } else {
            quote!{
                Some(instruction_pointer + #instruction_offset)
            }
        };
        quote!{
            #enum_name::#ident => {
                #parameter_bounds_guard
                #(#parameter_initializers)*
                #statement_runner
                Ok((#instruction_pointer_update, #output))
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

                fn run<I: Iterator<Item = Result<EmulatorMemoryType, EmulatorError>>>(memory: &mut [EmulatorMemoryType], instruction_pointer: usize, input_iter: &mut I) -> Result<(Option<usize>, Option<EmulatorMemoryType>), EmulatorError> {
                    let (instruction, mut parameter_mode_iterator) = #enum_name::get_current_instruction(memory, instruction_pointer)?;
                    let mut new_instruction_pointer = None;
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
