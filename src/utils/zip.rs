use actix_multipart::form::tempfile::TempFile;
use mime_guess::from_path;
use std::fs::File;
use std::io::{Read, Write};
use tempfile::NamedTempFile;
use zip::read::ZipArchive;

pub fn extract_file(file: File) -> Vec<TempFile> {
    let mut archive = ZipArchive::new(file).unwrap();
    let mut files = Vec::new();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();

        // Skip directories, macOS metadata, and .DS_Store files
        // .DS_Store files are created by macOS Finder and are not useful for our purposes
        if file.is_dir()
            || file.name().starts_with("__MACOSX")
            || file.name().ends_with(".DS_Store")
        {
            continue;
        }

        let mut content = Vec::new();
        file.read_to_end(&mut content).unwrap();

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(&content).unwrap();

        let file_name = file.name().to_string();
        let content_type = from_path(&file_name).first_or_octet_stream();

        let temp_file = TempFile {
            file: temp_file,
            content_type: Some(content_type),
            file_name: Some(file_name),
            size: content.len(),
        };

        files.push(temp_file);
    }

    files
}
