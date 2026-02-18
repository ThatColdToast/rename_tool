use std::env;
use std::fs;
use std::path::PathBuf;

const EXPORT_USAGE: &str = "Usage: rename_tool export <directory_path> [output_csv]";
const IMPORT_USAGE: &str = "Usage: rename_tool import <directory_path> <input_csv>";

fn main() {
    let mut args = env::args().skip(1);

    let Some(command) = args.next() else {
        eprintln!("{EXPORT_USAGE}");
        eprintln!("{IMPORT_USAGE}");
        std::process::exit(1);
    };

    match command.as_str() {
        "export" => {
            let Some(directory_path) = args.next() else {
                eprintln!("{EXPORT_USAGE}");
                std::process::exit(1);
            };

            let output_csv_path = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("folders.csv"));

            if args.next().is_some() {
                eprintln!("{EXPORT_USAGE}");
                std::process::exit(1);
            }

            export(PathBuf::from(directory_path), output_csv_path);
        }
        "import" => {
            let Some(directory_path) = args.next() else {
                eprintln!("{IMPORT_USAGE}");
                std::process::exit(1);
            };

            let Some(input_csv) = args.next() else {
                eprintln!("{IMPORT_USAGE}");
                std::process::exit(1);
            };

            if args.next().is_some() {
                eprintln!("{IMPORT_USAGE}");
                std::process::exit(1);
            }

            import(PathBuf::from(directory_path), PathBuf::from(input_csv));
        }
        _ => {
            eprintln!("{EXPORT_USAGE}");
            eprintln!("{IMPORT_USAGE}");
            std::process::exit(1);
        }
    }
}

fn export(directory_path: PathBuf, output_csv_path: PathBuf) {
    let resolved_path = resolve_path(directory_path);

    if !resolved_path.is_dir() {
        eprintln!("Not a valid directory: {}", resolved_path.display());
        std::process::exit(1);
    }

    let entries = match fs::read_dir(&resolved_path) {
        Ok(entries) => entries,
        Err(error) => {
            eprintln!(
                "Failed to read directory {}: {error}",
                resolved_path.display()
            );
            std::process::exit(1);
        }
    };

    let mut writer = match csv::Writer::from_path(&output_csv_path) {
        Ok(writer) => writer,
        Err(error) => {
            eprintln!(
                "Failed to create CSV {}: {error}",
                output_csv_path.display()
            );
            std::process::exit(1);
        }
    };

    if let Err(error) = writer.write_record(["old_name"]) {
        eprintln!(
            "Failed to write CSV header {}: {error}",
            output_csv_path.display()
        );
        std::process::exit(1);
    }

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let Some(folder_name_os) = path.file_name() else {
                continue;
            };

            let folder_name = folder_name_os.to_string_lossy();
            if let Err(error) = writer.write_record([folder_name.as_ref()]) {
                eprintln!(
                    "Failed to write CSV row {}: {error}",
                    output_csv_path.display()
                );
                std::process::exit(1);
            }
        }
    }

    if let Err(error) = writer.flush() {
        eprintln!("Failed to flush CSV {}: {error}", output_csv_path.display());
        std::process::exit(1);
    }

    println!("Wrote CSV: {}", output_csv_path.display());
}

fn import(directory_path: PathBuf, input_csv: PathBuf) {
    let resolved_directory = resolve_path(directory_path);

    if !resolved_directory.is_dir() {
        eprintln!("Not a valid directory: {}", resolved_directory.display());
        std::process::exit(1);
    }

    let resolved_csv = resolve_path(input_csv);

    if !resolved_csv.is_file() {
        eprintln!("Not a valid CSV file: {}", resolved_csv.display());
        std::process::exit(1);
    }

    let mut reader = match csv::Reader::from_path(&resolved_csv) {
        Ok(reader) => reader,
        Err(error) => {
            eprintln!("Failed to read CSV {}: {error}", resolved_csv.display());
            std::process::exit(1);
        }
    };

    let headers = match reader.headers() {
        Ok(headers) => headers.clone(),
        Err(error) => {
            eprintln!(
                "Failed to read CSV headers {}: {error}",
                resolved_csv.display()
            );
            std::process::exit(1);
        }
    };

    if headers.get(0) != Some("old_name") || headers.get(1) != Some("new_name") {
        eprintln!(
            "Invalid CSV headers in {}. Expected: old_name,new_name",
            resolved_csv.display()
        );
        std::process::exit(1);
    }

    for (index, result) in reader.records().enumerate() {
        let record = match result {
            Ok(record) => record,
            Err(error) => {
                eprintln!("Failed to read CSV row {}: {error}", index + 2);
                continue;
            }
        };

        let old_name = record.get(0).unwrap_or("").trim();
        let new_name = record.get(1).unwrap_or("").trim();

        if old_name.is_empty() || new_name.is_empty() {
            eprintln!("Skipping row {}: empty old_name or new_name", index + 2);
            continue;
        }

        let old_path = resolved_directory.join(old_name);
        let new_path = resolved_directory.join(new_name);

        if !old_path.is_dir() {
            eprintln!(
                "Skipping row {}: source folder does not exist: {}",
                index + 2,
                old_path.display()
            );
            continue;
        }

        if new_path.exists() {
            eprintln!(
                "Skipping row {}: target already exists: {}",
                index + 2,
                new_path.display()
            );
            continue;
        }

        if let Err(error) = fs::rename(&old_path, &new_path) {
            eprintln!(
                "Failed to rename row {} ({} -> {}): {error}",
                index + 2,
                old_name,
                new_name
            );
            continue;
        }

        println!("Renamed: {} -> {}", old_name, new_name);
    }
}

fn resolve_path(path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        match env::current_dir() {
            Ok(current_dir) => current_dir.join(path),
            Err(error) => {
                eprintln!("Failed to get current directory: {error}");
                std::process::exit(1);
            }
        }
    }
}
