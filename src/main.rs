use clap::Parser;
use colored::Colorize;
use glob::glob;

#[derive(Parser)]
struct CliArgs {
    #[clap(parse(from_os_str))]
    source_dir: std::path::PathBuf,
    #[clap(parse(from_os_str))]
    target_dir: std::path::PathBuf,
}

fn glob_pattern_from_path_buf(path_buf: &std::path::PathBuf) -> String {
    // Return a glob pattern for every file in a directory (recursively) from a PathBuf, assumed to
    // be a directory

    format!("{}{}", path_buf.as_path().to_str().unwrap(), "/**/*")
}

fn get_dir_listing(path_buf: &std::path::PathBuf) -> Vec<std::path::PathBuf> {
    // Return a full recursive directory listing

    let glob_pattern = glob_pattern_from_path_buf(path_buf);

    let maybe_paths = glob(&glob_pattern).expect("Failed to read glob pattern");

    let mut paths: Vec<std::path::PathBuf> = Vec::new();

    // remove the parent directory from the paths, so the diffs don't show everything as different
    for maybe_path in maybe_paths {
        let path = maybe_path.unwrap();

        // if the path doesn't have any parent directories, then just add it
        if path.components().count() == 1 {
            paths.push(path);
            continue;
        }

        // otherwise remove the parent directory
        paths.push(path.strip_prefix(path_buf).unwrap().to_path_buf());
    }

    paths
}

fn dir_listing_to_string(dir_listing: &Vec<std::path::PathBuf>) -> String {
    // Return the directory listing in string form

    let mut string = String::new();

    for path in dir_listing {
        string.push_str(path.to_str().unwrap());
        string.push_str("\n");
    }

    string
}

fn print_dir_diff(dir_diff: &Vec<diff::Result<&str>>) {
    for diff_fragment in dir_diff {
        match diff_fragment {
            diff::Result::Left(path) => println!("{} {}", "-".red(), path.red()),
            diff::Result::Both(path, _) => println!(" {}", path),
            diff::Result::Right(path) => println!("{} {}", "+".green(), path.green()),
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = CliArgs::parse();

    // error if directories do not exist
    if !args.source_dir.exists() {
        return Err("source directory does not exist".into());
    }

    if !args.target_dir.exists() {
        return Err("target directory does not exist".into());
    }

    let source_dir_listing = get_dir_listing(&args.source_dir);
    let target_dir_listing = get_dir_listing(&args.target_dir);

    let source_dir_listing_string = dir_listing_to_string(&source_dir_listing);
    let target_dir_listing_string = dir_listing_to_string(&target_dir_listing);

    let dir_diff = diff::lines(&source_dir_listing_string, &target_dir_listing_string);

    print_dir_diff(&dir_diff);

    Ok(())
}
