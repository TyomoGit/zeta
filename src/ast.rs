#[derive(Debug, Clone)]
pub struct MarkdownFile {
    pub frontmatter: ZetaHeader,
    pub elements: Vec<Element>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ZetaHeader {
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
