use crate::token::Token;

#[derive(Debug, Clone)]
pub struct MarkdownDoc<F, E> {
    pub frontmatter: F,
    pub elements: Vec<E>,
}

impl<F, E> MarkdownDoc<F, E> {
    pub fn new(frontmatter: F, elements: Vec<E>) -> Self {
        Self { frontmatter, elements }
    }
}

pub type TokenizedMd = MarkdownDoc<String, Token>;
pub type ParsedMd = MarkdownDoc<ZetaFrontmatter, Element>;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ZetaFrontmatter {
    pub title: String,
    pub emoji: String,
    pub r#type: String,
    pub topics: Vec<String>,
    pub published: bool,
    /// compile only specified platform
    #[serde(skip_serializing_if = "Option::is_none")]
    pub only: Option<Platform>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, clap::ValueEnum)]
pub enum Platform {
    #[serde(alias = "zenn")]
    Zenn,
    #[serde(alias = "qiita")]
    Qiita,
}

pub type StringMacro = Macro<Option<String>>;
pub type TokenizedMacro = Macro<Vec<Token>>;
pub type ParsedMacro = Macro<Vec<Element>>;

#[derive(Debug, Clone, serde::Deserialize, PartialEq, Eq)]
pub struct Macro<T> {
    pub zenn: T,
    pub qiita: T,
}

#[derive(Debug, Clone)]
pub enum Element {
    Text(String),
    Url(String),
    Macro(ParsedMacro),
    Image {
        alt: String,
        url: String,
    },
    InlineFootnote(String),
    Footnote(String),
    Message {
        msg_type: MessageType,
        body: Vec<Element>,
    },
    Details {
        title: String,
        body: Vec<Element>,
    },
}

#[derive(Debug, Clone)]
pub enum MessageType {
    Info,
    Warn,
    Alert,
}
