use crate::{
    r#macro::{ParsedMacro, Platform},
    token::Token,
};

#[derive(Debug, Clone)]
pub struct MarkdownDoc<F, E> {
    pub frontmatter: F,
    pub elements: Vec<E>,
}

impl<F, E> MarkdownDoc<F, E> {
    pub fn new(frontmatter: F, elements: Vec<E>) -> Self {
        Self {
            frontmatter,
            elements,
        }
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

#[derive(Debug, Clone)]
pub enum Element {
    Text(String),
    Url(String),
    Macro(ParsedMacro),
    LinkCard {
        card_type: String,
        url: String,
    },
    Image {
        alt: String,
        url: String,
    },
    InlineFootnote(String),
    Footnote(String),
    Message {
        level: usize,
        msg_type: MessageType,
        body: Vec<Element>,
    },
    Details {
        level: usize,
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
