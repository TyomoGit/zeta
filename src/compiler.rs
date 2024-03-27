use std::{
    collections::{HashMap, HashSet},
    fs,
    process::{Command, Stdio},
};

use serde::{Deserialize, Serialize};

use crate::{
    ast::{Element, MessageType, ParsedMd, ZetaFrontmatter},
    print::zeta_error,
    Settings,
};

#[allow(non_snake_case)]
#[derive(Debug, Serialize, Deserialize)]
pub struct QiitaFrontmatter {
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
    existing_fm: Option<QiitaFrontmatter>,
    footnotes: HashSet<String>,
    inline_footnotes: HashMap<String, String>,
}

impl QiitaCompiler {
    pub fn new(existing_header: Option<QiitaFrontmatter>) -> Self {
        Self {
            existing_fm: existing_header,
            footnotes: HashSet::new(),
            inline_footnotes: HashMap::new(),
        }
    }

    pub fn compile(mut self, file: ParsedMd) -> String {
        self.compile_frontmatter(file.frontmatter) + &self.compile_elements(file.elements)
    }

    fn compile_frontmatter(&mut self, frontmatter: ZetaFrontmatter) -> String {
        let mut result = b"---\n".to_vec();

        let frontmatter = if let Some(existing_fm) = &self.existing_fm {
            QiitaFrontmatter {
                title: frontmatter.title,
                tags: frontmatter.topics,
                private: existing_fm.private,
                updated_at: existing_fm.updated_at.clone(),
                id: existing_fm.id.clone(),
                organization_url_name: existing_fm.organization_url_name.clone(),
                slide: existing_fm.slide,
                ignorePublish: !frontmatter.published,
            }
        } else {
            QiitaFrontmatter {
                title: frontmatter.title,
                tags: frontmatter.topics,
                private: false,
                updated_at: "".to_string(),
                id: None,
                organization_url_name: None,
                slide: false,
                ignorePublish: !frontmatter.published,
            }
        };
        let mut ser = serde_yaml::Serializer::new(&mut result);
        frontmatter.serialize(&mut ser).unwrap();

        result.extend(b"---\n");

        let result = String::from_utf8(result).unwrap();
        let mut lines: Vec<String> = result.split('\n').map(|s| s.to_string()).collect();
        let updated_at = lines
            .iter()
            .position(|s| s.starts_with("updated_at:"))
            .unwrap();
        let updated_at = lines.get_mut(updated_at).unwrap();

        if updated_at.ends_with('\"') || updated_at.ends_with('\'') {
            result
        } else {
            *updated_at = format!("updated_at: \'{}\'", &updated_at[12..]);

            lines.join("\n")
        }
    }

    fn compile_elements(&mut self, elements: Vec<Element>) -> String {
        let mut result: String = elements
            .into_iter()
            .map(|element| self.compile_element(element))
            .collect();

        for (name, content) in &self.inline_footnotes {
            result.push_str(&format!("\n[^{}]: {}\n", name, content));
        }

        result
    }

    fn compile_element(&mut self, element: Element) -> String {
        match element {
            Element::Text(text) => text,
            Element::Url(url) => format!("\n{}\n", url),
            Element::Macro(macro_info) => self.compile_elements(macro_info.qiita),
            Element::Image { alt, url } => {
                let url = if url.starts_with("/images") {
                    image_path_github(url.as_str())
                } else {
                    url
                };
                format!("![{}]({})", alt, url)
            }
            Element::InlineFootnote(content) => {
                let mut i: usize = 1;
                let name = loop {
                    let name = format!("zeta.inline.{}", i);
                    if !self.inline_footnotes.contains_key(&name) {
                        break name;
                    }
                    i += 1;
                };

                self.inline_footnotes.insert(name.clone(), content);

                format!("[^{}]", name)
            }
            Element::Footnote(name) => {
                let result = format!("[^{}]", &name);
                self.footnotes.insert(name);
                result
            }
            Element::Message {
                level: _,
                msg_type,
                body,
            } => {
                let msg_type = match msg_type {
                    MessageType::Info => "info",
                    MessageType::Warn => "warn",
                    MessageType::Alert => "alert",
                };

                let mut compiler = QiitaCompiler::new(None);
                let body = compiler.compile_elements(body);

                format!(":::note {}\n{}:::", msg_type, body)
            }
            Element::Details {
                level: _,
                title,
                body,
            } => {
                let mut compiler = QiitaCompiler::new(None);
                let body = compiler.compile_elements(body);
                format!(
                    "<details><summary>{}</summary>\n\n{}</details>\n",
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

    let mut main_branch = String::from_utf8(grep.wait_with_output().unwrap().stdout)
        .unwrap()
        .split(' ')
        .last()
        .unwrap()
        .to_string();

    if main_branch.is_empty() {
        zeta_error("Failed to get main branch");
        return path.to_string();
    }

    main_branch.pop(); // \n

    format!(
        "https://raw.githubusercontent.com/{}/{}{}",
        repository, main_branch, path
    )
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ZennFrontmatter {
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

    pub fn compile(mut self, file: ParsedMd) -> String {
        self.compile_header(file.frontmatter) + &self.compile_elements(file.elements)
    }

    fn compile_header(&mut self, frontmatter: ZetaFrontmatter) -> String {
        let mut result = b"---\n".to_vec();
        let frontmatter = ZennFrontmatter {
            title: frontmatter.title,
            emoji: frontmatter.emoji,
            r#type: frontmatter.r#type,
            topics: frontmatter.topics,
            published: frontmatter.published,
        };
        let mut ser = serde_yaml::Serializer::new(&mut result);
        frontmatter.serialize(&mut ser).unwrap();
        result.extend(b"---\n");
        String::from_utf8(result).unwrap()
    }

    fn compile_elements(&mut self, elements: Vec<Element>) -> String {
        elements
            .into_iter()
            .map(|element| self.compile_element(element))
            .collect()
    }

    fn compile_element(&mut self, element: Element) -> String {
        match element {
            Element::Text(text) => text,
            Element::Url(url) => format!("\n{}\n", url),
            Element::Macro(macro_info) => self.compile_elements(macro_info.zenn),
            Element::Image { alt, url } => {
                format!("![{}]({})", alt, url)
            }
            Element::InlineFootnote(content) => format!("^[{}]", content),
            Element::Footnote(name) => format!("[^{}]", name),
            Element::Message {
                level,
                msg_type,
                body,
            } => {
                let msg_type = match msg_type {
                    MessageType::Info => "",
                    MessageType::Warn => "",
                    MessageType::Alert => "alert",
                };

                let mut compiler = ZennCompiler {};
                let body = compiler.compile_elements(body);

                format!(
                    ":::{0}message {1}\n{2}:::{0}",
                    ":".repeat(level),
                    msg_type,
                    body
                )
            }
            Element::Details { level, title, body } => {
                let mut compiler = ZennCompiler {};
                let body = compiler.compile_elements(body);
                format!(
                    ":::{0}details {1}\n{2}:::{0}",
                    ":".repeat(level),
                    title,
                    body
                )
            }
        }
    }
}
