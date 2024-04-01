use std::fmt::Display;

use crate::{
    ast::{Element, MarkdownDoc, MessageType, ParsedMd, TokenizedMd, ZetaFrontmatter},
    r#macro::ParsedMacro,
    token::{Token, TokenType},
};

const FRONTMATTER_TOPICS_MAX: usize = 5;

type Result<T> = std::result::Result<T, ParseError>;

#[derive(Debug, Clone)]
pub struct ParseError {
    pub error_type: ParseErrorType,
    pub row: usize,
    pub col: usize,
}

impl ParseError {
    pub fn new(error_type: ParseErrorType, row: usize, col: usize) -> Self {
        Self {
            error_type,
            row,
            col,
        }
    }
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorType {
    TooManyTopics(Vec<String>),
    InvalidFrontMatter,
    InvalidMacro,
    InvalidMessageType,
    InvalidNestingLevel(usize),
    CouldNotFindEndToken(TokenType),
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} at row {}, col {}",
            self.error_type, self.row, self.col
        )
    }
}

impl Display for ParseErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseErrorType::TooManyTopics(topics) => write!(
                f,
                "Too many topics: [{}]. The maximum number of topics is {}.",
                topics.join(", "),
                FRONTMATTER_TOPICS_MAX
            ),
            ParseErrorType::InvalidFrontMatter => write!(f, "Invalid front matter"),
            ParseErrorType::InvalidMacro => write!(f, "Invalid macro"),
            ParseErrorType::InvalidMessageType => write!(f, "Invalid message type"),
            ParseErrorType::InvalidNestingLevel(level) => write!(
                f,
                "Invalid nesting level: {}. The nesting level must be smaller than the outer one.",
                level
            ),
            ParseErrorType::CouldNotFindEndToken(token_type) => write!(
                f,
                "Could not find end token: {:?}.",
                token_type
            ),
        }
    }
}

impl std::error::Error for ParseError {}

pub struct Parser {
    source: Vec<Token>,
    frontmatter: String,

    position: usize,

    nesting_levels: Vec<usize>,

    errors: Vec<ParseError>,
}

impl Parser {
    pub fn new(md: TokenizedMd) -> Self {
        Self {
            source: md.elements,
            frontmatter: md.frontmatter,
            position: 0,
            nesting_levels: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn parse(mut self) -> std::result::Result<ParsedMd, Vec<ParseError>> {
        let frontmatter = match self.parse_frontmatter() {
            Ok(frontmatter) => frontmatter,
            Err(error) => {
                self.errors.push(error);
                ZetaFrontmatter::default()
            }
        };

        let elements = self.parse_body()?;

        Ok(ParsedMd {
            elements,
            frontmatter,
        })
    }

    fn parse_frontmatter(&mut self) -> Result<ZetaFrontmatter> {
        let content = &self.frontmatter;

        let result = serde_yaml::from_str::<ZetaFrontmatter>(content).map_err(|error| {
            let (row, col) = if let Some(location) = error.location() {
                (location.line(), location.column())
            } else {
                (0, 0)
            };

            ParseError::new(ParseErrorType::InvalidFrontMatter, row, col)
        });

        if let Ok(frontmatter) = &result {
            if frontmatter.topics.len() > FRONTMATTER_TOPICS_MAX {
                return Err(ParseError::new(
                    ParseErrorType::TooManyTopics(frontmatter.topics.clone()),
                    0,
                    0,
                ));
            }
        }

        result
    }

    fn parse_body(mut self) -> std::result::Result<Vec<Element>, Vec<ParseError>> {
        let elements = self.parse_block(None);
        if !self.errors.is_empty() {
            return Err(self.errors);
        }
        Ok(elements)
    }

    fn parse_block(&mut self, end: Option<TokenType>) -> Vec<Element> {
        let mut elements = Vec::new();

        while let Some(token) = self.peek() {
            if let Some(ref end) = end {
                if token.token_type == *end {
                    break;
                }
            }

            let element = match self.parse_element() {
                Ok(element) => element,
                Err(error) => {
                    self.errors.push(error);
                    break;
                }
            };

            elements.push(element);
        }

        if let Some(end) = end {
            if self.peek().is_none() {
                self.errors.push(ParseError::new(
                    ParseErrorType::CouldNotFindEndToken(end),
                        0,
                        0
                ));
            }
        }

        elements
    }

    fn parse_element(&mut self) -> Result<Element> {
        let Some(token) = self.advance().cloned() else {
            unreachable!("parse_element() should not be called when source is empty");
        };

        let elem = match token.token_type {
            TokenType::Text(text) => Element::Text(text),
            TokenType::Url(url) => Element::Url(url),
            TokenType::Image { alt, url } => Element::Image { alt, url },
            TokenType::LinkCard { card_type, url } => Element::LinkCard { card_type, url },
            TokenType::InlineFootnote(footnote) => Element::InlineFootnote(footnote),
            TokenType::Footnote(footnote) => Element::Footnote(footnote),
            TokenType::MessageBegin { level, r#type } => {
                let msg_type = match r#type.as_str() {
                    "info" => MessageType::Info,
                    "warn" => MessageType::Warn,
                    "alert" => MessageType::Alert,
                    _ => {
                        return Err(ParseError::new(
                            ParseErrorType::InvalidMessageType,
                            token.row,
                            token.col,
                        ))
                    }
                };
                self.nest(level, token.row, token.col)?;
                let body = self.parse_block(Some(TokenType::MessageOrDetailsEnd { level }));
                self.advance();
                self.unnest();
                Element::Message {
                    level,
                    msg_type,
                    body,
                }
            }
            TokenType::DetailsBegin { level, title } => {
                self.nest(level, token.row, token.col)?;
                let body = self.parse_block(Some(TokenType::MessageOrDetailsEnd { level }));
                self.advance();
                self.unnest();
                Element::Details { level, title, body }
            }
            TokenType::MessageOrDetailsEnd { level: _ } => Element::Text("".to_string()),
            TokenType::Macro(macro_info) => {
                let zenn_parser = Parser::new(MarkdownDoc {
                    frontmatter: String::new(),
                    elements: macro_info.zenn,
                });
                let zenn_elements = match zenn_parser.parse_body() {
                    Ok(zenn_elements) => zenn_elements,
                    Err(errors) => {
                        self.errors.extend(errors);
                        return Err(ParseError::new(
                            ParseErrorType::InvalidMacro,
                            token.row,
                            token.col,
                        ));
                    }
                };

                let qiita_parser = Parser::new(MarkdownDoc {
                    frontmatter: String::new(),
                    elements: macro_info.qiita,
                });
                let qiita_elements = match qiita_parser.parse_body() {
                    Ok(qiita_elements) => qiita_elements,
                    Err(errors) => {
                        self.errors.extend(errors);
                        return Err(ParseError::new(
                            ParseErrorType::InvalidMacro,
                            token.row,
                            token.col,
                        ));
                    }
                };

                Element::Macro(ParsedMacro {
                    zenn: zenn_elements,
                    qiita: qiita_elements,
                })
            }
        };

        Ok(elem)
    }

    fn advance(&mut self) -> Option<&Token> {
        let result = self.source.get(self.position);
        self.position += 1;
        result
    }

    fn peek(&mut self) -> Option<&Token> {
        self.source.get(self.position)
    }

    fn nest(&mut self, level: usize, row: usize, col: usize) -> Result<()> {
        if let Some(last) = self.nesting_levels.last() {
            if level >= *last {
                return Err(ParseError::new(
                    ParseErrorType::InvalidNestingLevel(level),
                    row,
                    col,
                ));
            }
        }

        self.nesting_levels.push(level);

        Ok(())
    }

    fn unnest(&mut self) {
        self.nesting_levels
            .pop()
            .expect("unnest() should be called only when nesting_levels is not empty");
    }
}
