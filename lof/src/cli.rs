use crate::lexer::Lexer;
use crate::parser::Parser as LofParser;
use crate::pipeline::{CompilerError, CompilerPipeline};
use clap::{Parser, Subcommand, ValueEnum};
use colored::*;
use lofit::ConstraintSystem;
use serde_json::{json, to_string_pretty};
use std::collections::BTreeMap;
use std::fs;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};

struct ProjectPaths {
    base: PathBuf,
    build_dir: PathBuf,
    keys_dir: PathBuf,
    inputs_dir: PathBuf,
    proofs_dir: PathBuf,
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(name = "lof")]
#[command(version = VERSION)]
#[command(about = "Lof language compiler for ZK circuit verification", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, ValueEnum)]
enum Target {
    R1cs,
    Wasm,
}

#[derive(Subcommand)]
enum Commands {
    Check {
        #[arg(value_name = "FILE")]
        #[arg(long)]
        skip_wasm: bool,
        file: PathBuf,

        #[arg(short, long)]
        verbose: bool,
    },
    Compile {
        #[arg(value_name = "FILE")]
        #[arg(long)]
        skip_wasm: bool,
        file: PathBuf,

        #[arg(short, long)]
        verbose: bool,

        /// Target: r1cs (default) or wasm. Use --target wasm for WebAssembly
        #[arg(short, long, value_enum, default_value = "r1cs")]
        target: Target,

        #[arg(short, long)]
        output: Option<PathBuf>,

        #[arg(short = 'g', long)]
        generate_templates: bool,
    },
    Parse {
        #[arg(value_name = "FILE")]
        file: PathBuf,

        #[arg(short, long)]
        verbose: bool,

        #[arg(short, long)]
        pretty: bool,
    },
    Version,
}

pub fn run_cli() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    let _ = tracing_subscriber::fmt::try_init();

    run()
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Version => {
            println!("{}", VERSION);
            Ok(())
        }
        Commands::Check { file, verbose, .. } => handle_check(file, verbose),
        Commands::Compile {
            file,
            verbose,
            target,
            output,
            generate_templates,
            skip_wasm,
        } => handle_compile(file, verbose, target, output, generate_templates, skip_wasm),
        Commands::Parse {
            file,
            verbose,
            pretty,
            ..
        } => handle_parse(file, verbose, pretty),
    }
}

fn ensure_lof_extension(file: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if file.extension().and_then(|ext| ext.to_str()) == Some("lof") {
        Ok(())
    } else {
        let err_msg = "File must have .lof extension";
        error!("{}", err_msg);
        Err(err_msg.into())
    }
}

fn handle_check(file: PathBuf, verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    ensure_lof_extension(&file)?;

    info!("Type checking file: {}", file.display());
    println!("{} {}", "Type checking".blue(), file.display());
    let source = fs::read_to_string(&file)?;

    let pipeline = CompilerPipeline::new(source, verbose);

    if verbose {
        debug!("Starting type checking in verbose mode");
        println!("{}", "Starting type checking...".yellow());
    }

    match pipeline.type_check_only(file.as_path()) {
        Ok(_) => {
            info!("Type checking completed successfully");
            println!("{}", "Type checking successful".green());
            Ok(())
        }
        Err(err) => match err {
            CompilerError::LexerError(e) => {
                error!("Lexer error: {}", e);
                Err(format!("Lexer error: {}", e).into())
            }
            CompilerError::ParserError(e) => {
                error!("Parser error: {}", e);
                Err(format!("Parser error: {}", e).into())
            }
            CompilerError::TypeCheckerError(e) => {
                error!("Type error: {:?}", e);
                Err(format!("Type error: {:?}", e).into())
            }
            CompilerError::NoProofs => {
                error!("No proofs found in the source file");
                Err("No proofs found in the source file".into())
            }
            CompilerError::R1CSError => {
                error!("R1CS generation failed");
                Err("R1CS error".into())
            }
            CompilerError::IRError(e) => {
                error!("IR generation failed: {}", e);
                Err(format!("IR error: {}", e).into())
            }
        },
    }
}

fn handle_compile(
    file: PathBuf,
    verbose: bool,
    target: Target,
    output: Option<PathBuf>,
    generate_templates: bool,
    skip_wasm: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    ensure_lof_extension(&file)?;

    match target {
        Target::R1cs => compile_r1cs(file.as_path(), verbose, generate_templates, None).map(|_| ()),
        Target::Wasm => compile_wasm(file.as_path(), verbose, output.as_deref(), skip_wasm),
    }
}

fn handle_parse(
    file: PathBuf,
    verbose: bool,
    pretty: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    ensure_lof_extension(&file)?;

    info!("Parsing file: {}", file.display());
    println!("{} {}", "Parsing".blue(), file.display());

    let source = fs::read_to_string(&file)?;

    if verbose {
        println!("\n{}", "Source code:".yellow());
        println!("{}", source);
        println!("\n{}", "--- Lexing & Parsing ---".yellow());
    }

    let lexer = Lexer::new(&source);
    let mut parser = LofParser::new(lexer);

    match parser.parse_program() {
        Ok(ast) => {
            info!("Parsing completed successfully");
            println!("{}", "✅ Parsing successful!".green());

            println!("\n{}", "AST:".cyan());
            if pretty {
                println!("{:#?}", ast);
            } else {
                println!("{:?}", ast);
            }

            Ok(())
        }
        Err(e) => {
            error!("Parser error: {}", e);
            println!("{} {}", "❌ Parse error:".red(), e);
            Err(format!("Parser error: {}", e).into())
        }
    }
}

fn compile_r1cs(
    file: &Path,
    verbose: bool,
    generate_templates: bool,
    output_root: Option<&Path>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let base_name = file
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| format!("Unable to determine base name for '{}'", file.display()))?;
    let paths = prepare_project_paths(file, output_root)?;

    run_compiler_pipeline(file, verbose)?;
    handle_compilation_artifacts(file, base_name, &paths, generate_templates)?;

    Ok(paths.base)
}

fn compile_wasm(
    file: &Path,
    verbose: bool,
    output: Option<&Path>,
    skip_wasm: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if verbose {
        println!("{}", "WASM target selected".yellow());
    }

    // Step 1: compile to R1CS/IR into a temporary workspace
    let temp_dir = tempfile::tempdir()?;
    let temp_output = compile_r1cs(file, verbose, false, Some(temp_dir.path()))?;

    let base_name = file
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("Invalid file name")?;
    let build_dir = temp_output.join("build");
    let r1cs_path = build_dir.join(format!("{}.r1cs", base_name));

    if !r1cs_path.exists() {
        let err_msg = format!(
            "R1CS artifact missing at {} after compilation",
            r1cs_path.display()
        );
        error!("{}", err_msg);
        return Err(err_msg.into());
    }

    println!();
    println!(
        "{} {}",
        "Packaging circuit for browser proving:".blue(),
        r1cs_path.display()
    );

    // Step 2: reuse lofit's packaging helper to build witness/prover assets
    let package_dir = lofit::package_for_web(&r1cs_path, output, skip_wasm)?;

    println!(
        "{} {}",
        "Web-ready assets generated in".green(),
        package_dir.display()
    );
    println!(
        "{}",
        "Use `python3 -m http.server` (or similar) inside the package directory to try it.".cyan()
    );

    Ok(())
}

fn prepare_project_paths(
    file: &Path,
    output_root: Option<&Path>,
) -> Result<ProjectPaths, Box<dyn std::error::Error>> {
    let file_dir = file.parent().unwrap_or_else(|| Path::new("."));
    let base = output_root
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| file_dir.to_path_buf());

    let paths = ProjectPaths {
        build_dir: base.join("build"),
        keys_dir: base.join("keys"),
        inputs_dir: base.join("inputs"),
        proofs_dir: base.join("proofs"),
        base: base.clone(),
    };

    for dir in [
        &paths.build_dir,
        &paths.keys_dir,
        &paths.inputs_dir,
        &paths.proofs_dir,
    ] {
        fs::create_dir_all(dir)?;
    }

    println!(
        "{} {}",
        "Created project directories under".cyan(),
        paths.base.display()
    );

    Ok(paths)
}

fn run_compiler_pipeline(file: &Path, verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    info!("Processing file: {}", file.display());
    println!("{} {}", "Processing".blue(), file.display());

    let source = fs::read_to_string(file)?;
    let pipeline = CompilerPipeline::new(source, verbose);

    if verbose {
        debug!("Starting compilation pipeline in verbose mode");
        println!("{}", "Starting compilation pipeline...".yellow());
    }

    pipeline.run(file).map_err(map_compiler_error)?;

    info!("Compilation completed successfully");
    println!("{}", "Compilation successful".green());

    Ok(())
}

fn map_compiler_error(err: CompilerError) -> Box<dyn std::error::Error> {
    match err {
        CompilerError::LexerError(e) => {
            error!("Lexer error: {}", e);
            format!("Lexer error: {}", e).into()
        }
        CompilerError::ParserError(e) => {
            error!("Parser error: {}", e);
            format!("Parser error: {}", e).into()
        }
        CompilerError::TypeCheckerError(e) => {
            error!("Type error: {:?}", e);
            format!("Type error: {:?}", e).into()
        }
        CompilerError::NoProofs => {
            error!("No proofs found in the source file");
            "No proofs found in the source file".into()
        }
        CompilerError::R1CSError => {
            error!("R1CS generation failed");
            "R1CS error".into()
        }
        CompilerError::IRError(e) => {
            error!("IR generation failed: {}", e);
            format!("IR error: {}", e).into()
        }
    }
}

fn handle_compilation_artifacts(
    source_file: &Path,
    base_name: &str,
    paths: &ProjectPaths,
    generate_templates: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let r1cs_file = source_file.with_extension("r1cs");
    if !r1cs_file.exists() {
        warn!("R1CS file not found, compilation may have failed");
        println!("{} R1CS file not generated", "Warning:".yellow());
        return Ok(());
    }

    let build_r1cs = paths.build_dir.join(format!("{}.r1cs", base_name));
    copy_artifact(&r1cs_file, &build_r1cs, "Generated R1CS")?;

    let ir_file = source_file.with_extension("ir");
    if ir_file.exists() {
        let build_ir = paths.build_dir.join(format!("{}.ir", base_name));
        copy_artifact(&ir_file, &build_ir, "Generated IR")?;
    } else {
        warn!("IR file not found after compilation");
        println!("{} IR file not generated", "Warning:".yellow());
    }

    if generate_templates {
        let (public_inputs, witness_inputs) = load_signal_names(&build_r1cs)?;
        info!("Generating JSON templates for proof: {}", base_name);
        generate_json_templates(&paths.base, base_name, &public_inputs, &witness_inputs)?;
        print_next_steps(&paths.base, base_name)?;
    }

    Ok(())
}

fn copy_artifact(src: &Path, dest: &Path, label: &str) -> Result<(), Box<dyn std::error::Error>> {
    fs::copy(src, dest)?;
    fs::remove_file(src)?;
    println!("{} {}", label.green(), dest.display());
    Ok(())
}

fn load_signal_names(
    r1cs_path: &Path,
) -> Result<(Vec<String>, Vec<String>), Box<dyn std::error::Error>> {
    let file = fs::File::open(r1cs_path)?;
    let reader = BufReader::new(file);
    let r1cs = ConstraintSystem::from_file(reader)?;
    Ok((r1cs.public_inputs.clone(), r1cs.witnesses.clone()))
}

fn generate_json_templates(
    base_dir: &Path,
    proof_name: &str,
    public_inputs: &[String],
    witnesses: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    write_template_file(
        base_dir,
        proof_name,
        "public",
        public_inputs,
        "Generated public inputs template",
    )?;

    write_template_file(
        base_dir,
        proof_name,
        "witness",
        witnesses,
        "Generated witness template",
    )?;

    Ok(())
}

fn write_template_file(
    base_dir: &Path,
    proof_name: &str,
    template_type: &str,
    signals: &[String],
    label: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut placeholders = BTreeMap::new();
    for name in signals {
        placeholders.insert(name.clone(), "0".to_string());
    }

    let json_body = json!(placeholders);
    let file_path = base_dir
        .join("inputs")
        .join(format!("{}_{}.json", proof_name, template_type));

    let mut file = fs::File::create(&file_path)?;
    file.write_all(to_string_pretty(&json_body)?.as_bytes())?;

    info!("{}: {}", label, file_path.display());
    println!("{} {}", label.green(), file_path.display());

    Ok(())
}

fn print_next_steps(base_dir: &Path, proof_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{}", "Next steps:".bold().green());

    let paths = NextStepPaths::new(base_dir, proof_name);

    println!("1. Generate keys:");
    println!(
        "   {}",
        format!(
            "lofit setup --input {} --proving-key {} --verification-key {}",
            paths.r1cs.display(),
            paths.proving_key.display(),
            paths.verification_key.display()
        )
        .cyan()
    );

    println!("\n2. Edit your input files:");
    println!(
        "   {} - Edit with your public input values",
        paths.public_inputs.display().to_string().yellow()
    );
    println!(
        "   {} - Edit with your witness values",
        paths.witness_inputs.display().to_string().yellow()
    );

    println!("\n3. Generate proof:");
    println!(
        "   {}",
        format!(
            "lofit prove --input {} --proving-key {} --public-inputs {} --witness {} --output {}",
            paths.r1cs.display(),
            paths.proving_key.display(),
            paths.public_inputs.display(),
            paths.witness_inputs.display(),
            paths.proof_file.display()
        )
        .cyan()
    );

    println!("\n4. Verify proof:");
    println!(
        "   {}",
        format!(
            "lofit verify --verification-key {} --proof {} --public-inputs {}",
            paths.verification_key.display(),
            paths.proof_file.display(),
            paths.public_inputs.display()
        )
        .cyan()
    );

    Ok(())
}

struct NextStepPaths {
    r1cs: PathBuf,
    proving_key: PathBuf,
    verification_key: PathBuf,
    public_inputs: PathBuf,
    witness_inputs: PathBuf,
    proof_file: PathBuf,
}

impl NextStepPaths {
    fn new(base_dir: &Path, proof_name: &str) -> Self {
        Self {
            r1cs: base_dir.join("build").join(format!("{}.r1cs", proof_name)),
            proving_key: base_dir.join("keys").join(format!("{}_pk.bin", proof_name)),
            verification_key: base_dir.join("keys").join(format!("{}_vk.bin", proof_name)),
            public_inputs: base_dir
                .join("inputs")
                .join(format!("{}_public.json", proof_name)),
            witness_inputs: base_dir
                .join("inputs")
                .join(format!("{}_witness.json", proof_name)),
            proof_file: base_dir
                .join("proofs")
                .join(format!("{}_proof.bin", proof_name)),
        }
    }
}
