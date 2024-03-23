use crate::{
    ast::{Element, Macro, MarkdownFile, MessageType, ZetaHeader},
    print::zeta_error,
};

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
        self.consume_string("---\n");
        self.delete_buffer();

        while self.peek() != Some('-') {
            self.advance();
        }
        let header = self.get_buffer().unwrap();

        self.consume_string("---\n");
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

        self.consume_buffer();

        self.result
    }

    fn parse_element(&mut self) {
        dbg!(self.peek());
        let Some(c) = self.peek() else {
            return;
        };

        match c {
            '[' => {
                self.consume_buffer();
                self.consume_string("[");

                while self.peek() != Some(')') && !self.is_at_end() {
                    self.advance();
                }

                if !self.is_at_end() {
                    self.advance();
                }
            }
            '<' => {
                if !self.matches_keyword("<macro>") {
                    self.advance();
                    return;
                }
                self.consume_buffer();

                while self.peek() != Some('\n') {
                    self.advance();
                }
                self.consume_string("\n");
                self.delete_buffer();
                while self.peek() != Some('<') {
                    self.advance();
                }
                let macro_yaml = self.get_buffer().unwrap();

                self.consume_string("</macro>");
                self.delete_buffer();
                

                let Ok(macro_yaml): Result<Macro, _> = serde_yaml::from_str(macro_yaml.as_str()) else {
                    zeta_error("Invalid macro yaml");
                    return;
                };
                

                self.result.push(Element::Macro(macro_yaml));

            }
            'h' => {
                if !(self.matches_keyword("https://") || self.matches_keyword("http://")) {
                    self.advance();
                    return;
                }

                self.consume_buffer();

                while self.peek().is_some() && !self.peek().unwrap().is_whitespace() {
                    self.advance();
                }

                let url = self.get_buffer().unwrap();
                self.result.push(Element::Url(url));
            }
            '\n' => {
                self.advance();
                let Some(c_next) = self.peek() else {
                    return;
                };
                match c_next {
                    ':' => {
                        const MESSAGE_TAG: &str = "message";
                        const DETAILS_TAG: &str = "details";

                        if self.matches_keyword(format!(":::{DETAILS_TAG}").as_str()) {
                            self.consume_buffer();
                            while matches!(self.peek(), Some(':')) {
                                self.advance();
                            }

                            self.consume_string(DETAILS_TAG);

                            self.advance_spaces();

                            self.delete_buffer();
                            while self.peek() != Some('\n') {
                                self.advance();
                            }
                            let title = self.get_buffer().unwrap_or("".to_string());

                            if let Some('\n') = self.peek() {
                                self.advance();
                            }
                            self.delete_buffer();
                            while self.peek() != Some(':') {
                                self.advance();
                            }
                            let content = self.get_buffer().unwrap_or("".to_string());
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

                            return;
                        }

                        if !self.matches_keyword(format!(":::{MESSAGE_TAG}").as_str()) {
                            return;
                        }
                        self.consume_buffer();

                        while matches!(self.peek(), Some(':')) {
                            self.advance();
                        }

                        (0..MESSAGE_TAG.len()).for_each(|_| {
                            self.advance();
                        });

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

                        while self.peek() != Some('\n') {
                            self.advance();
                        }

                        self.consume_string("\n");

                        self.delete_buffer();

                        while self.peek() != Some(':') {
                            self.advance();
                        }

                        let content = self.get_buffer().unwrap();
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

                    '`' => {
                        if self.peek_next() != Some('`') {
                            // inline
                            while self.peek() != Some('`') {
                                self.advance();
                            }
                            self.consume_string("`");
                        } else if self.matches_keyword("```") {
                            // block
                            self.consume_string("```");

                            while self.peek() != Some('`') && !self.is_at_end() {
                                self.advance();
                            }

                            self.consume_string("```");
                        }
                    }

                    '!' => {
                        if !self.matches_keyword("![") {
                            return;
                        }
                        self.consume_buffer();
                        self.consume_string("![");
                        self.delete_buffer();

                        while self.peek() != Some(']') {
                            self.advance();
                        }

                        let alt = self.get_buffer().unwrap_or("".to_string());
                        self.consume_string("](");
                        self.delete_buffer();

                        while self.peek() != Some(')') {
                            self.advance();
                        }

                        let url = self.get_buffer().unwrap();
                        self.consume_string(")");
                        self.delete_buffer();

                        self.result.push(Element::Image { alt, url });
                    }
                    _ => (),
                }
            }

            '^' => {
                if !self.matches_keyword("^[") {
                    self.advance();
                    return;
                }
                self.consume_buffer();

                self.consume_string("^[");
                self.delete_buffer();

                while self.peek() != Some(']') {
                    self.advance();
                }

                let inline_footnote = self.get_buffer().unwrap();
                self.result.push(Element::InlineFootnote(inline_footnote));

                self.consume_string("]");
                self.delete_buffer();
            }

            _ => {
                self.advance();
            }
        }
    }

    fn advance(&mut self) -> Option<char> {
        self.position += 1;
        self.source.get(self.position - 1).copied()
    }

    fn back(&mut self) -> bool {
        if self.position == 0 {
            return false;
        }

        self.position -= 1;

        true
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

    fn get_buffer(&mut self) -> Option<String> {
        if self.start == self.position {
            None
        } else {
            let test = self.source.get(self.start..self.position).unwrap();
            self.start = self.position;
            Some(test.iter().collect())
        }
    }

    fn consume_buffer(&mut self) {
        if let Some(buffer) = self.get_buffer() {
            self.result.push(Element::Text(buffer));
        }
    }

    fn advance_spaces(&mut self) {
        while self.peek() == Some(' ') {
            self.advance();
        }
    }

    fn consume_string(&mut self, string: &str) -> bool {
        if self.matches_keyword(string) {
            self.position += string.len();
            return true;
        }

        false
    }
}
