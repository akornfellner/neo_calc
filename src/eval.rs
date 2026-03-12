use std::collections::HashMap;

// ── Expression evaluator (recursive-descent parser) ──────────────────────────

const KNOWN_FUNCTIONS: &[&str] = &[
    "sin", "cos", "tan", "asin", "acos", "atan", "log", "ln", "sqrt", "abs", "floor", "ceil",
];

const RESERVED_NAMES: &[&str] = &[
    "pi", "e", "sin", "cos", "tan", "asin", "acos", "atan", "log", "ln", "sqrt", "abs", "floor",
    "ceil",
];

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ParseError {
    pub msg: String,
    /// Character index in the *original* (with-whitespace) input string.
    pub pos: Option<usize>,
}

impl ParseError {
    fn at(msg: String, pos: usize) -> Self {
        Self {
            msg,
            pos: Some(pos),
        }
    }
}

struct Parser<'a> {
    chars: Vec<char>,
    pos: usize,
    vars: &'a HashMap<String, f64>,
    last_was_close_paren: bool,
    /// Maps index in `chars` (whitespace-stripped) → index in the original input.
    orig_positions: Vec<usize>,
}

impl<'a> Parser<'a> {
    fn new(input: &str, vars: &'a HashMap<String, f64>) -> Self {
        let mut chars = Vec::new();
        let mut orig_positions = Vec::new();
        for (i, c) in input.chars().enumerate() {
            if !c.is_whitespace() {
                chars.push(c);
                orig_positions.push(i);
            }
        }
        Self {
            chars,
            pos: 0,
            vars,
            last_was_close_paren: false,
            orig_positions,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn consume(&mut self) -> Option<char> {
        let c = self.chars.get(self.pos).copied();
        self.pos += 1;
        c
    }

    /// Map the current stripped position back to the original input index.
    fn orig_pos(&self) -> usize {
        if self.pos < self.orig_positions.len() {
            self.orig_positions[self.pos]
        } else {
            self.orig_positions.last().map(|p| p + 1).unwrap_or(0)
        }
    }

    fn err_here(&self, msg: String) -> ParseError {
        ParseError::at(msg, self.orig_pos())
    }

    // expr = term (('+' | '-') term)*
    fn expr(&mut self) -> Result<f64, ParseError> {
        let mut left = self.term()?;
        loop {
            match self.peek() {
                Some('+') => {
                    self.consume();
                    left += self.term()?;
                }
                Some('-') => {
                    self.consume();
                    left -= self.term()?;
                }
                _ => break,
            }
        }
        Ok(left)
    }

    // term = power (('*' | '/' | implicit) power)*
    fn term(&mut self) -> Result<f64, ParseError> {
        let mut left = self.power()?;
        loop {
            match self.peek() {
                Some('*') => {
                    self.consume();
                    left *= self.power()?;
                }
                Some('/') => {
                    let op_pos = self.orig_pos();
                    self.consume();
                    let right = self.power()?;
                    if right == 0.0 {
                        return Err(ParseError::at("Division durch Null".into(), op_pos));
                    }
                    left /= right;
                }
                // implicit multiplication: 2pi, 3(4+5), (2)(3), pi(2), etc.
                Some(c)
                    if c == '('
                        || c.is_alphabetic()
                        || c == '_'
                        || (c.is_ascii_digit() && self.last_was_close_paren) =>
                {
                    left *= self.power()?;
                }
                _ => break,
            }
        }
        Ok(left)
    }

    // power = postfix ('^' power)?  — right-associative
    fn power(&mut self) -> Result<f64, ParseError> {
        let base = self.postfix()?;
        if self.peek() == Some('^') {
            self.consume();
            let exp = self.power()?;
            Ok(base.powf(exp))
        } else {
            Ok(base)
        }
    }

    // postfix = factor ('!' | '°')*
    fn postfix(&mut self) -> Result<f64, ParseError> {
        let mut val = self.factor()?;
        loop {
            match self.peek() {
                Some('!') => {
                    let bang_pos = self.orig_pos();
                    self.consume();
                    val = factorial(val).map_err(|msg| ParseError::at(msg, bang_pos))?;
                }
                Some('°') => {
                    self.consume();
                    val *= std::f64::consts::PI / 180.0;
                }
                _ => break,
            }
        }
        Ok(val)
    }

    // factor = number | identifier/function | '(' expr ')' | unary
    fn factor(&mut self) -> Result<f64, ParseError> {
        self.last_was_close_paren = false;
        match self.peek() {
            Some('(') => {
                self.consume();
                let val = self.expr()?;
                match self.peek() {
                    Some(')') => {
                        self.consume();
                        self.last_was_close_paren = true;
                        Ok(val)
                    }
                    _ => Err(self.err_here("Fehlende schließende Klammer ')'".into())),
                }
            }
            Some('-') => {
                self.consume();
                Ok(-self.factor()?)
            }
            Some('+') => {
                self.consume();
                self.factor()
            }
            Some(c) if c.is_ascii_digit() || c == '.' => self.number(),
            Some(c) if c.is_alphabetic() || c == '_' => self.identifier(),
            Some(c) => Err(self.err_here(format!("Unerwartetes Zeichen: '{}'", c))),
            None => Err(self.err_here("Unerwartetes Ende des Ausdrucks".into())),
        }
    }

    fn number(&mut self) -> Result<f64, ParseError> {
        let start = self.orig_pos();
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '.' {
                s.push(c);
                self.consume();
            } else {
                break;
            }
        }
        s.parse::<f64>()
            .map_err(|_| ParseError::at(format!("Ungültige Zahl: {}", s), start))
    }

    fn identifier(&mut self) -> Result<f64, ParseError> {
        let start = self.orig_pos();
        let mut name = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                name.push(c);
                self.consume();
            } else {
                break;
            }
        }

        // if this is a known function and followed by '(', parse as function call
        if KNOWN_FUNCTIONS.contains(&name.as_str()) && self.peek() == Some('(') {
            self.consume(); // '('
            let arg = self.expr()?;
            match self.peek() {
                Some(')') => {
                    self.consume();
                    self.last_was_close_paren = true;
                }
                _ => {
                    return Err(self.err_here(format!("Fehlende ')' nach {}(…", name)));
                }
            }
            return apply_function(&name, arg).map_err(|msg| ParseError::at(msg, start));
        }

        // built-in constants
        match name.as_str() {
            "pi" => Ok(std::f64::consts::PI),
            "e" => Ok(std::f64::consts::E),
            _ => self
                .vars
                .get(&name)
                .copied()
                .ok_or_else(|| ParseError::at(format!("Unbekannte Variable '{}'", name), start)),
        }
    }
}

fn apply_function(name: &str, arg: f64) -> Result<f64, String> {
    match name {
        "sin" => Ok(arg.sin()),
        "cos" => Ok(arg.cos()),
        "tan" => Ok(arg.tan()),
        "asin" => Ok(arg.asin()),
        "acos" => Ok(arg.acos()),
        "atan" => Ok(arg.atan()),
        "log" => Ok(arg.log10()),
        "ln" => Ok(arg.ln()),
        "sqrt" => {
            if arg < 0.0 {
                Err("Wurzel aus negativer Zahl".into())
            } else {
                Ok(arg.sqrt())
            }
        }
        "abs" => Ok(arg.abs()),
        "floor" => Ok(arg.floor()),
        "ceil" => Ok(arg.ceil()),
        _ => Err(format!("Unbekannte Funktion '{}'", name)),
    }
}

fn factorial(n: f64) -> Result<f64, String> {
    if n < 0.0 || n.fract().abs() > 1e-9 {
        return Err("Fakultät erfordert eine nicht-negative ganze Zahl".into());
    }
    let n = n.round() as u64;
    if n > 170 {
        return Err("Fakultät-Überlauf (max. 170!)".into());
    }
    let mut result: f64 = 1.0;
    for i in 2..=n {
        result *= i as f64;
    }
    Ok(result)
}

pub(crate) fn evaluate(input: &str, vars: &HashMap<String, f64>) -> Result<f64, ParseError> {
    if input.trim().is_empty() {
        return Ok(0.0);
    }
    let mut parser = Parser::new(input, vars);
    let result = parser.expr()?;
    if parser.pos < parser.chars.len() {
        return Err(parser.err_here(format!(
            "Unerwartetes Zeichen: '{}'",
            parser.chars[parser.pos]
        )));
    }
    Ok(result)
}

pub(crate) fn fmt_value(v: f64) -> String {
    let s = format!("{:.10}", v);
    let s = s.trim_end_matches('0');
    let s = s.trim_end_matches('.');
    s.to_string()
}

// ── Variable name validation ─────────────────────────────────────────────────

pub(crate) fn validate_var_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Name darf nicht leer sein".into());
    }
    if RESERVED_NAMES.contains(&name) {
        return Err(format!("'{}' ist reserviert", name));
    }
    let mut chars = name.chars();
    let first_ok = chars
        .next()
        .map(|c| c.is_alphabetic() || c == '_')
        .unwrap_or(false);
    let rest_ok = chars.all(|c| c.is_alphanumeric() || c == '_');
    if !first_ok || !rest_ok {
        return Err(
            "Nur Buchstaben, Ziffern und '_' erlaubt; muss mit einem Buchstaben beginnen".into(),
        );
    }
    Ok(())
}

/// Build the error pointer string: spaces up to the error position, then "^".
pub(crate) fn error_pointer(input: &str, pos: usize) -> String {
    // Count the display width up to `pos` (each char = 1 in monospace).
    let width = input.chars().take(pos).count();
    let mut s = String::with_capacity(width + 1);
    for _ in 0..width {
        s.push(' ');
    }
    s.push('^');
    s
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(expr: &str) -> Result<f64, ParseError> {
        evaluate(expr, &HashMap::new())
    }

    fn eval_with(expr: &str, vars: &[(&str, f64)]) -> Result<f64, ParseError> {
        let map: HashMap<String, f64> = vars.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        evaluate(expr, &map)
    }

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    // basic arithmetic
    #[test]
    fn test_basic() {
        assert!(approx(eval("2+3").unwrap(), 5.0));
        assert!(approx(eval("10-3*2").unwrap(), 4.0));
        assert!(approx(eval("(1+2)*3").unwrap(), 9.0));
        assert!(approx(eval("2^10").unwrap(), 1024.0));
    }

    // functions
    #[test]
    fn test_functions() {
        assert!(approx(eval("sin(0)").unwrap(), 0.0));
        assert!(approx(eval("cos(0)").unwrap(), 1.0));
        assert!(approx(eval("sqrt(4)").unwrap(), 2.0));
        assert!(approx(eval("abs(-5)").unwrap(), 5.0));
        assert!(approx(eval("log(100)").unwrap(), 2.0));
        assert!(approx(eval("ln(e)").unwrap(), 1.0));
        assert!(approx(eval("floor(3.7)").unwrap(), 3.0));
        assert!(approx(eval("ceil(3.2)").unwrap(), 4.0));
    }

    // implicit multiplication
    #[test]
    fn test_implicit_mult() {
        assert!(approx(eval("2pi").unwrap(), 2.0 * std::f64::consts::PI));
        assert!(approx(eval("3(4+5)").unwrap(), 27.0));
        assert!(approx(eval("(2)(3)").unwrap(), 6.0));
        assert!(approx(eval("2sin(0)").unwrap(), 0.0));
        assert!(approx(eval("2cos(0)").unwrap(), 2.0));
    }

    // factorial
    #[test]
    fn test_factorial() {
        assert!(approx(eval("0!").unwrap(), 1.0));
        assert!(approx(eval("5!").unwrap(), 120.0));
        assert!(approx(eval("3!^2").unwrap(), 36.0));
        assert!(eval("(-1)!").is_err());
        assert!(eval("2.5!").is_err());
    }

    // variables
    #[test]
    fn test_variables() {
        assert!(approx(eval_with("x+1", &[("x", 5.0)]).unwrap(), 6.0));
        assert!(approx(eval_with("2x", &[("x", 5.0)]).unwrap(), 10.0));
    }

    // errors
    #[test]
    fn test_errors() {
        assert!(eval("1/0").is_err());
        assert!(eval("sqrt(-1)").is_err());
        assert!(eval("unknown(3)").is_err());
    }

    // error positions
    #[test]
    fn test_error_positions() {
        // "1+}" → error at position 2 (the '}')
        let err = eval("1+}").unwrap_err();
        assert_eq!(err.pos, Some(2));

        // "1 + }" → error at position 4 (whitespace-aware)
        let err = eval("1 + }").unwrap_err();
        assert_eq!(err.pos, Some(4));

        // "(1+2" → missing ')' at position 4
        let err = eval("(1+2").unwrap_err();
        assert_eq!(err.pos, Some(4));

        // "1/0" → division by zero at '/'
        let err = eval("1/0").unwrap_err();
        assert_eq!(err.pos, Some(1));

        // unknown variable
        let err = eval("foo+1").unwrap_err();
        assert_eq!(err.pos, Some(0));
    }

    // error pointer string
    #[test]
    fn test_error_pointer() {
        assert_eq!(error_pointer("1+}", 2), "  ^");
        assert_eq!(error_pointer("1 + }", 4), "    ^");
        assert_eq!(error_pointer("x", 0), "^");
    }

    // validation
    #[test]
    fn test_validate_var_name() {
        assert!(validate_var_name("x").is_ok());
        assert!(validate_var_name("my_var").is_ok());
        assert!(validate_var_name("").is_err());
        assert!(validate_var_name("pi").is_err());
        assert!(validate_var_name("sin").is_err());
        assert!(validate_var_name("1x").is_err());
    }
}
