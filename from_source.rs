#[cfg(feature = "from-source")]
use crate::download::download_and_extract_tar_gz;
#[cfg(feature = "from-source")]
use std::env;
use std::path::PathBuf;

#[cfg(feature = "from-source")]
pub fn is_from_source_feature_enabled() -> bool {
    true
}

#[cfg(not(feature = "from-source"))]
pub fn is_from_source_feature_enabled() -> bool {
    false
}

#[cfg(not(feature = "from-source"))]
pub fn download_scip_source() -> PathBuf {
    unimplemented!("Cannot download SCIP source code without the `from-source` feature")
}

#[cfg(feature = "from-source")]
pub fn download_scip_source() -> PathBuf {
    let scip_version = "10.0.2";
    let url = format!(
        "https://github.com/scipopt/scip/releases/download/v{scip_version}/scipoptsuite-{scip_version}.tgz"
    );
    let target = env::var("OUT_DIR").unwrap();
    let target = std::path::Path::new(&target);
    if target.join(format!("scipoptsuite-{scip_version}")).exists() {
        println!("cargo:warning=SCIP was previously downloaded, skipping download");
    } else {
        download_and_extract_tar_gz(&url, target).expect("Failed to download SCIP");
    }
    target.join(format!("scipoptsuite-{scip_version}"))
}

#[cfg(feature = "from-source")]
pub fn compile_scip(source_path: PathBuf) -> PathBuf {
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_dir = std::path::Path::new(&out_dir);
    let lib_path = out_dir.join("lib");
    if lib_path.exists() {
        println!("cargo:warning=SCIP was previously compiled, skipping compilation");
        return out_dir.to_path_buf();
    }

    use cmake::Config;
    let dst = Config::new(source_path)
        .define("CMAKE_BUILD_TYPE", "Release")
        .define("CMAKE_POSITION_INDEPENDENT_CODE", "ON")
        .define("LPS", "spx")
        .define("LPSEXACT", "none")
        .define("EXACTSOLVE", "OFF")
        .define("AMPL", "OFF")
        .define("CONOPT", "OFF")
        .define("IPOPT", "OFF")
        .define("LAPACK", "OFF")
        .define("ZIMPL", "OFF")
        .define("GMP", "OFF")
        .define("MPFR", "OFF")
        .define("QUADMATH", "OFF")
        .define("READLINE", "OFF")
        .define("BOOST", "OFF")
        .define("AUTOBUILD", "OFF")
        .define("PAPILO", "OFF")
        .define("SYM", "snauty")
        .define("TPI", "tny")
        .define("WORHP", "OFF")
        .define("ZLIB", "OFF")
        .define("LTO", "OFF")
        .define("SHARED", "OFF")
        .define("CMAKE_INSTALL_LIBDIR", "lib")
        .define("GCG", "OFF")
        .define("UG", "OFF")
        .define("SANITIZE_ADDRESS", "OFF")
        .define("SANITIZE_MEMORY", "OFF")
        .define("SANITIZE_UNDEFINED", "OFF")
        .define("SANITIZE_THREAD", "OFF")
        .build();

    dst
}

#[cfg(not(feature = "from-source"))]
pub fn compile_scip(_source_path: PathBuf) -> PathBuf {
    unimplemented!("Cannot compile SCIP without the `from-source` feature")
}
