use std::{fs, path::Path, process::Command};

fn main() {
    let profile = std::env::var("PROFILE").unwrap();

    let dist = Path::new("frontend/dist");
    if !dist.exists() {
        fs::create_dir_all(dist).expect("Failed to create frontend/dist");
    }

    if profile == "release" {
        let status = Command::new("deno")
            .args(["task", "build"])
            .current_dir("frontend")
            .status()
            .expect("Failed to run deno");

        if !status.success() {
            panic!("Frontend build failed");
        }

        println!("cargo:rerun-if-changed=frontend/src");
        println!("cargo:rerun-if-changed=frontend/package.json");
        println!("cargo:rerun-if-changed=frontend/deno.lock");
        println!("cargo:rerun-if-changed=frontend/tsconfig.json");
        println!("cargo:rerun-if-changed=frontend/vite.config.ts");
        println!("cargo:rerun-if-changed=frontend/svelte.config.js");
        println!("cargo:rerun-if-changed=frontend/.npmrc");
        println!("cargo:rerun-if-changed=frontend/static");
    }
}
