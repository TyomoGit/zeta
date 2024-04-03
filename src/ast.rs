use crate::{
    r#macro::{ParsedMacro, Platform},
    token::Token,
};

/// Markdown document
#[derive(Debug, Clone)]
pub struct MarkdownDoc<F, E> {
    /// Frontmatter
    pub frontmatter: F,
    /// Elements
    pub elements: Vec<E>,
}

impl<F, E> MarkdownDoc<F, E> {
    /// Create a new MarkdownDoc
    pub fn new(frontmatter: F, elements: Vec<E>) -> Self {
        Self {
            frontmatter,
            elements,
        }
    }
}

/// Tokenized Markdown document. It contains `String` frontmatter and `Token` tokenized elements.
pub type TokenizedMd = MarkdownDoc<String, Token>;
/// Parsed Markdown document. It contains `ZetaFrontmatter` frontmatter and `Element` elements.
pub type ParsedMd = MarkdownDoc<ZetaFrontmatter, Element>;

/// Frontmatter in Zeta format
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ZetaFrontmatter {
    /// title of the article
    pub title: String,
    /// emoji
    pub emoji: String,
    /// "tech" or "idea"
    pub r#type: String,
    /// topics. Up to 5 topics can be specified.
    pub topics: Vec<String>,
    /// whether to publish or not
    pub published: bool,
    /// compile only specified platform
    #[serde(skip_serializing_if = "Option::is_none")]
    pub only: Option<Platform>,
}

/// element of Markdown document
#[derive(Debug, Clone)]
pub enum Element {
    /// Text.
    Text(String),
    /// Link starting with `http://` or `https://`.
    Url(String),
    /// Parsed macro.
    Macro(ParsedMacro),
    /// Card.
    LinkCard {
        card_type: String,
        url: String,
    },
    /// Image.
    Image {
        alt: String,
        url: String,
    },
    /// Inline footnote. E.g. `^[content]`.
    InlineFootnote(String),
    /// Footnote. E.g. `[^identifier]`.
    Footnote(String),
    /// Message to emphasize. `:::message [type]`.
    Message {
        /// The level of the message. Higher on the outside.
        level: usize,
        msg_type: MessageType,
        body: Vec<Element>,
    },
    /// Detailed folding element. `:::details [title]`.
    Details {
        /// The level of the details. Higher on the outside.
        level: usize,
        title: String,
        body: Vec<Element>,
    },
}

/// Type of the message.
#[derive(Debug, Clone)]
pub enum MessageType {
    /// Info message. Indicates supplementary information.
    Info,
    /// Warning message. Indicates a potential problem.
    Warn,
    /// Alert message. Indicates a serious problem.
    Alert,
}
