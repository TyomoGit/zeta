#[derive(Debug, Clone)]
pub struct MarkdownFile {
    pub header: ZetaHeader,
    pub elements: Vec<Element>,
}

#[derive(Debug, Clone)]
pub struct ZetaHeader {
    pub title: String,
    pub emoji: String,
    pub type_: String,
    pub topics: Vec<String>,
    pub publish: bool,
}

#[derive(Debug, Clone)]
pub enum Element {
    Text(String),
    Url(String),
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
