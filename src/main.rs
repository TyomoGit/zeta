use ast::ZetaHeader;
use clap::{command, Parser};
use compiler::{QiitaCompiler, QiitaHeader, ZennCompiler};
use print::zeta_error;
use serde::Deserialize;
use std::{collections::HashMap, fs::{self, DirBuilder}, io::Write, process::Command};

use crate::print::zeta_message;

mod ast;
mod parser;
mod print;
mod compiler;

#[derive(Debug, Clone, clap::Parser)]
#[command(version, about)]
struct Args {
    mode: String,
    target: Option<String>,
}

fn main() {
    let args = Args::parse();
    let mode = args.mode.as_str();
    match mode {
        "init" => init(),

        "new" => {
            let Some(target) = args.target else {
                zeta_error("Target is required");
                return;
            };
            new(target.as_str());
        }

        "build" => {
            let Some(target) = args.target else {
                zeta_error("Target is required");
                return;
            };
            build(target.as_str());
        }

        _ => {
            zeta_error(
                format!("Unknown mode: {}", mode).as_str()
            );
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
struct Settings {
    repository: String,
    macros: Option<HashMap<String, (String, String)>>
}

fn init() {
    zeta_message("Zeta init");

    print!("GitHub Repository(User/Repo): ");
    std::io::stdout().flush().unwrap();
    let mut repository = String::new();
    std::io::stdin().read_line(&mut repository).unwrap();
    repository = repository.trim().to_string();

    let settings = Settings { repository, macros: None };

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
    let output =Command::new("npm")
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

    zeta_message("Done!");
}

fn new(target: &str) {
    let Ok(file) = fs::File::create(format!("zeta/{}.md", target)) else {
        zeta_error("Target already exists");
        return;
    };

    let mut file = std::io::BufWriter::new(file);
    file.write_all(
        include_str!("zeta_templete.txt").as_bytes()
    ).unwrap();
}

fn build(target: &str) {
    let Ok(file) = fs::read_to_string(format!("zeta/{}.md", target)) else {
        zeta_error("Target not found");
        return;
    };
    
    let parser = parser::Parser::new(file.chars().collect());
    let file = parser.parse_file();

    let existing_header = if let Ok(existing_file) = fs::read_to_string(format!("public/{}.md", target)) {
        let existing_file = &existing_file[4..];
        let end = existing_file.find("---").unwrap();
        let existing_file = &existing_file[..end];
        let de =serde_yaml::Deserializer::from_str(existing_file);
        Some(QiitaHeader::deserialize(de).unwrap())
    } else {
        None
    };

    let compiler = QiitaCompiler::new(existing_header);
    let qiita_md = compiler.compile(file.clone());

    DirBuilder::new().recursive(true).create("public").unwrap();
    fs::write(format!("public/{}.md", target), qiita_md).unwrap();

    // /////////////////////
    let compiler = ZennCompiler::new();
    let zenn_md = compiler.compile(file);
    fs::write(format!("articles/{}.md", target), zenn_md).unwrap();
}
