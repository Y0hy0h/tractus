#[macro_use]
extern crate pest_derive;

mod parser;

pub use parser::{parse, RExp, RStmt};

#[cfg(test)]
mod tests {
    use crate::parser::{parse, RExp, RStmt};

    fn test_parse(code: &'static str) -> Vec<RStmt> {
        parse(code).unwrap_or_else(|e| panic!("{}", e))
    }

    #[test]
    fn parses_comments() {
        let code = "\
#123
#hello

# another thing   ";
        let result = test_parse(code);
        let expected: Vec<RStmt> = vec!["#123", "#hello", "# another thing   "]
            .iter()
            .map(|text| RStmt::Comment(text.to_string()))
            .collect();
        assert_eq!(expected, result);
    }

    #[test]
    fn parses_assignments() {
        let code = "\
a <- 1
b = 2";
        let result = test_parse(code);
        let expected = vec![
            RStmt::Assignment(RExp::variable("a"), RExp::constant("1")),
            RStmt::Assignment(RExp::variable("b"), RExp::constant("2")),
        ];
        assert_eq!(expected, result);
    }

    #[test]
    fn parses_function_calls() {
        let code = "\
empty()
single(1)
with_args(1, x, name = value)";
        let result = test_parse(code);
        let expected = vec![
            RStmt::Expression(RExp::Call("empty".into(), vec![])),

            RStmt::Expression(RExp::Call(
                "single".into(),
                vec![(None, RExp::constant("1"))],
            )),
            RStmt::Expression(RExp::Call(
                "with_args".into(),
                vec![
                    (None, RExp::constant("1")),
                    (None, RExp::variable("x")),
                    (Some("name".to_string()), RExp::variable("value")),
                ],
            )),
        ];
        assert_eq!(expected, result);
    }

    #[test]
    fn parses_strings() {
        let code = "\
'first'
\"second\"";
        let result = test_parse(code);
        let expected = vec![
            RStmt::Expression(RExp::constant("'first'")),
            RStmt::Expression(RExp::constant("\"second\"")),
        ];
        assert_eq!(expected, result);
    }

    #[test]
    fn parses_library_calls() {
        let code = "\
library(plyr)
library(MASS)";
        let result = test_parse(code);
        let expected = vec![RStmt::Library("plyr".into()), RStmt::Library("MASS".into())];
        assert_eq!(expected, result);
    }

    #[test]
    fn parses_indexing() {
        let code = "\
item$column
item[other$thing]
other[multiple, index, arguments]
get_matrix()$column[1]";
        let result = test_parse(code);
        let expected = vec![
            RStmt::Expression(RExp::Column(
                Box::new(RExp::variable("item")),
                Box::new(RExp::variable("column")),
            )),
            RStmt::Expression(RExp::Index(
                Box::new(RExp::variable("item")),
                vec![RExp::Column(
                    Box::new(RExp::variable("other")),
                    Box::new(RExp::variable("thing")),
                )],
            )),
            RStmt::Expression(RExp::Index(
                Box::new(RExp::variable("other")),
                vec![
                    RExp::variable("multiple"),
                    RExp::variable("index"),
                    RExp::variable("arguments"),
                ],
            )),
            RStmt::Expression(RExp::Index(
                Box::new(RExp::Column(
                    Box::new(RExp::Call("get_matrix".into(), vec![])),
                    Box::new(RExp::variable("column")),
                )),
                vec![RExp::constant("1")],
            )),
        ];
        assert_eq!(expected, result);
    }
}
