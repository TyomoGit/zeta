use crate::{
    ast::{Element, Macro, MarkdownFile, MessageType, ZetaHeader},
    print::zeta_error,
};

const MESSAGE_TAG: &str = "message";
const DETAILS_TAG: &str = "details";

pub struct Parser {
    source: Vec<char>,

    position: usize,
    start: usize,

    result: Vec<Element>,
}

impl Parser {
    pub fn new(source: Vec<char>) -> Self {
        Self {
            source,
            position: 0,
            start: 0,

            result: Vec::new(),
        }
    }

    pub fn parse_file(mut self) -> MarkdownFile {
        self.expect_string("---\n");
        self.delete_buffer();

        self.extract_until('-');
            
        let header = self.consume_buffer().unwrap();

        self.expect_string("---\n");
        self.delete_buffer();

        let Ok(header) = serde_yaml::from_str(header.as_str()) else {
            zeta_error("Invalid header yaml");
            return MarkdownFile { header: ZetaHeader::default(), elements: vec![] };
        };

        let elements = self.parse();
        MarkdownFile { header, elements }
    }

    fn parse(mut self) -> Vec<Element> {
        while !self.is_at_end() {
            self.parse_element();
        }

        self.collect_text();

        self.result
    }

    fn parse_element(&mut self) {
        dbg!(self.peek());
        let Some(c) = self.peek() else {
            return;
        };

        match c {
            '[' => {
                self.square_brackets_or_link();
            }
            '<' => {
                if !self.matches_keyword("<macro>") {
                    self.advance();
                    return;
                }
                
                self.macro_call();
            }

            '^' => {
                if !self.matches_keyword("^[") {
                    self.advance();
                    return;
                }
                 self.inline_footnote();
            }

            '\n' => {
                self.advance();
                
                self.parse_block_element();
            }

            _ => {
                self.advance();
            }
        }
    }

    fn parse_block_element(&mut self) {
        let Some(c_next) = self.peek() else {
            return;
        };
        
        match c_next {
            'h' => {
                if !(self.matches_keyword("https://") || self.matches_keyword("http://")) {
                    self.advance();
                    return;
                }

                self.url();
            }
            ':' => {
                if self.matches_keyword(format!(":::{DETAILS_TAG}").as_str()) {
                    self.details();
                } else if self.matches_keyword(format!(":::{MESSAGE_TAG}").as_str()) {
                    self.message();
                }
            }

            '`' => {
                self.code_block();
            }

            '!' => {
                if !self.matches_keyword("![") {
                    return;
                }
                self.image();
            }
            _ => (),
        }
    }

    fn square_brackets_or_link(&mut self) {
        self.extract_until(']');
        self.expect_string("]");
        

        if self.matches_keyword("(") {

            self.extract_until(')');
            self.expect_string(")");
        }
    }

    fn macro_call(&mut self) {
        self.collect_text();

        self.extract_until('\n');
        self.expect_string("\n");
        self.delete_buffer();
        self.extract_until('<');

        let macro_yaml = self.consume_buffer().unwrap();

        self.expect_string("</macro>");
        self.delete_buffer();
        

        let Ok(macro_yaml): Result<Macro, _> = serde_yaml::from_str(macro_yaml.as_str()) else {
            zeta_error("Invalid macro yaml");
            return;
        };
        

        self.result.push(Element::Macro(macro_yaml));
    }

    fn inline_footnote(&mut self) {
        self.collect_text();

        self.expect_string("^[");
        self.delete_buffer();

        self.extract_until(']');

        let inline_footnote = self.consume_buffer().unwrap();
        self.result.push(Element::InlineFootnote(inline_footnote));

        self.expect_string("]");
        self.delete_buffer();
    }

    fn url(&mut self) {
        self.collect_text();
        
        while self.peek().is_some() && !self.peek().unwrap().is_whitespace() {
            self.advance();
        }

        let url = self.consume_buffer().unwrap();
        self.result.push(Element::Url(url));
    }

    fn details(&mut self) {
        self.collect_text();
        while matches!(self.peek(), Some(':')) {
            self.advance();
        }

        self.expect_string(DETAILS_TAG);

        self.advance_spaces();

        self.delete_buffer();
        self.extract_until(':');

        let title = self.consume_buffer().unwrap_or_default();

        if let Some('\n') = self.peek() {
            self.advance();
        }
        self.delete_buffer();
        self.extract_until(':');
        let content = self.consume_buffer().unwrap_or_default();
        let parser = Parser::new(content.chars().collect());
        let content = parser.parse();
        self.result.push(Element::Details {
            title,
            body: content,
        });

        while matches!(self.peek(), Some(':')) {
            self.advance();
        }

        self.delete_buffer();
    }

    fn message(&mut self) {
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
            zeta_error("Invalid message type");
            MessageType::Info
        };

        self.extract_until('\n');
        self.expect_string("\n");
        self.delete_buffer();

        self.extract_until(':');

        let content = self.consume_buffer().unwrap();

        let parser = Parser::new(content.chars().collect());
        let content = parser.parse();

        self.result.push(Element::Message {
            msg_type: message_type,
            body: content,
        });

        while matches!(self.peek(), Some(':')) {
            self.advance();
        }

        self.delete_buffer();
    }

    fn code_block(&mut self) {
        if self.peek_next() != Some('`') {
            // inline
            self.expect_string("`");
            self.extract_until('`');
            self.expect_string("`");
        } else if self.matches_keyword("```") {
            // block
            self.expect_string("```");
            self.extract_until('`');
            self.expect_string("```");
        }
    }

    fn image(&mut self) {
        self.collect_text();
        self.expect_string("![");
        self.delete_buffer();

        self.extract_until(']');

        let alt = self.consume_buffer().unwrap_or_default();
        self.expect_string("](");
        self.delete_buffer();

        self.extract_until(')');

        let url = self.consume_buffer().unwrap();
        self.expect_string(")");
        self.delete_buffer();

        self.result.push(Element::Image { alt, url });
    }

    fn advance(&mut self) -> Option<char> {
        self.position += 1;
        self.source.get(self.position - 1).copied()
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
            self.position += string.len();
            return true;
        }

        false
    }

    fn collect_text(&mut self) {
        if let Some(buffer) = self.consume_buffer() {
            self.result.push(Element::Text(buffer));
        }
    }

    fn extract_until(&mut self, until: char) {
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
