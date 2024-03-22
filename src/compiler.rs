use std::{fs, process::{Command, Stdio}};

use serde::{Deserialize, Serialize};

use crate::{ast::{Element, MarkdownFile, MessageType, ZetaHeader}, print::zeta_error, Settings};

#[allow(non_snake_case)]
#[derive(Debug, Serialize, Deserialize)]
pub struct QiitaHeader {
    title: String,
    tags: Vec<String>,
    private: bool,
    updated_at: String,
    id: Option<String>,
    organization_url_name: Option<String>,
    slide: bool,
    ignorePublish: bool,
}

pub struct QiitaCompiler {
    existing_header: Option<QiitaHeader>,
}

impl QiitaCompiler {
    pub fn new(existing_header: Option<QiitaHeader>) -> Self {
        Self { existing_header }
    }

    pub fn compile(mut self, file: MarkdownFile) -> String {
        self.compile_header(file.header) + &self.compile_elements(file.elements)
    }

    fn compile_header(&mut self, header: ZetaHeader) -> String {
        let mut result = b"---\n".to_vec();

        let qiita_header = if let Some(existing_header) = &self.existing_header {
            let updated_at = existing_header.updated_at.clone();
            // let updated_at = format!("\'{}\'", updated_at);
            // let mut info = updated_at.split(": ")
            //     .collect::<Vec<&str>>();
            // info.insert(1, "\"");
            // info.push("\"");
            // let updated_at = info.join(": ");
            QiitaHeader {
                title: header.title,
                tags: header.topics,
                private: existing_header.private,
                updated_at,
                id: existing_header.id.clone(),
                organization_url_name: existing_header.organization_url_name.clone(),
                slide: existing_header.slide,
                ignorePublish: !header.publish,
            }
        } else {
            QiitaHeader {
                title: header.title,
                tags: header.topics,
                private: false,
                updated_at: "".to_string(),
                id: None,
                organization_url_name: None,
                slide: false,
                ignorePublish: !header.publish,
            }
        };
        let mut ser = serde_yaml::Serializer::new(&mut result);
        qiita_header.serialize(&mut ser).unwrap();

        result.extend(b"---\n");

        let result = String::from_utf8(result).unwrap();
        let mut lines: Vec<String> = result.split('\n').map(|s| s.to_string()).collect();
        let updated_at = lines.iter().position(|s| s.starts_with("updated_at:")).unwrap();
        let updated_at = lines.get_mut(updated_at).unwrap();

        if updated_at.ends_with('\"') || updated_at.ends_with('\'') {
            result
        } else {
            *updated_at = format!("updated_at: \'{}\'", &updated_at[12..]);

            lines.join("\n")
        }
    }

    fn compile_elements(&mut self, elements: Vec<Element>) -> String {
        let mut result = String::new();
        for element in elements {
            result += &self.compile_element(element);
        }

        result
    }

    fn compile_element(&mut self, element: Element) -> String {
        match element {
            Element::Text(text) => text,
            Element::Url(url) => format!("\n{}\n", url),
            Element::Macro(macro_info) => macro_info.qiita.unwrap_or_default(),
            Element::Image { alt, url } => {
                let url = if url.starts_with("/images") {
                    image_path_github(url.as_str())
                } else {
                    url
                };
                format!("![{}]({})", alt, url)
            }
            Element::InlineFootnote(name) => format!("[^{}]", name),
            Element::Message { msg_type, body } => {
                let msg_type = match msg_type {
                    MessageType::Info => "info",
                    MessageType::Warn => "warn",
                    MessageType::Alert => "alert",
                };

                let mut compiler = QiitaCompiler {
                    existing_header: None,
                };
                let body = compiler.compile_elements(body);

                format!(":::note {}\n{}:::", msg_type, body)
            }
            Element::Details { title, body } => {
                let mut compiler = QiitaCompiler {
                    existing_header: None,
                };
                let body = compiler.compile_elements(body);
                format!(
                    "<details><summary>{}</summary>\n\n{}</details>",
                    title, body
                )
            }
        }
    }
}

fn image_path_github(path: &str) -> String {
    let Ok(settings) = fs::read_to_string("./Zeta.toml") else {
        zeta_error("Failed to read Zeta.toml");
        return path.to_string();
    };
    let Ok(settings): Result<Settings, _> = toml::from_str(settings.as_str()) else {
        zeta_error("Failed to parse Zeta.toml");
        return path.to_string();
    };
    let repository = settings.repository;

    let mut remote = Command::new("git")
        .args(["remote", "show", "origin"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    if !remote.wait().unwrap().success() {
        zeta_error("Failed to get remote origin");
        return path.to_string();
    }

    let grep = Command::new("grep")
        .arg("HEAD branch")
        .stdin(Stdio::from(remote.stdout.unwrap()))
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let awk = Command::new("awk")
        .arg("{print $NF}")
        .stdin(Stdio::from(grep.stdout.unwrap()))
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut main_branch = String::from_utf8(awk.wait_with_output().unwrap().stdout).unwrap();


    if main_branch.is_empty() {
        zeta_error("Failed to get main branch");
        return path.to_string();
    }

    main_branch.pop(); // \n

    format!("https://raw.githubusercontent.com/{}/{}{}", repository, main_branch, path)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ZennHeader {
    title: String,
    emoji: String,
    r#type: String,
    topics: Vec<String>,
    published: bool,
}

pub struct ZennCompiler;

impl ZennCompiler {
    pub fn new() -> Self {
        Self {}
    }

    pub fn compile(mut self, file: MarkdownFile) -> String {
        self.compile_header(file.header) + &self.compile_elements(file.elements)
    }

    fn compile_header(&mut self, header: ZetaHeader) -> String {
        let mut result = b"---\n".to_vec();
        let zenn_header = ZennHeader {
            title: header.title,
            emoji: header.emoji,
            r#type: "tech".to_string(),
            topics: header.topics,
            published: header.publish,
        };
        let mut ser = serde_yaml::Serializer::new(&mut result);
        zenn_header.serialize(&mut ser).unwrap();
        result.extend(b"---\n");
        String::from_utf8(result).unwrap()
    }

    fn compile_elements(&mut self, elements: Vec<Element>) -> String {
        let mut result = String::new();
        for element in elements {
            result += &self.compile_element(element);
        }

        result
    }

    fn compile_element(&mut self, element: Element) -> String {
        match element {
            Element::Text(text) => text,
            Element::Url(url) => format!("\n{}\n", url),
            Element::Macro(macro_info) => macro_info.zenn.unwrap_or_default(),
            Element::Image { alt, url } => {
                format!("![{}]({})", alt, url)
            }
            Element::InlineFootnote(name) => format!("^[{}]", name),
            Element::Message { msg_type, body } => {
                let msg_type = match msg_type {
                    MessageType::Info => "",
                    MessageType::Warn => "",
                    MessageType::Alert => "alert",
                };

                let mut compiler = ZennCompiler {};
                let body = compiler.compile_elements(body);

                format!(":::message {}\n{}:::", msg_type, body)
            }
            Element::Details { title, body } => {
                let mut compiler = ZennCompiler {};
                let body = compiler.compile_elements(body);
                format!(":::details {}\n{}:::", title, body)
            }
        }
    }
}
