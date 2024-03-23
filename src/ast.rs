#[derive(Debug, Clone)]
pub struct MarkdownFile {
    pub header: ZetaHeader,
    pub elements: Vec<Element>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct ZetaHeader {
    pub title: String,
    pub emoji: String,
    pub r#type: String,
    pub topics: Vec<String>,
    pub published: bool,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Macro {
    pub zenn: Option<String>,
    pub qiita: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Element {
    Text(String),
    Url(String),
    Macro(Macro),
    Image {
        alt: String,
        url: String,
    },
    InlineFootnote(String),
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
