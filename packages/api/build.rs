use std::fs;

fn main() {
    for path in [
        "src/db/authorized_pool.rs",
        "src/bin/knowledge-ingestion-worker.rs",
    ] {
        println!("cargo:rerun-if-changed={path}");
        println!("cargo:warning=EDUTALENT_DIAGNOSTIC_BEGIN {path}");
        match fs::read_to_string(path) {
            Ok(contents) => {
                for (index, line) in contents.lines().enumerate() {
                    println!(
                        "cargo:warning=EDUTALENT_DIAGNOSTIC {path}:{} {line}",
                        index + 1
                    );
                }
            }
            Err(error) => println!("cargo:warning=EDUTALENT_DIAGNOSTIC_MISSING {path}: {error}"),
        }
        println!("cargo:warning=EDUTALENT_DIAGNOSTIC_END {path}");
    }
}
