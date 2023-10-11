use std::io::Result;

fn main() -> Result<()> {
    slint_build::compile("src/ui/main_window.slint").unwrap();
    Ok(())
}
