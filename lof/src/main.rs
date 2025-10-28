fn main() {
    if let Err(err) = lof::cli::run_cli() {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
}
