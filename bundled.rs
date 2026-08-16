#[cfg(feature = "bundled")]
use crate::download::{download_and_extract_zip, extract_zip_file};
#[cfg(feature = "bundled")]
use std::env;
#[cfg(feature = "bundled")]
use std::path::PathBuf;

/// Map the current target OS/arch to the platform tag used both for the
/// prebuilt SCIP download and for selecting the matching prebuilt bindings in
/// `src/bindings/<tag>.rs`. Keeping a single source of truth ensures the
/// downloaded library and the committed bindings always refer to the same
/// platform.
#[cfg(feature = "bundled")]
pub fn target_string() -> String {
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();

    let os_string = if os == "linux" && arch == "x86_64" {
        "linux"
    } else if os == "linux" && arch == "aarch64" {
        "linux-arm"
    } else if os == "macos" && arch == "x86_64" {
        "macos-intel"
    } else if os == "macos" && arch == "aarch64" {
        "macos-arm"
    } else if os == "windows" && arch == "x86_64" {
        "windows"
    } else {
        panic!("Unsupported OS-arch combination: {}-{}", os, arch);
    };

    os_string.to_string()
}

#[cfg(feature = "bundled")]
pub fn download_scip() {
    let extract_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    println!("cargo:rerun-if-env-changed=SCIP_SYS_BUNDLE_PATH");

    let local_bundle = env::var_os("SCIP_SYS_BUNDLE_PATH").map(PathBuf::from);
    let bundle_source = match &local_bundle {
        Some(path) => {
            if !path.is_absolute() {
                panic!(
                    "SCIP_SYS_BUNDLE_PATH must be an absolute path: {}",
                    path.display()
                );
            }
            println!("cargo:rerun-if-changed={}", path.display());
            format!("file:{}", path.display())
        }
        None => format!(
            "https://github.com/incitron/scip-sys/releases/download/\
             kessel-scip-10.0.2-r1/libscip-{}.zip",
            target_string()
        ),
    };
    let source_marker = extract_path.join("scip-bundle-source.txt");
    let installed = extract_path.join("scip_install");
    let source_matches = std::fs::read_to_string(&source_marker)
        .map(|source| source == bundle_source)
        .unwrap_or(false);

    if installed.exists() && source_matches && local_bundle.is_none() {
        println!("cargo:warning=SCIP was previously downloaded, skipping download");
        return;
    }
    if installed.exists() {
        std::fs::remove_dir_all(&installed).unwrap_or_else(|error| {
            panic!(
                "failed to remove stale SCIP bundle at {}: {error}",
                installed.display()
            )
        });
    }

    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    println!("cargo:warning=Detected OS: {}", os);
    println!("cargo:warning=Detected arch: {}", arch);

    let result = match local_bundle {
        Some(path) => {
            println!("cargo:warning=Using local SCIP bundle {}", path.display());
            extract_zip_file(&path, &extract_path)
        }
        None => download_and_extract_zip(&bundle_source, &extract_path),
    };
    result.unwrap_or_else(|error| panic!("Failed to obtain SCIP bundle: {error}"));

    if !installed.exists() {
        panic!(
            "SCIP bundle did not contain the expected scip_install directory: {}",
            installed.display()
        );
    }
    std::fs::write(source_marker, bundle_source)
        .unwrap_or_else(|error| panic!("failed to record SCIP bundle source: {error}"));
}
