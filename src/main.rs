use rusty_viking::MietteDefaultConfig;
static FOOTER: &'static str = "If the bug continues raise an issue on github.";
mod cli;
fn main() -> miette::Result<()> {
    MietteDefaultConfig::init_set_panic_hook(Some(FOOTER.into()))?;

    let args = cli::Cli::parse()?;
    dbg!(args);
    Ok(())
}
