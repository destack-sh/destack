use anyhow::Result;

mod generate;

fn main() -> Result<()> {
    generate::run()
}
