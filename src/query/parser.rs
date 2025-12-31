use super::ast::{CompareOp, Date, Expr, Value};

pub struct Parser<'a> {
    input: &'a str,
    pos: usize,
}

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub pos: usize,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Parse error at position {}: {}", self.pos, self.message)
    }
}

impl std::error::Error for ParseError {}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    pub fn parse(mut self) -> Result<Expr, ParseError> {
        let expr = self.parse_or()?;
        self.skip_whitespace();
        if self.pos < self.input.len() {
            return Err(self.error("Unexpected input after expression"));
        }
        Ok(expr)
    }

    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_and()?;
        loop {
            self.skip_whitespace();
            if !self.match_keyword("OR") {
                break;
            }
            let right = self.parse_and()?;
            left = Expr::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_primary()?;
        loop {
            self.skip_whitespace();
            if !self.match_keyword("AND") {
                break;
            }
            let right = self.parse_primary()?;
            left = Expr::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        self.skip_whitespace();

        if self.match_char('(') {
            let expr = self.parse_or()?;
            self.skip_whitespace();
            if !self.match_char(')') {
                return Err(self.error("Expected ')'"));
            }
            return Ok(expr);
        }

        let field = self.parse_identifier()?;
        self.skip_whitespace();

        if self.match_keyword("contains") {
            self.skip_whitespace();
            let value = self.parse_value()?;
            return Ok(Expr::Contains { field, value });
        }

        let op = self.parse_operator()?;
        self.skip_whitespace();
        let value = self.parse_value()?;

        Ok(Expr::Compare { field, op, value })
    }

    fn parse_identifier(&mut self) -> Result<String, ParseError> {
        self.skip_whitespace();
        let start = self.pos;

        while self.pos < self.input.len() {
            let c = self.current_char();
            if c.is_alphanumeric() || c == '_' || c == '-' {
                self.pos += 1;
            } else {
                break;
            }
        }

        if self.pos == start {
            return Err(self.error("Expected identifier"));
        }

        Ok(self.input[start..self.pos].to_string())
    }

    fn parse_operator(&mut self) -> Result<CompareOp, ParseError> {
        self.skip_whitespace();

        if self.match_str(">=") {
            return Ok(CompareOp::Ge);
        }
        if self.match_str("<=") {
            return Ok(CompareOp::Le);
        }
        if self.match_str("!=") {
            return Ok(CompareOp::Ne);
        }
        if self.match_char('=') {
            return Ok(CompareOp::Eq);
        }
        if self.match_char('>') {
            return Ok(CompareOp::Gt);
        }
        if self.match_char('<') {
            return Ok(CompareOp::Lt);
        }

        Err(self.error("Expected operator (=, !=, >, <, >=, <=)"))
    }

    fn parse_value(&mut self) -> Result<Value, ParseError> {
        self.skip_whitespace();

        if self.match_char('"') {
            return self.parse_string();
        }

        if self.match_keyword("true") {
            return Ok(Value::Bool(true));
        }
        if self.match_keyword("false") {
            return Ok(Value::Bool(false));
        }

        self.parse_number_or_date()
    }

    fn parse_string(&mut self) -> Result<Value, ParseError> {
        let start = self.pos;
        while self.pos < self.input.len() && self.current_char() != '"' {
            self.pos += 1;
        }
        let s = self.input[start..self.pos].to_string();
        if !self.match_char('"') {
            return Err(self.error("Unterminated string"));
        }
        Ok(Value::String(s))
    }

    fn parse_number_or_date(&mut self) -> Result<Value, ParseError> {
        let start = self.pos;

        let negative = self.match_char('-');

        while self.pos < self.input.len() {
            let c = self.current_char();
            if c.is_ascii_digit() || c == '.' || c == '-' {
                self.pos += 1;
            } else {
                break;
            }
        }

        if self.pos == start || (negative && self.pos == start + 1) {
            return Err(self.error("Expected number or date"));
        }

        let text = &self.input[start..self.pos];

        if let Some(date) = try_parse_date(text) {
            return Ok(Value::Date(date));
        }

        text.parse::<f64>()
            .map(Value::Number)
            .map_err(|_| self.error("Invalid number"))
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() && self.current_char().is_whitespace() {
            self.pos += 1;
        }
    }

    fn current_char(&self) -> char {
        self.input[self.pos..].chars().next().unwrap_or('\0')
    }

    fn match_char(&mut self, c: char) -> bool {
        if self.pos < self.input.len() && self.current_char() == c {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn match_str(&mut self, s: &str) -> bool {
        if self.input[self.pos..].starts_with(s) {
            self.pos += s.len();
            true
        } else {
            false
        }
    }

    fn match_keyword(&mut self, kw: &str) -> bool {
        let remaining = &self.input[self.pos..];
        if remaining.len() < kw.len() {
            return false;
        }
        if !remaining[..kw.len()].eq_ignore_ascii_case(kw) {
            return false;
        }
        let after = remaining.chars().nth(kw.len());
        if after.map_or(true, |c| !c.is_alphanumeric() && c != '_') {
            self.pos += kw.len();
            true
        } else {
            false
        }
    }

    fn error(&self, message: &str) -> ParseError {
        ParseError {
            message: message.to_string(),
            pos: self.pos,
        }
    }
}

fn try_parse_date(s: &str) -> Option<Date> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return None;
    }

    let year: i32 = parts[0].parse().ok()?;
    let month: u8 = parts[1].parse().ok()?;
    let day: u8 = parts[2].parse().ok()?;

    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }

    Some(Date::new(year, month, day))
}

pub fn parse(input: &str) -> Result<Expr, ParseError> {
    Parser::new(input).parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_eq() {
        let expr = parse(r#"status = "active""#).unwrap();
        assert!(matches!(expr, Expr::Compare { op: CompareOp::Eq, .. }));
    }

    #[test]
    fn test_and() {
        let expr = parse(r#"status = "done" AND priority > 2"#).unwrap();
        assert!(matches!(expr, Expr::And(_, _)));
    }

    #[test]
    fn test_contains() {
        let expr = parse(r#"tags contains "project""#).unwrap();
        assert!(matches!(expr, Expr::Contains { .. }));
    }

    #[test]
    fn test_date() {
        let expr = parse("created >= 2024-01-01").unwrap();
        if let Expr::Compare { value: Value::Date(d), .. } = expr {
            assert_eq!(d.year, 2024);
            assert_eq!(d.month, 1);
            assert_eq!(d.day, 1);
        } else {
            panic!("Expected date comparison");
        }
    }
}
