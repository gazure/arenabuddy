use std::{
    fs,
    io::Result,
    path::{Path, PathBuf},
};

fn main() -> Result<()> {
    // Tell Cargo to rerun this script if any .proto files change
    println!("cargo:rerun-if-changed=protos/");

    // Find all proto files
    let proto_dir = "protos";
    let proto_files = find_proto_files(proto_dir)?;

    if proto_files.is_empty() {
        println!("cargo:warning=No .proto files found in {proto_dir}");
        return Ok(());
    }

    // Convert PathBuf to &str references
    let proto_paths: Vec<&str> = proto_files.iter().filter_map(|p| p.to_str()).collect();

    // Compile the proto files with prost-build
    let mut config = prost_build::Config::new();

    // Add clippy allow attributes to generated code
    config.type_attribute(
        ".",
        "#[allow(clippy::derive_partial_eq_without_eq, clippy::pedantic, clippy::nursery)]",
    );
    config.field_attribute(".", "#[allow(clippy::pedantic, clippy::nursery)]");

    // Compile the protos
    config.compile_protos(&proto_paths, &[proto_dir])?;

    Ok(())
}

fn find_proto_files(dir: &str) -> Result<Vec<PathBuf>> {
    let mut proto_files = Vec::new();
    let dir_path = Path::new(dir);

    if !dir_path.is_dir() {
        println!("cargo:warning=Directory '{dir}' not found");
        return Ok(proto_files);
    }

    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Recursively search subdirectories
            let mut sub_proto_files = find_proto_files(path.to_str().unwrap_or(dir))?;
            proto_files.append(&mut sub_proto_files);
        } else if let Some(extension) = path.extension() {
            // Check if the file has a .proto extension
            if extension == "proto" {
                proto_files.push(path);
            }
        }
    }

    Ok(proto_files)
}
