use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;

fn main() -> Result<(), Box<dyn Error>> {
    let assets = Path::new("assets");
    let output_dir = PathBuf::from(env::var_os("OUT_DIR").ok_or("Cargo не задал OUT_DIR")?);
    // BFGE читает обычный OBJ. Распаковываем модели один раз при сборке.
    for name in ["corwin", "corwin-weapons"] {
        let model = assets.join(format!("{name}.obj.gz"));
        println!("cargo:rerun-if-changed={}", model.display());
        let mut source = GzDecoder::new(BufReader::new(File::open(model)?));
        let mut destination = BufWriter::new(File::create(output_dir.join(format!("{name}.obj")))?);
        io::copy(&mut source, &mut destination)?;
        destination.flush()?;
    }

    for name in [
        "corwin.mtl",
        "corwin-palette.png",
        "corwin-weapons.mtl",
        "corwin-belt.png",
        "corwin-sword.png",
        "corwin-sword-grip.png",
    ] {
        let material = assets.join(name);
        println!("cargo:rerun-if-changed={}", material.display());
        fs::copy(material, output_dir.join(name))?;
    }
    Ok(())
}
