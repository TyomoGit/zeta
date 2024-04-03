use crate::{ast::Element, token::Token};

/// Type of platform that the macro targets.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, clap::ValueEnum)]
pub enum Platform {
    #[serde(alias = "zenn")]
    Zenn,
    #[serde(alias = "qiita")]
    Qiita,
}

/// Macro before tokenization.
pub type StringMacro = Macro<Option<String>>;
/// Tokenized macro.
pub type TokenizedMacro = Macro<Vec<Token>>;
/// Parsed macro.
pub type ParsedMacro = Macro<Vec<Element>>;

/// Macro. It contains `T` for Zenn and Qiita.
#[derive(Debug, Clone, serde::Deserialize, PartialEq, Eq)]
pub struct Macro<T> {
    pub zenn: T,
    pub qiita: T,
}
