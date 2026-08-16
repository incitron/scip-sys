use std::error::Error;
use std::io::Cursor;
use std::io::Read;
use std::path::Path;
use std::time::Duration;
#[cfg(feature = "bundled")]
use zip_extract::extract;

#[cfg(feature = "bundled")]
pub fn download_and_extract_zip(url: &str, extract_path: &Path) -> Result<(), Box<dyn Error>> {
    // Download the ZIP file
    println!("cargo:warning=Downloading from {}", url);
    let resp = ureq::get(url).timeout(Duration::from_secs(300)).call()?;
    let mut content: Vec<u8> = Vec::new();
    resp.into_reader().read_to_end(&mut content)?;

    extract_zip_bytes(content, extract_path)?;

    Ok(())
}

#[cfg(feature = "bundled")]
pub fn extract_zip_file(zip_path: &Path, extract_path: &Path) -> Result<(), Box<dyn Error>> {
    let content = std::fs::read(zip_path)?;
    extract_zip_bytes(content, extract_path)
}

#[cfg(feature = "bundled")]
fn extract_zip_bytes(content: Vec<u8>, extract_path: &Path) -> Result<(), Box<dyn Error>> {
    println!("cargo:warning=Extracting SCIP bundle to {:?}", extract_path);
    extract(Cursor::new(content), extract_path, false)?;

    extract_nested_zip(extract_path)?;
    Ok(())
}

/// Downloads a `.tar.gz`/`.tgz` archive and extracts it into `extract_path`.
#[cfg(feature = "from-source")]
pub fn download_and_extract_tar_gz(url: &str, extract_path: &Path) -> Result<(), Box<dyn Error>> {
    println!("cargo:warning=Downloading from {}", url);
    let resp = ureq::get(url).timeout(Duration::from_secs(300)).call()?;
    let mut content: Vec<u8> = Vec::new();
    resp.into_reader().read_to_end(&mut content)?;

    println!("cargo:warning=Extracting to {:?}", extract_path);
    let decoder = flate2::read::GzDecoder::new(Cursor::new(content));
    tar::Archive::new(decoder).unpack(extract_path)?;
    Ok(())
}

/// If extracting produced a single nested zip file, extract that too (some
/// release archives wrap the install in another zip).
#[cfg(feature = "bundled")]
fn extract_nested_zip(target_dir: &Path) -> Result<(), Box<dyn Error>> {
    let extracted_files: Vec<_> = std::fs::read_dir(target_dir)?.collect();
    if extracted_files.len() == 1 {
        let first_file = extracted_files[0].as_ref().unwrap();
        if first_file
            .path()
            .extension()
            .is_some_and(|ext| ext == "zip")
        {
            println!("cargo:warning=Found nested zip file, extracting again");
            let nested_zip_path = first_file.path();
            extract(
                Cursor::new(std::fs::read(&nested_zip_path).unwrap()),
                &(target_dir.join("scip_install")),
                true,
            )?;
            std::fs::remove_file(nested_zip_path)?;
        }
    }
    Ok(())
}
