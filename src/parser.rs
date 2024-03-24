use std::fmt::Display;

use crate::{
    ast::{Element, Macro, MarkdownFile, MessageType},
    print::zeta_error_position,
};

const SEPARATOR: &str = "---\n";
const MESSAGE_TAG: &str = "message";
const DETAILS_TAG: &str = "details";

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
    Incomplete(String),
    InvalidFrontMatter,
    InvalidMacro,
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
            ParseErrorType::Incomplete(string) => write!(f, "Incomplete {}", string),
            ParseErrorType::InvalidFrontMatter => write!(f, "Invalid front matter"),
            ParseErrorType::InvalidMacro => write!(f, "Invalid macro"),
        }
    }
}

impl std::error::Error for ParseError {}

pub struct Parser {
    source: Vec<char>,

    position: usize,
    start: usize,

    row: usize,
    col: usize,

    result: Vec<Element>,
    errors: Vec<ParseError>,
}

impl Parser {
    pub fn new(source: Vec<char>) -> Self {
        Self {
            source,
            position: 0,
            start: 0,

            row: 1,
            col: 1,

            result: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn parse_file(mut self) -> std::result::Result<MarkdownFile, Vec<ParseError>> {
        self.expect_string(SEPARATOR);
        self.delete_buffer();

        if let Err(error) = self.extract_until(SEPARATOR) {
            self.errors.push(error);
        };

        let header = self.consume_buffer().unwrap();

        self.expect_string(SEPARATOR);
        self.delete_buffer();

        let yaml_result = serde_yaml::from_str(header.as_str());
        match yaml_result {
            Ok(frontmatter) => {
                let result = self.parse();
                Ok(MarkdownFile {
                    frontmatter,
                    elements: result?,
                })
            }
            Err(error) => {
                let location = error
                    .location()
                    .map(|l| (l.line() + 1, l.column()))
                    .unwrap_or((self.row, self.col));
                let error =
                    ParseError::new(ParseErrorType::InvalidFrontMatter, location.0, location.1);
                self.errors.push(error);
                Err(self.errors)
            }
        }
    }

    fn parse(mut self) -> std::result::Result<Vec<Element>, Vec<ParseError>> {
        while !self.is_at_end() {
            if let Err(error) = self.parse_element() {
                self.errors.push(error)
            }
        }

        self.collect_text();

        if self.errors.is_empty() {
            Ok(self.result)
        } else {
            Err(self.errors)
        }
    }

    fn parse_element(&mut self) -> Result<()> {
        let Some(c) = self.peek() else {
            return Ok(());
        };

        match c {
            '[' => {
                self.square_brackets_or_link()?;
            }
            '<' => {
                if !self.matches_keyword("<macro>") {
                    self.advance();
                    return Ok(());
                }

                self.macro_call()?;
            }

            '^' => {
                if !self.matches_keyword("^[") {
                    self.advance();
                    return Ok(());
                }
                self.inline_footnote()?;
            }

            '\n' => {
                // block element

                self.advance();

                self.parse_block_element()?;
            }

            _ => {
                self.advance();
            }
        }

        Ok(())
    }

    fn parse_block_element(&mut self) -> Result<()> {
        let Some(c_next) = self.peek() else {
            return Ok(());
        };

        match c_next {
            'h' => {
                if !(self.matches_keyword("https://") || self.matches_keyword("http://")) {
                    self.advance();
                    return Ok(());
                }

                self.url();
            }
            ':' => {
                if self.matches_keyword(format!(":::{DETAILS_TAG}").as_str()) {
                    self.details()?;
                } else if self.matches_keyword(format!(":::{MESSAGE_TAG}").as_str()) {
                    self.message()?;
                }
            }

            '`' => {
                self.code_block()?;
            }

            '!' => {
                if !self.matches_keyword("![") {
                    return Ok(());
                }
                self.image()?;
            }
            _ => (),
        }

        Ok(())
    }

    fn square_brackets_or_link(&mut self) -> Result<()> {
        self.extract_until("]")?;
        self.expect_string("]");

        if self.matches_keyword("(") {
            self.extract_until(")")?;
            self.expect_string(")");
        }

        Ok(())
    }

    fn macro_call(&mut self) -> Result<()> {
        self.collect_text();

        self.expect_string("<macro>");
        self.delete_buffer();

        self.extract_until("</macro>")?;

        let macro_yaml = self.consume_buffer().unwrap_or_default();

        self.expect_string("</macro>");
        self.delete_buffer();

        let Ok(macro_yaml): std::result::Result<Macro, _> =
            serde_yaml::from_str(macro_yaml.as_str())
        else {
            return Err(ParseError::new(
                ParseErrorType::InvalidMacro,
                self.row,
                self.col,
            ));
        };

        self.result.push(Element::Macro(macro_yaml));

        Ok(())
    }

    fn inline_footnote(&mut self) -> Result<()> {
        self.collect_text();

        self.expect_string("^[");
        self.delete_buffer();

        self.extract_until("]")?;

        let inline_footnote = self.consume_buffer().unwrap();
        self.result.push(Element::InlineFootnote(inline_footnote));

        self.expect_string("]");
        self.delete_buffer();

        Ok(())
    }

    fn url(&mut self) {
        self.collect_text();

        while self.peek().is_some() && !self.peek().unwrap().is_whitespace() {
            self.advance();
        }

        let url = self.consume_buffer().unwrap();
        self.result.push(Element::Url(url));
    }

    fn details(&mut self) -> Result<()> {
        self.collect_text();
        while matches!(self.peek(), Some(':')) {
            self.advance();
        }

        self.expect_string(DETAILS_TAG);

        self.advance_spaces();

        self.delete_buffer();
        self.extract_until_unchecked('\n');

        let title = self.consume_buffer().unwrap_or_default();

        self.expect_string("\n");
        self.delete_buffer();
        self.extract_until(":::")?;
        let content = self.consume_buffer().unwrap_or_default();
        let parser = Parser::new(content.chars().collect());
        let content = parser.parse();
        match content {
            Ok(content) => {
                self.result.push(Element::Details {
                    title,
                    body: content,
                });
            }
            Err(errors) => {
                self.errors.extend(errors);
            }
        }

        while matches!(self.peek(), Some(':')) {
            self.advance();
        }

        self.delete_buffer();

        Ok(())
    }

    fn message(&mut self) -> Result<()> {
        self.collect_text();

        while matches!(self.peek(), Some(':')) {
            self.advance();
        }

        self.expect_string(MESSAGE_TAG);

        self.advance_spaces();

        let message_type = if self.matches_keyword("info") {
            MessageType::Info
        } else if self.matches_keyword("warn") {
            MessageType::Warn
        } else if self.matches_keyword("alert") {
            MessageType::Alert
        } else {
            zeta_error_position("Invalid message type", self.row, self.col);
            MessageType::Info
        };

        self.extract_until_unchecked('\n');
        self.expect_string("\n");
        self.delete_buffer();

        self.extract_until(":::")?;

        let content = self.consume_buffer().unwrap();

        let parser = Parser::new(content.chars().collect());
        let content = parser.parse();

        match content {
            Ok(content) => {
                self.result.push(Element::Message {
                    msg_type: message_type,
                    body: content,
                });
            }
            Err(errors) => {
                self.errors.extend(errors);
            }
        }

        while matches!(self.peek(), Some(':')) {
            self.advance();
        }

        self.delete_buffer();

        Ok(())
    }

    fn code_block(&mut self) -> Result<()> {
        if self.peek_next() != Some('`') {
            // inline
            self.expect_string("`");
            self.extract_until("`")?;
            self.expect_string("`");
        } else if self.matches_keyword("```") {
            // block
            self.expect_string("```");
            self.extract_until("```")?;
            self.expect_string("```");
        }

        Ok(())
    }

    fn image(&mut self) -> Result<()> {
        self.collect_text();
        self.expect_string("![");
        self.delete_buffer();

        self.extract_until("]")?;

        let alt = self.consume_buffer().unwrap_or_default();
        self.expect_string("](");
        self.delete_buffer();

        self.extract_until(")")?;

        let url = self.consume_buffer().unwrap();
        self.expect_string(")");
        self.delete_buffer();

        self.result.push(Element::Image { alt, url });

        Ok(())
    }

    fn advance(&mut self) -> Option<char> {
        let result = self.source.get(self.position).copied();
        self.position += 1;
        if let Some('\n') = result {
            self.row += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }

        result
    }

    fn peek(&mut self) -> Option<char> {
        self.source.get(self.position).copied()
    }

    fn peek_next(&mut self) -> Option<char> {
        self.source.get(self.position + 1).copied()
    }

    fn matches_keyword(&mut self, keyword: &str) -> bool {
        let Some(target) = self
            .source
            .get(self.position..self.position + keyword.len())
        else {
            return false;
        };

        target.iter().cloned().eq(keyword.chars())
    }

    fn is_at_end(&mut self) -> bool {
        self.position >= self.source.len()
    }

    fn delete_buffer(&mut self) {
        self.start = self.position;
    }

    fn consume_buffer(&mut self) -> Option<String> {
        if self.start == self.position {
            None
        } else {
            let test = self.source.get(self.start..self.position).unwrap();
            self.start = self.position;
            Some(test.iter().collect())
        }
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
        if let Some(buffer) = self.consume_buffer() {
            self.result.push(Element::Text(buffer));
        }
    }

    fn extract_until(&mut self, end: &str) -> Result<()> {
        let (pos, row, col) = (self.position, self.row, self.col);
        self.extract_until_unchecked(end.chars().next().expect("end should not be empty"));

        if self.is_at_end() {
            zeta_error_position("Incomplete brackets", self.row, self.col);

            self.position = pos;
            self.row = row;
            self.col = col;

            self.advance();

            return Err(ParseError::new(
                ParseErrorType::Incomplete(end.to_string()),
                self.row,
                self.col,
            ));
        }

        Ok(())
    }

    fn extract_until_unchecked(&mut self, until: char) {
        while self.peek() != Some(until) && !self.is_at_end() {
            self.advance();
        }
    }

    fn advance_spaces(&mut self) {
        while self.peek() == Some(' ') {
            self.advance();
        }
    }
}
