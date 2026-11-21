use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;

fn main() -> Result<(), Box<dyn Error>> {
    let assets = Path::new("assets");
    let output_dir = PathBuf::from(env::var_os("OUT_DIR").ok_or("Cargo не задал OUT_DIR")?);
    let model = assets.join("corwin.obj.gz");

    // BFGE читает обычный OBJ. Распаковываем модель один раз при сборке.
    println!("cargo:rerun-if-changed={}", model.display());
    let mut source = GzDecoder::new(BufReader::new(File::open(model)?));
    let mut destination = BufWriter::new(File::create(output_dir.join("corwin.obj"))?);
    io::copy(&mut source, &mut destination)?;
    destination.flush()?;

    for name in ["corwin.mtl", "corwin-palette.png"] {
        let material = assets.join(name);
        println!("cargo:rerun-if-changed={}", material.display());
        fs::copy(material, output_dir.join(name))?;
    }
    Ok(())
}
