use clap::Parser;
use colored::Colorize;
use glob::glob;

#[derive(Parser)]
struct CliArgs {
    #[clap(parse(from_os_str))]
    source_dir: std::path::PathBuf,
    #[clap(parse(from_os_str))]
    target_dir: std::path::PathBuf,
    #[clap(short, long)]
    quiet: bool,
    #[clap(short, long)]
    depth: Option<u8>,
    #[clap(long)]
    no_color: bool,
}

fn glob_pattern_from_path_buf(path_buf: &std::path::PathBuf, depth: Option<u8>) -> String {
    // Return a glob pattern for every file in a directory (recursively) from a PathBuf, assumed to
    // be a directory

    match depth {
        None => format!("{}{}", path_buf.as_path().to_str().unwrap(), "/**/*"),
        Some(d) => {
            let mut output = String::from(path_buf.as_path().to_str().unwrap());

            for _ in 0..d {
                output.push_str("/*");
            }

            output
        }
    }
}

fn get_dir_listing(dir_path: &std::path::PathBuf, depth: Option<u8>) -> Vec<std::path::PathBuf> {
    // Return a full recursive directory listing

    let absolute_dir_path = std::fs::canonicalize(dir_path).unwrap();

    let glob_pattern = glob_pattern_from_path_buf(&absolute_dir_path, depth);

    let maybe_paths = glob(&glob_pattern).expect("Failed to read glob pattern");

    let mut paths: Vec<std::path::PathBuf> = Vec::new();

    // remove the parent directory from the paths, so the diffs don't show everything as different
    for maybe_path in maybe_paths {
        let path = match maybe_path {
            Ok(p) => p,
            Err(e) => {
                // print the error if it doesn't have permission to read the dir, or other errors
                eprintln!("{}", e);
                continue;
            }
        };

        // if the path doesn't have any parent directories, then just add it
        if path.components().count() == 1 {
            paths.push(path);
            continue;
        }

        // otherwise remove the parent directory
        paths.push(path.strip_prefix(&absolute_dir_path).unwrap().to_path_buf());
    }

    paths
}

fn dir_listing_to_string(dir_listing: &Vec<std::path::PathBuf>) -> String {
    // Return the directory listing in string form

    let mut string = String::new();

    // put each path on a new line, making sure there is no trailing "\n" at the end
    string.push_str(dir_listing[0].to_str().unwrap());

    for i in 1..dir_listing.len() {
        string.push_str("\n");
        string.push_str(dir_listing[i].to_str().unwrap());
    }

    string
}

fn print_dir_diff(dir_diff: &Vec<diff::Result<&str>>, hide_similarities: bool, color: bool) {
    for diff_fragment in dir_diff {
        match diff_fragment {
            diff::Result::Left(path) => {
                if color {
                    println!("{} {}", "-".red(), path.red());
                } else {
                    println!("- {}", path);
                }
            }
            diff::Result::Both(path, _) => {
                if !hide_similarities {
                    println!(" {}", path);
                }
            }
            diff::Result::Right(path) => {
                if color {
                    println!("{} {}", "+".green(), path.green());
                } else {
                    println!("+ {}", path);
                }
            }
        }
    }
}

fn print_diff_summary(dir_diff: &Vec<diff::Result<&str>>, hide_similarities: bool) {
    let mut num_removed = 0;
    let mut num_similar = 0;
    let mut num_added = 0;

    for diff_fragment in dir_diff {
        match diff_fragment {
            diff::Result::Left(_) => num_removed += 1,
            diff::Result::Both(_, _) => num_similar += 1,
            diff::Result::Right(_) => num_added += 1,
        }
    }

    if hide_similarities {
        println!("{} removed, {} added", num_removed, num_added);
    } else {
        println!(
            "{} similar, {} removed, {} added",
            num_similar, num_removed, num_added
        );
    }
}

fn check_cli_args(args: &CliArgs) -> Result<(), Box<dyn std::error::Error>> {
    // check if paths exist
    if !args.source_dir.exists() {
        return Err(format!("{} does not exist", args.source_dir.display()).into());
    }

    if !args.target_dir.exists() {
        return Err(format!("{} does not exist", args.target_dir.display()).into());
    }

    // check if paths are directories
    if !args.source_dir.is_dir() {
        return Err(format!("{} is not a directory", args.source_dir.display()).into());
    }

    if !args.target_dir.is_dir() {
        return Err(format!("{} is not a directory", args.target_dir.display()).into());
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = CliArgs::parse();

    // error if directories do not exist
    check_cli_args(&args)?;

    // list both directories
    let source_dir_listing = get_dir_listing(&args.source_dir, args.depth);
    let target_dir_listing = get_dir_listing(&args.target_dir, args.depth);

    // get diff
    let source_dir_listing_string = dir_listing_to_string(&source_dir_listing);
    let target_dir_listing_string = dir_listing_to_string(&target_dir_listing);
    let dir_diff = diff::lines(&source_dir_listing_string, &target_dir_listing_string);

    print_dir_diff(&dir_diff, args.quiet, !args.no_color);
    print_diff_summary(&dir_diff, args.quiet);

    Ok(())
}
