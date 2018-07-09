use std::{
    env, error::Error, fs::{self, File}, io::Write, path::Path,
};

const SOURCE_DIR: &str = "../client/dist/";

fn main() -> Result<(), Box<Error>> {
    let project_dir = env::var("CARGO_MANIFEST_DIR")?;
    let client_dir = Path::new(&project_dir).join(SOURCE_DIR).canonicalize()?;
    let out_dir = env::var("OUT_DIR")?;
    let dest_path = Path::new(&out_dir).join("all_the_files.rs");
    let mut all_the_files = File::create(&dest_path)?;

    writeln!(&mut all_the_files, r#"["#,)?;

    for f in fs::read_dir(SOURCE_DIR)? {
        let f = f?;

        if !f.file_type()?.is_file() {
            continue;
        }

        writeln!(
            &mut all_the_files,
            r#"("{name}", include_str!("{path}")),"#,
            name = f.path().file_name().unwrap().to_str().unwrap(),
            path = f.path().canonicalize()?.display(),
        )?;
    }

    writeln!(&mut all_the_files, r#"];"#,)?;

    Ok(())
}