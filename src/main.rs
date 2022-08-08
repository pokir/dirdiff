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
    #[clap(short, long)]
    compare_content: bool,
}

#[derive(Debug)]
enum DirDiff<T> {
    Removed(T), // path is only in source
    Added(T),   // path is only in target

    Similar(T, Option<DirDiffFileContent>),
    // path is both source and target; if Option is None, then either the path points to a directory
    // or file content checking is disabled
}

#[derive(Debug)]
enum DirDiffFileContent {
    Unchanged, // file contents are the same
    Changed,   // file contents are different
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

fn get_dir_diff(
    source_dir_listing: &Vec<std::path::PathBuf>,
    target_dir_listing: &Vec<std::path::PathBuf>,
    compare_file_contents: bool,
) -> Vec<DirDiff<std::path::PathBuf>> {
    // Return diff between two directories
    // NOTE: this function assumes both directory listings are sorted by unicode values

    println!("{:?}", source_dir_listing);
    println!("{:?}", target_dir_listing);

    let longest_listing = std::cmp::max(source_dir_listing.len(), target_dir_listing.len());

    // indexes for both vectors
    let mut source_index = 0;
    let mut target_index = 0;

    let mut diff_output = Vec::<DirDiff<std::path::PathBuf>>::new();

    // go through both arrays at the same time, to ensure O(n) time
    for _ in 0..longest_listing {
        if source_dir_listing[source_index] < target_dir_listing[target_index] {
            println!(
                "{:?} comes before {:?}",
                source_dir_listing[source_index], target_dir_listing[target_index]
            );

            diff_output.push(DirDiff::Removed(source_dir_listing[source_index].clone()));

            source_index += 1;
        } else if source_dir_listing[source_index] > target_dir_listing[target_index] {
            println!(
                "{:?} comes after {:?}",
                source_dir_listing[source_index], target_dir_listing[target_index]
            );

            diff_output.push(DirDiff::Added(target_dir_listing[target_index].clone()));

            target_index += 1;
        } else {
            println!(
                "{:?} is equal to {:?}",
                source_dir_listing[source_index], target_dir_listing[target_index]
            );

            // TODO: if file content checking is used, do it here
            diff_output.push(DirDiff::Similar(
                source_dir_listing[source_index].clone(),
                None, // TODO: no file content checking used yet
            ));

            source_index += 1;
            target_index += 1;
        }

        // add the remaining items to the dir diff if it reached the end of one dir listing
        if source_index >= source_dir_listing.len() {
            // add the remaining ADDED items of the other dir listing
            for i in target_index..target_dir_listing.len() {
                diff_output.push(DirDiff::Added(target_dir_listing[i].clone()));
            }

            break;
        } else if target_index >= target_dir_listing.len() {
            // add the remaining REMOVED items of the other dir listing
            for i in source_index..source_dir_listing.len() {
                diff_output.push(DirDiff::Removed(source_dir_listing[i].clone()));
            }

            break;
        }
    }

    diff_output
}

fn print_dir_diff(
    dir_diff: &Vec<DirDiff<std::path::PathBuf>>,
    hide_similarities: bool,
    color: bool,
) {
    for diff_fragment in dir_diff {
        match diff_fragment {
            DirDiff::Removed(path) => {
                if color {
                    println!("{} {}", "-".red(), path.to_str().unwrap().red());
                } else {
                    println!("- {}", path.to_str().unwrap());
                }
            }
            DirDiff::Added(path) => {
                if color {
                    println!("{} {}", "+".green(), path.to_str().unwrap().green());
                } else {
                    println!("+ {}", path.to_str().unwrap());
                }
            }
            DirDiff::Similar(path, _) => {
                // TODO: print differently if it is similar changed/unchanged
                if !hide_similarities {
                    println!(" {}", path.to_str().unwrap());
                }
            }
        }
    }
}

fn print_diff_summary(dir_diff: &Vec<DirDiff<std::path::PathBuf>>, hide_similarities: bool) {
    let mut num_removed = 0;
    let mut num_similar = 0;
    let mut num_added = 0;

    // TODO: count number of similar changed and similar unchanged as well
    for diff_fragment in dir_diff {
        match diff_fragment {
            DirDiff::Removed(_) => num_removed += 1,
            DirDiff::Added(_) => num_added += 1,
            DirDiff::Similar(_, _) => num_similar += 1,
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = CliArgs::parse();

    // error if directories do not exist
    check_cli_args(&args)?;

    // list both directories
    let source_dir_listing = get_dir_listing(&args.source_dir, args.depth);
    let target_dir_listing = get_dir_listing(&args.target_dir, args.depth);

    // get diff
    let dir_diff = get_dir_diff(
        &source_dir_listing,
        &target_dir_listing,
        args.compare_content,
    );

    print_dir_diff(&dir_diff, args.quiet, !args.no_color);
    print_diff_summary(&dir_diff, args.quiet);

    Ok(())
}
