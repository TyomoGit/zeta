use ast::{MarkdownFile, Platform, ZetaFrontmatter};
use clap::{command, Parser, Subcommand};
use compiler::{QiitaCompiler, QiitaFrontmatter, ZennCompiler};
use print::{zeta_error, zeta_error_position};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, DirBuilder},
    io::Write,
    process::Command,
};

use crate::print::zeta_message;

mod ast;
mod compiler;
mod parser;
mod print;

#[derive(Debug, Clone, clap::Parser)]
#[command(version, about)]
struct Cli {
    /// Subcommand
    #[command(subcommand)]
    command: ZetaCommand,
}

#[derive(Debug, Clone, Subcommand)]
enum ZetaCommand {
    /// Initialize Zeta
    Init,
    /// Create new article
    New {
        target: String,
        #[arg(long)]
        only: Option<Platform>,
    },
    /// Build article
    Build { target: String },
    /// Rename article
    Rename { target: String, new_name: String },
    /// Remove article
    Remove { target: String },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        ZetaCommand::Init => init(),
        ZetaCommand::New { target, only } => new(&target, &only),
        ZetaCommand::Build { target } => build(&target),
        ZetaCommand::Rename { target, new_name } => rename(&target, &new_name),
        ZetaCommand::Remove { target } => remove(&target),
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Settings {
    repository: String,
}

fn init() {
    zeta_message("Zeta init");

    print!("GitHub Repository(User/Repo): ");
    std::io::stdout().flush().unwrap();
    let mut repository = String::new();
    std::io::stdin().read_line(&mut repository).unwrap();
    repository = repository.trim().to_string();

    let settings = Settings { repository };

    zeta_message("Creating Zeta.toml...");
    fs::File::create("Zeta.toml")
        .unwrap()
        .write_all(toml::to_string(&settings).unwrap().as_bytes())
        .unwrap();

    zeta_message("Initializing NPM...");
    let output = Command::new("npm").args(["init", "-y"]).output().unwrap();
    println!("{}", String::from_utf8_lossy(&output.stdout));

    zeta_message("Installing Zenn CLI...");
    let output = Command::new("npm")
        .args(["install", "zenn-cli", "--save-dev"])
        .output()
        .unwrap();
    println!("{}", String::from_utf8_lossy(&output.stdout));

    zeta_message("Installing Qiita CLI...");
    let output = Command::new("npm")
        .args(["install", "@qiita/qiita-cli", "--save-dev"])
        .output()
        .unwrap();
    println!("{}", String::from_utf8_lossy(&output.stdout));

    zeta_message("Initializing Zenn...");
    let output = Command::new("npx").args(["zenn", "init"]).output().unwrap();
    println!("{}", String::from_utf8_lossy(&output.stdout));

    zeta_message("Initializing Qiita...");
    let output = Command::new("npx")
        .args(["qiita", "init"])
        .output()
        .unwrap();
    println!("{}", String::from_utf8_lossy(&output.stdout));

    zeta_message("Creating images directory...");
    fs::DirBuilder::new().create("images").unwrap();

    zeta_message("Creating zeta directory...");
    fs::DirBuilder::new().create("zeta").unwrap();

    zeta_message("Initializing git...");
    let output = Command::new("git").arg("init").output().unwrap();
    println!("{}", String::from_utf8_lossy(&output.stdout));

    zeta_message("Creating .gitignore...");
    let mut file = fs::File::create(".gitignore").unwrap();
    file.write_all(include_str!("gitignore.txt").as_bytes())
        .unwrap();

    zeta_message("Done!");
}

fn new(target: &str, only: &Option<Platform>) {
    let _ = fs::DirBuilder::new()
        .recursive(true)
        .create(format!("images/{}", target));

    let Ok(file) = fs::File::create(format!("zeta/{}.md", target)) else {
        zeta_error("Target already exists");
        return;
    };

    let mut file = std::io::BufWriter::new(file);
    let frontmatter = ZetaFrontmatter {
        title: "".to_string(),
        emoji: "😀".to_string(),
        r#type: "tech".to_string(),
        topics: vec![],
        published: false,
        only: only.clone(),
    };
    file.write_all(b"---\n").unwrap();
    let mut serializer = serde_yaml::Serializer::new(&mut file);
    frontmatter.serialize(&mut serializer).unwrap();
    file.write_all(b"---\n").unwrap();
}

fn build(target: &str) {
    let Ok(file) = fs::read_to_string(format!("zeta/{}.md", target)) else {
        zeta_error("Target not found");
        return;
    };

    let parser = parser::Parser::new(file.chars().collect());
    let result = parser.parse_file();
    let Ok(file) = result else {
        result.unwrap_err().iter().for_each(|error| {
            zeta_error_position(&error.error_type.to_string(), error.row, error.col);
        });
        return;
    };

    if let Some(platform) = &file.frontmatter.only {
        match platform {
            ast::Platform::Zenn => compile_zenn(file, target),
            ast::Platform::Qiita => compile_qiita(file, target),
        }
    } else {
        compile_zenn(file.clone(), target);
        compile_qiita(file, target);
    }
}

fn compile_zenn(file: MarkdownFile, target: &str) {
    let compiler = ZennCompiler::new();
    let zenn_md = compiler.compile(file);
    fs::write(format!("articles/{}.md", target), zenn_md).unwrap();
}

fn compile_qiita(file: MarkdownFile, target: &str) {
    let existing_header =
        if let Ok(existing_file) = fs::read_to_string(format!("public/{}.md", target)) {
            let existing_file = &existing_file[4..];
            let end = existing_file.find("---").unwrap();
            let existing_file = &existing_file[..end];
            let de = serde_yaml::Deserializer::from_str(existing_file);
            Some(QiitaFrontmatter::deserialize(de).unwrap())
        } else {
            None
        };

    let compiler = QiitaCompiler::new(existing_header);
    let qiita_md = compiler.compile(file.clone());

    DirBuilder::new().recursive(true).create("public").unwrap();
    fs::write(format!("public/{}.md", target), qiita_md).unwrap();
}

fn rename(target: &str, new_name: &str) {
    fs::rename(
        format!("zeta/{}.md", target),
        format!("zeta/{}.md", new_name),
    )
    .unwrap();

    if fs::File::open(format!("public/{}.md", target)).is_ok() {
        fs::rename(
            format!("public/{}.md", target),
            format!("public/{}.md", new_name),
        )
        .unwrap();
    }

    if fs::File::open(format!("articles/{}.md", target)).is_ok() {
        fs::rename(
            format!("articles/{}.md", target),
            format!("articles/{}.md", new_name),
        )
        .unwrap();
    }
}

fn remove(target: &str) {
    let _ = fs::remove_file(format!("zeta/{}.md", target));
    let _ = fs::remove_file(format!("articles/{}.md", target));
    let _ = fs::remove_file(format!("public/{}.md", target));
}
