use std::{error::Error, fmt::Display};

use crate::{
    ast::{MarkdownDoc, TokenizedMd},
    r#macro::{StringMacro, TokenizedMacro},
    token::{Token, TokenType},
};

const SEPARATOR: &str = "---\n";
const MESSAGE_TAG: &str = "message";
const DETAILS_TAG: &str = "details";

type Result<T> = std::result::Result<T, ScanError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanError {
    pub error_type: ScanErrorType,
    pub row: usize,
    pub col: usize,
}

impl ScanError {
    pub fn new(error_type: ScanErrorType, row: usize, col: usize) -> Self {
        Self {
            error_type,
            row,
            col,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanErrorType {
    Incomplete(String),
    InvalidMacro,
}

impl Display for ScanErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScanErrorType::Incomplete(string) => write!(f, "Incomplete '{}'.", string),
            ScanErrorType::InvalidMacro => write!(f, "Invalid macro."),
        }
    }
}

impl Error for ScanErrorType {}

#[derive(Debug, Clone)]
pub struct Scanner {
    source: Vec<char>,

    current: usize,
    start: usize,

    row: usize,
    col: usize,

    tokens: Vec<Token>,
    errors: Vec<ScanError>,
}

impl Scanner {
    pub fn new(source: Vec<char>) -> Self {
        Self::with_row_col(source, 1, 1)
    }

    fn with_row_col(source: Vec<char>, row: usize, col: usize) -> Self {
        Self {
            source,
            current: 0,
            start: 0,
            row,
            col,
            tokens: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn scan_file(mut self) -> std::result::Result<TokenizedMd, Vec<ScanError>> {
        let frontmatter = match self.scan_frontmatter() {
            Ok(frontmatter) => frontmatter,
            Err(error) => {
                self.errors.push(error);
                String::new()
            }
        };

        self.source.insert(self.current, '\n');

        let mut body = self.scan_body()?;
        if let Some(first) = body.first_mut() {
            if let TokenType::Text(text) = &mut first.token_type {
                *text = text.get(1..).unwrap_or_default().to_string();
            }
        }

        Ok(MarkdownDoc::new(frontmatter, body))
    }

    fn scan_body(mut self) -> std::result::Result<Vec<Token>, Vec<ScanError>> {
        while !self.is_at_end() {
            if let Err(error) = self.scan() {
                self.errors.push(error);
            }
        }

        self.collect_text();

        if !self.errors.is_empty() {
            return Err(self.errors);
        }

        Ok(self.tokens)
    }

    fn scan_frontmatter(&mut self) -> Result<String> {
        self.consume_spaces();
        self.expect_string(SEPARATOR);
        self.delete_buffer();
        self.extract_until(SEPARATOR)?;

        let header = self.consume_buffer();

        self.expect_string(SEPARATOR);
        self.delete_buffer();
        Ok(header)
    }

    fn scan(&mut self) -> Result<()> {
        let Some(c) = self.peek() else {
            return Ok(());
        };

        match c {
            '!' => {
                if !self.matches_keyword("![") {
                    self.advance();
                    return Ok(());
                }
                self.collect_text();
                self.expect_string("![");
                self.delete_buffer();

                self.extract_until("]")?;

                let alt = self.consume_buffer();
                self.expect_string("](");
                self.delete_buffer();

                self.extract_until(")")?;

                let url = self.consume_buffer();
                self.expect_string(")");
                self.delete_buffer();

                self.tokens
                    .push(self.make_token(TokenType::Image { alt, url }));
            }

            '@' => {
                if !self.matches_keyword("@[") {
                    self.advance();
                    return Ok(());
                }
                self.collect_text();
                self.expect_string("@[");
                self.delete_buffer();

                self.extract_until("]")?;

                let card_type = self.consume_buffer();
                self.expect_string("](");
                self.delete_buffer();

                self.extract_until(")")?;

                let url = self.consume_buffer();
                self.expect_string(")");
                self.delete_buffer();

                self.tokens
                    .push(self.make_token(TokenType::LinkCard { card_type, url }));
            }

            '^' => {
                if !self.matches_keyword("^[") {
                    self.advance();
                    return Ok(());
                }
                self.collect_text();
                self.expect_string("^[");
                self.delete_buffer();
                self.extract_until("]")?;
                let footnote = self.consume_buffer();
                self.expect_string("]");
                self.delete_buffer();
                self.tokens
                    .push(self.make_token(TokenType::InlineFootnote(footnote)));
            }

            '[' => {
                if !self.matches_keyword("[^") {
                    self.advance();
                    return Ok(());
                }
                let start = self.current;
                self.collect_text();
                self.expect_string("[^");
                self.delete_buffer();
                self.extract_until("]")?;
                let footnote = self.consume_buffer();
                self.expect_string("]");
                self.delete_buffer();
                if self.matches_keyword(":") {
                    self.start = start;
                    return Ok(());
                }
                self.tokens
                    .push(self.make_token(TokenType::Footnote(footnote)));
            }

            '`' => {
                if self.matches_keyword("```") {
                    self.expect_string("```");
                    self.extract_until("```")?;
                    self.expect_string("```");
                } else {
                    self.expect_string("`");
                    self.extract_until("`")?;
                    self.expect_string("`");
                }
            }
            '<' => {
                if !self.matches_keyword("<macro>") {
                    self.advance();
                    return Ok(());
                }
                self.collect_text();
                self.expect_string("<macro>");
                let (row, col) = (self.row, self.col);
                self.delete_buffer();
                self.extract_until("</macro>")?;

                let body = self.consume_buffer();
                self.expect_string("</macro>");
                self.delete_buffer();

                let yaml = serde_yaml::from_str::<StringMacro>(&body).map_err(|error| {
                    let (row, col) = if let Some(location) = error.location() {
                        (location.line(), location.column())
                    } else {
                        (0, 0)
                    };

                    ScanError::new(ScanErrorType::InvalidMacro, row, col)
                })?;

                let zenn = yaml.zenn.unwrap_or_default();
                let scanner = Scanner::with_row_col(zenn.chars().collect(), row, col);
                let zenn_tokens = match scanner.scan_body() {
                    Ok(tokens) => tokens,
                    Err(errors) => {
                        self.errors.extend(errors);
                        return Err(ScanError::new(ScanErrorType::InvalidMacro, row, col));
                    }
                };
                let qiita = yaml.qiita.unwrap_or_default();
                let scanner = Scanner::with_row_col(qiita.chars().collect(), row, col);
                let qiita_tokens = match scanner.scan_body() {
                    Ok(tokens) => tokens,
                    Err(errors) => {
                        self.errors.extend(errors);
                        return Err(ScanError::new(ScanErrorType::InvalidMacro, row, col));
                    }
                };
                self.tokens
                    .push(self.make_token(TokenType::Macro(TokenizedMacro {
                        zenn: zenn_tokens,
                        qiita: qiita_tokens,
                    })));
            }

            '\n' => {
                self.advance();
                
                self.block_element()?;
            }

            _ => {
                self.advance();
            }
        }

        Ok(())
    }

    fn block_element(&mut self) -> Result<()> {
        let Some(c_next) = self.peek() else {
            return Ok(());
        };

        self.consume_spaces();

        match c_next {
            'h' => {
                if !(self.matches_keyword("https://") || self.matches_keyword("http://")) {
                    self.advance();
                    return Ok(());
                }

                self.collect_text();

                while let Some(c) = self.peek() {
                    if c.is_whitespace() {
                        break;
                    }
                    if self.is_at_end() {
                        break;
                    }
                    self.advance();
                }

                let url = self.consume_buffer();
                self.tokens.push(self.make_token(TokenType::Url(url)));
            }
            ':' => {
                if !self.matches_keyword(":::") {
                    return Ok(());
                }

                self.collect_text();
                self.expect_string(":::");
                let mut level: usize = 0;

                while self.expect_string(":") {
                    level += 1;
                }

                if self.matches_keyword(MESSAGE_TAG) {
                    self.expect_string(MESSAGE_TAG);
                    self.consume_spaces();
                    self.delete_buffer();
                    self.extract_until("\n")?;
                    let message_type = self.consume_buffer();
                    self.delete_buffer();
                    self.tokens.push(self.make_token(TokenType::MessageBegin {
                        level,
                        r#type: message_type,
                    }));
                } else if self.matches_keyword(DETAILS_TAG) {
                    self.expect_string(DETAILS_TAG);
                    self.consume_spaces();
                    self.delete_buffer();
                    self.extract_until("\n")?;
                    let title = self.consume_buffer();
                    self.delete_buffer();
                    self.tokens
                        .push(self.make_token(TokenType::DetailsBegin { level, title }));
                } else {
                    self.delete_buffer();
                    self.tokens
                        .push(self.make_token(TokenType::MessageOrDetailsEnd { level }));
                }
            }
            _ => (),
        }

        Ok(())
    }

    fn make_token(&self, token_type: TokenType) -> Token {
        Token {
            token_type,
            row: self.row,
            col: self.col,
        }
    }

    fn advance(&mut self) -> Option<char> {
        let result = self.source.get(self.current).copied();
        self.current += 1;
        if let Some('\n') = result {
            self.row += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }

        result
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }

    fn peek(&mut self) -> Option<char> {
        self.source.get(self.current).copied()
    }

    fn matches_keyword(&mut self, keyword: &str) -> bool {
        let keyword = keyword.chars().collect::<Vec<char>>();
        let Some(target) = self.source.get(self.current..self.current + keyword.len()) else {
            return false;
        };

        target == keyword
    }

    fn delete_buffer(&mut self) {
        self.start = self.current;
    }

    #[must_use]
    fn consume_buffer(&mut self) -> String {
        let result = self
            .source
            .get(self.start..self.current)
            .unwrap_or_default();
        self.start = self.current;
        result.iter().collect()
    }

    fn expect_string(&mut self, string: &str) -> bool {
        if self.matches_keyword(string) {
            (0..string.len()).for_each(|_| {
                self.advance();
            });
            true
        } else {
            false
        }
    }

    fn collect_text(&mut self) {
        let text = self.consume_buffer();
        self.tokens.push(self.make_token(TokenType::Text(text)));
    }

    fn extract_until(&mut self, end: &str) -> Result<()> {
        let (pos, row, col) = (self.current, self.row, self.col);
        self.extract_until_unchecked(end);

        if self.is_at_end() {
            self.current = pos;
            self.row = row;
            self.col = col;

            self.advance();

            return Err(ScanError::new(
                ScanErrorType::Incomplete(end.to_string()),
                self.row,
                self.col,
            ));
        }

        Ok(())
    }

    fn extract_while(&mut self, char: char) {
        while self.peek() == Some(char) && !self.is_at_end() {
            self.advance();
        }
    }

    fn extract_until_unchecked(&mut self, until: &str) {
        let first_char = until.chars().next().expect("until should not be empty");
        while !((self.peek() == Some(first_char)) && self.matches_keyword(until)
            || self.is_at_end())
        {
            self.advance();
        }
    }

    fn consume_spaces(&mut self) {
        self.extract_while(' ');
    }
}
