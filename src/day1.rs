use std::fs::File;
use std::io::prelude::*;

pub fn run_part_1(path: &str) -> u64 {
    let input_file = File::open(path).unwrap();
    let reader = std::io::BufReader::new(input_file);
    reader
        .lines()
        .map(|s| s.unwrap().parse::<u64>().unwrap())
        .map(calculate_required_fuel_naive)
        .fold(0u64, |acc, e| acc + e)
}

fn calculate_required_fuel_naive(mass: u64) -> u64 {
    (mass / 3).checked_sub(2).unwrap_or(0)
}

pub fn run_part_2(path: &str) -> u64 {
    let input_file = File::open(path).unwrap();
    let reader = std::io::BufReader::new(input_file);
    reader
        .lines()
        .map(|s| s.unwrap().parse::<u64>().unwrap())
        .map(calculate_required_fuel_with_wish)
        .fold(0u64, |acc, e| acc + e)
}

fn calculate_required_fuel_with_wish(mass: u64) -> u64 {
    let mut total_mass = 0;
    let mut fuel_mass = calculate_required_fuel_naive(mass);
    while match fuel_mass {
        0 => false,
        _ => {
            total_mass += fuel_mass;
            fuel_mass = calculate_required_fuel_naive(fuel_mass);
            true
        }
    } {}
    total_mass
}
