use std::fmt::{Display};

use crate::{ast::{Element, Macro, MarkdownDoc, MessageType, ParsedMd, TokenizedMd, ZetaFrontmatter}, token::{Token, TokenType}};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorType {
    // Incomplete(String),
    InvalidFrontMatter,
    InvalidMacro,
    InvalidMessageType,
    UnexpectedToken(TokenType),
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
            // ParseErrorType::Incomplete(string) => write!(f, "Incomplete {}", string),
            ParseErrorType::InvalidFrontMatter => write!(f, "Invalid front matter"),
            ParseErrorType::InvalidMacro => write!(f, "Invalid macro"),
            ParseErrorType::InvalidMessageType => write!(f, "Invalid message type"),
            ParseErrorType::UnexpectedToken(token_type) => write!(f, "Unexpected token: {:?}", token_type),
        }
    }
}

impl std::error::Error for ParseError {}

pub struct Parser {
    source: Vec<Token>,
    frontmatter: String,

    position: usize,

    errors: Vec<ParseError>,
}

impl Parser {
    pub fn new(md: TokenizedMd) -> Self {
        Self {
            source: md.elements,
            frontmatter: md.frontmatter,
            position: 0,
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

        serde_yaml::from_str::<ZetaFrontmatter>(content).map_err(|error| {
            let (row, col) = if let Some(location) = error.location() {
                (location.line(), location.column())
            } else {
                (0, 0)
            };

            ParseError::new(ParseErrorType::InvalidFrontMatter, row, col)
        })
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
            TokenType::InlineFootnote(footnote) => Element::InlineFootnote(footnote),
            TokenType::Footnote(footnote) => Element::Footnote(footnote),
            TokenType::MessageBegin { level, r#type } => {
                let msg_type = match r#type.as_str() {
                    "info" => MessageType::Info,
                    "warn" => MessageType::Warn,
                    "alert" => MessageType::Alert,
                    _ => return Err(ParseError::new(ParseErrorType::InvalidMessageType, token.row, token.col)),
                };
                let body = self.parse_block(Some(TokenType::MessageOrDetailsEnd { level }));
                self.advance();
                Element::Message { msg_type, body }
            }
            TokenType::DetailsBegin { level, title } => {
                let body = self.parse_block(Some(TokenType::MessageOrDetailsEnd { level }));
                self.advance();
                Element::Details { title, body }
            },
            TokenType::MessageOrDetailsEnd { level } => return Err(ParseError::new(ParseErrorType::UnexpectedToken(TokenType::MessageOrDetailsEnd { level }), token.row, token.col)),
            TokenType::Macro(macro_info) => {
                let zenn_parser = Parser::new(MarkdownDoc { frontmatter: String::new(), elements: macro_info.zenn });
                let zenn_elements = match zenn_parser.parse_body() {
                    Ok(zenn_elements) => zenn_elements,
                    Err(errors) => {
                        self.errors.extend(errors);
                        return Err(ParseError::new(ParseErrorType::InvalidMacro, token.row, token.col));
                    }
                };

                let qiita_parser = Parser::new(MarkdownDoc { frontmatter: String::new(), elements: macro_info.qiita });
                let qiita_elements = match qiita_parser.parse_body() {
                    Ok(qiita_elements) => qiita_elements,
                    Err(errors) => {
                        self.errors.extend(errors);
                        return Err(ParseError::new(ParseErrorType::InvalidMacro, token.row, token.col));
                    }
                };

                Element::Macro(Macro { zenn: zenn_elements, qiita: qiita_elements })
                
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
}
