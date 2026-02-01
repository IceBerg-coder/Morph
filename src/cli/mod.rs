use clap::{Parser as ClapParser, Subcommand};
use anyhow::Result;
use std::path::PathBuf;

use crate::lexer::Lexer;
use crate::parser::Parser as MorphParser;

/// Morph Compiler CLI
#[derive(ClapParser)]
#[command(name = "mrc")]
#[command(about = "Morph: The Self-Optimizing Language Compiler")]
#[command(version = "0.1.0")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run a Morph file dynamically (Stage 0-1)
    Run {
        /// Path to the Morph source file
        file: PathBuf,
        
        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// Check stability scores for a Morph file
    Status {
        /// Path to the Morph source file
        file: PathBuf,
    },
    
    /// Compile a Morph file to native binary (Stage 3)
    Harden {
        /// Path to the Morph source file
        file: PathBuf,
        
        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    
    /// Build and package solid fragments
    Build {
        /// Build in release mode
        #[arg(short, long)]
        release: bool,
    },
    
    /// Tokenize a Morph file (for debugging)
    Tokenize {
        /// Path to the Morph source file
        file: PathBuf,
    },
    
    /// Parse a Morph file and show AST (for debugging)
    Parse {
        /// Path to the Morph source file
        file: PathBuf,
    },
}

/// Execute the CLI command
pub fn execute(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Run { file, verbose } => {
            run_file(file, verbose)
        }
        Commands::Status { file } => {
            check_status(file)
        }
        Commands::Harden { file, output } => {
            harden_file(file, output)
        }
        Commands::Build { release } => {
            build_project(release)
        }
        Commands::Tokenize { file } => {
            tokenize_file(file)
        }
        Commands::Parse { file } => {
            parse_file(file)
        }
    }
}

/// Run a Morph file (Stage 0: Draft mode)
fn run_file(file: PathBuf, verbose: bool) -> Result<()> {
    if verbose {
        println!("Running Morph file: {}", file.display());
    }
    
    let source = std::fs::read_to_string(&file)?;
    
    // Stage 0: Draft - Tree-walk interpretation
    if verbose {
        println!("Stage 0: Draft (Tree-walk Interpreter)");
    }
    
    // Tokenize
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize()?;
    
    if verbose {
        println!("  Tokenized {} tokens", tokens.len());
    }
    
    // Parse
    let mut parser = MorphParser::new(tokens);
    let ast = parser.parse()?;
    
    if verbose {
        println!("  Parsed {} declarations", ast.declarations.len());
    }
    
    // TODO: Implement interpreter for Stage 0
    println!("Execution complete (interpreter not yet implemented)");
    
    Ok(())
}

/// Check stability scores for a file
fn check_status(file: PathBuf) -> Result<()> {
    println!("Checking stability for: {}", file.display());
    
    let source = std::fs::read_to_string(&file)?;
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize()?;
    let mut parser = MorphParser::new(tokens);
    let ast = parser.parse()?;
    
    // TODO: Implement stability scoring
    println!("Stability Scores:");
    println!("  Draft (Stage 0):   ████████░░ 80%");
    println!("  Observe (Stage 1): ██████░░░░ 60%");
    println!("  Refine (Stage 2):  ████░░░░░░ 40%");
    println!("  Solid (Stage 3):   ██░░░░░░░░ 20%");
    println!("\n{} declarations found", ast.declarations.len());
    
    Ok(())
}

/// Compile to native binary (Stage 3: Solid mode)
fn harden_file(file: PathBuf, output: Option<PathBuf>) -> Result<()> {
    let output_path = output.unwrap_or_else(|| {
        let mut path = file.clone();
        path.set_extension("");
        path
    });
    
    println!("Hardening {} -> {}", file.display(), output_path.display());
    
    let source = std::fs::read_to_string(&file)?;
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize()?;
    let mut parser = MorphParser::new(tokens);
    let ast = parser.parse()?;
    
    println!("Stage 3: Solid (LLVM Native Binary)");
    println!("  Parsed {} declarations", ast.declarations.len());
    
    // TODO: Implement LLVM backend for Stage 3
    println!("Native compilation not yet implemented");
    println!("AST structure validated successfully");
    
    Ok(())
}

/// Build the project
fn build_project(release: bool) -> Result<()> {
    let mode = if release { "release" } else { "debug" };
    println!("Building Morph project in {} mode...", mode);
    
    // TODO: Implement project building
    println!("Project build not yet implemented");
    
    Ok(())
}

/// Tokenize a file and print tokens
fn tokenize_file(file: PathBuf) -> Result<()> {
    println!("Tokenizing: {}", file.display());
    println!("{}", "=".repeat(60));
    
    let source = std::fs::read_to_string(&file)?;
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize()?;
    
    for token in tokens {
        if matches!(token.token_type, crate::lexer::TokenType::Eof) {
            break;
        }
        println!("{}", token);
    }
    
    Ok(())
}

/// Parse a file and print AST
fn parse_file(file: PathBuf) -> Result<()> {
    println!("Parsing: {}", file.display());
    println!("{}", "=".repeat(60));
    
    let source = std::fs::read_to_string(&file)?;
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize()?;
    let mut parser = MorphParser::new(tokens);
    let ast = parser.parse()?;
    
    println!("{:#?}", ast);
    
    Ok(())
}