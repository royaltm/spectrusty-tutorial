use std::error::Error;
use std::path::PathBuf;
use std::io::ErrorKind;
use std::fs::OpenOptions;

const ROMS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), r"/resources/roms");
const ROMS_URL: &str = "https://github.com/royaltm/spectrusty/raw/master/resources/roms/{{name}}";

type DynResult<T> = Result<T, Box<dyn Error>>;

fn test_file_path(name: &str) -> PathBuf {
    let mut filepath = PathBuf::from(ROMS_DIR);
    filepath.push(name);
    filepath
}

fn fetch_file(name: &str) -> DynResult<()> {
    let filepath = test_file_path(name);
    if let Some(mut file) = OpenOptions::new().write(true)
                                        .create_new(true)
                                        .open(filepath)
                                        .map(|f| Some(f))
                                        .or_else(|e| {
                                            if e.kind() == ErrorKind::AlreadyExists {
                                                Ok(None)
                                            }
                                            else {
                                                Err(e)
                                            }
                                        })? {
        let url = ROMS_URL.replace("{{name}}", name);
        println!("cargo:warning=fetching: {}", url);
        let mut resp = reqwest::blocking::get(&url)?
                                .error_for_status()?;
        resp.copy_to(&mut file)?;
    }
    Ok(())
}

fn ensure_rom_files() -> DynResult<()> {
    for &name in &["48.rom",
                   "128-0.rom",
                   "128-1.rom"] {
        fetch_file(name)?
    }
    Ok(())
}

fn main() -> DynResult<()> {
    // Tell Cargo that if the given file changes, to rerun this build script.
    println!("cargo:rerun-if-changed=resources/roms/48.rom");
    println!("cargo:rerun-if-changed=resources/roms/128-0.rom");
    println!("cargo:rerun-if-changed=resources/roms/128-1.rom");
    ensure_rom_files()
}
