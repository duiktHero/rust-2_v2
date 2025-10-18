
mod part_1;
mod part_2;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let part1 = part_1::demo_transitions();
    let req = part_2::load_request()?;
    let toml = part_2::to_toml(&req)?;
    println!("{toml}");
    Ok(())
}