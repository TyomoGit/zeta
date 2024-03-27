use crate::{ast::Element, token::Token};

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
