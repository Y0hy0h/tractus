use std::fmt::Write;

use itertools::Itertools;
use pest::Parser;

#[derive(Parser)]
#[grammar = "r.pest"]
struct RParser;

#[derive(Eq, PartialEq, Debug, Clone, Hash)]
pub struct Lines(Vec<RStmt>);

impl Lines {
    fn vec(&self) -> &Vec<RStmt> {
        &self.0
    }
}

impl std::fmt::Display for Lines {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.vec().iter().join("\n"))
    }
}

impl From<Vec<RStmt>> for Lines {
    fn from(other: Vec<RStmt>) -> Lines {
        Lines(other)
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub enum RStmt {
    Empty,
    Comment(String),
    TailComment(Box<RStmt>, String),
    Assignment(RExp, Vec<RExp>, RExp),
    If(RExp, Lines, Option<Lines>),
    While(RExp, Lines),
    For(RExp, RExp, Lines),
    Library(RIdentifier),
    Expression(RExp),
}

impl RStmt {
    pub fn expression(&self) -> Option<&RExp> {
        use RStmt::*;
        match self {
            Assignment(_, _, expression) => Some(expression),
            Expression(expression) => Some(expression),
            TailComment(statement, _) => statement.expression(),
            // TODO: Check how to handle if, while, and for.
            If(_, _, _) => None,
            For(_, _, _) => None,
            While(_, _) => None,
            Empty => None,
            Comment(_) => None,
            Library(_) => None,
        }
    }
}

impl std::fmt::Display for RStmt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use RStmt::*;
        match self {
            Empty => writeln!(f),
            Comment(text) => write!(f, "{}", text),
            TailComment(expression, text) => write!(f, "{} {}", expression, text),
            Assignment(left, additional, right) => {
                let mut assigned = vec![left];
                assigned.append(&mut additional.iter().collect());
                for variable in assigned.iter() {
                    write!(f, "{} <- ", variable)?
                }
                write!(f, "{}", right)
            }
            If(condition, body, maybe_else_body) => {
                write!(f, "if ({}) {{\n{}\n}}", condition, body)?;
                if let Some(else_body) = maybe_else_body {
                    write!(f, "\nelse {{\n{}\n}}", else_body)?;
                }
                Ok(())
            }
            While(condition, body) => write!(f, "while ({}) {{\n{}\n}}", condition, body),
            For(variable, range, body) => {
                write!(f, "for ({} in {}) {{\n{}\n}}", variable, range, body)
            }
            Library(name) => write!(f, "{}", name),
            Expression(exp) => write!(f, "{}", exp),
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub enum RExp {
    Constant(String),
    Variable(RIdentifier),
    Call(Box<RExp>, Vec<(Option<RIdentifier>, RExp)>),
    Column(Box<RExp>, Box<RExp>),
    Index(Box<RExp>, Vec<Option<RExp>>),
    ListIndex(Box<RExp>, Vec<Option<RExp>>),
    Formula(RFormula),
    Function(Vec<(RIdentifier, Option<RExp>)>, Lines),
    Prefix(String, Box<RExp>),
    Infix(String, Box<RExp>, Box<RExp>),
}

impl RExp {
    pub fn constant(content: &'static str) -> RExp {
        RExp::Constant(content.to_string())
    }

    pub fn variable(content: &'static str) -> RExp {
        RExp::Variable(content.to_string())
    }

    pub fn boxed_constant(content: impl Into<String>) -> Box<RExp> {
        Box::new(RExp::Constant(content.into()))
    }

    pub fn boxed_variable(content: impl Into<String>) -> Box<RExp> {
        Box::new(RExp::Variable(content.into()))
    }

    pub fn extract_variable_name(&self) -> Option<RIdentifier> {
        use RExp::*;
        match self {
            Variable(name) => Some(name.to_string()),
            Column(left, _) => left.extract_variable_name(),
            Index(left, _) => left.extract_variable_name(),
            Call(name, args) => {
                // `colnames(variable) <- c("a", "b", "c")` is valid R.
                let valid_functions = ["colnames", "rownames", "names"];
                let function_name = name
                    .extract_variable_name()
                    .unwrap_or_else(|| panic!("Expected function name in {}.", name));
                if valid_functions.iter().any(|f| f == &function_name) && args.len() == 1 {
                    let (_, exp) = &args[0];
                    exp.extract_variable_name()
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

impl std::fmt::Display for RExp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use RExp::*;
        match self {
            Constant(constant) => write!(f, "{}", constant),
            Variable(name) => write!(f, "{}", name),
            Call(name, args) => {
                let arguments = args
                    .iter()
                    .map(|(maybe_name, expression)| {
                        let mut s = String::new();
                        if let Some(name) = maybe_name {
                            write!(s, "{} = ", name)?;
                        }
                        write!(s, "{}", expression)?;
                        Ok(s)
                    })
                    .collect::<Result<Vec<String>, std::fmt::Error>>()?
                    .join(", ");
                write!(f, "{}({})", name, arguments)
            }
            Column(left, right) => write!(f, "{}${}", left, right),
            Index(left, right) => {
                let indices = right
                    .iter()
                    .map(|maybe_expression| match maybe_expression {
                        Some(expression) => format!("{}", expression),
                        None => "".to_string(),
                    })
                    .collect::<Vec<String>>()
                    .join(", ");
                write!(f, "{}[{}]", left, indices)
            }
            ListIndex(left, right) => {
                let indices = right
                    .iter()
                    .map(|maybe_expression| match maybe_expression {
                        Some(expression) => format!("{}", expression),
                        None => "".to_string(),
                    })
                    .collect::<Vec<String>>()
                    .join(", ");
                write!(f, "{}[[{}]]", left, indices)
            }
            Formula(formula) => write!(f, "{}", formula),
            Function(params, body) => {
                let parameters = params
                    .iter()
                    .map(|(name, maybe_default)| {
                        let mut s = String::new();
                        write!(s, "{}", name)?;
                        if let Some(default) = maybe_default {
                            write!(s, " = {}", default)?;
                        }
                        Ok(s)
                    })
                    .collect::<Result<Vec<String>, std::fmt::Error>>()?
                    .join(", ");
                write!(f, "function ({}) {{\n{}\n}}", parameters, body)
            }
            Prefix(op, exp) => write!(f, "{}{}", op, exp),
            Infix(op, left, right) => write!(f, "{} {} {}", left, op, right),
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub enum RFormula {
    OneSided(Box<RExp>),
    TwoSided(Box<RExp>, Box<RExp>),
}

impl std::fmt::Display for RFormula {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use RFormula::*;
        match self {
            OneSided(right) => write!(f, "~ {}", right),
            TwoSided(left, right) => write!(f, "{} ~ {}", left, right),
        }
    }
}

pub type RIdentifier = String;

pub type Error = pest::error::Error<Rule>;

/// Helper macro for use instead of `unreachable!()` that outputs more information.
macro_rules! unexpected_rule {
    ( $rule:ident, $pair:ident) => {
        panic!(
            "Encountered unexpected rule {:?} for input {:#?}.",
            $rule,
            $pair.as_str()
        )
    };
}

pub fn parse(code: &str) -> Result<Vec<RStmt>, Error> {
    let parse_result = RParser::parse(Rule::file, code)?;

    Ok(parse_result
        .filter_map(|token| match token.as_rule() {
            Rule::EOI => None,
            _ => Some(parse_line(token)),
        })
        .collect())
}

fn parse_line(line_pair: pest::iterators::Pair<Rule>) -> RStmt {
    match line_pair.as_rule() {
        Rule::empty => RStmt::Empty,
        Rule::line => {
            let mut line = line_pair.into_inner();
            let first_pair = line.next().unwrap(); // A line always contains at least a statement or a comment.
            let first = match first_pair.as_rule() {
                Rule::statement => {
                    let statement = first_pair.into_inner().next().unwrap(); // Take statement out of line.
                    match statement.as_rule() {
                        Rule::expression => {
                            RStmt::Expression(parse_expression(statement)) // Expression is always non-empty.
                        }
                        Rule::assignment => {
                            // Can be multiple assignment, e. g. a=b=c=1. We want to extract the right-most expression,
                            // wich is assigned to all others, and the left-most one, which prevents an empty left side.
                            let mut elements: Vec<RExp> =
                                statement.into_inner().map(parse_expression).collect();
                            let error = "Assignment did not have enough elements.";
                            let right = elements.pop().expect(error);
                            if elements.is_empty() {
                                panic!(error);
                            }
                            let left = elements.remove(0);
                            let additional = elements;
                            RStmt::Assignment(left, additional, right)
                        }
                        Rule::if_statement => {
                            let mut elements = statement.into_inner();
                            let condition = if let Some(condition_pair) = elements.next() {
                                parse_expression(condition_pair)
                            } else {
                                panic!("If statement did not have enough elements.");
                            };
                            let body = elements.next().unwrap().into_inner(); // If statement always has a body.
                            let body: Vec<RStmt> = body.map(parse_line).collect();

                            let else_body = elements.next().map(|else_body| {
                                Lines::from(
                                    else_body
                                        .into_inner()
                                        .map(parse_line)
                                        .collect::<Vec<RStmt>>(),
                                )
                            });

                            RStmt::If(condition, Lines::from(body), else_body)
                        }
                        Rule::while_statement => {
                            let mut elements = statement.into_inner();
                            let condition = if let Some(condition_pair) = elements.next() {
                                parse_expression(condition_pair)
                            } else {
                                panic!("While statement did not have enough elements.");
                            };
                            let body = elements.next().unwrap().into_inner(); // For statement always has a body.
                            let body: Vec<RStmt> = body.map(parse_line).collect();
                            RStmt::While(condition, Lines::from(body))
                        }
                        Rule::for_statement => {
                            let mut elements = statement.into_inner();
                            let (pattern, range) = if let (Some(pattern_pair), Some(range_pair)) =
                                (elements.next(), elements.next())
                            {
                                (parse_expression(pattern_pair), parse_expression(range_pair))
                            } else {
                                panic!("For statement did not have enough elements.");
                            };
                            let body = elements.next().unwrap().into_inner(); // For statement always has a body.
                            let body: Vec<RStmt> = body.map(parse_line).collect();
                            RStmt::For(pattern, range, Lines::from(body))
                        }
                        Rule::library => {
                            let name = statement.into_inner().next().unwrap(); // Library name always exists.
                            RStmt::Library(name.as_str().into())
                        }
                        r => unexpected_rule!(r, statement),
                    }
                }
                Rule::comment => RStmt::Comment(first_pair.as_str().to_string()),
                r => unexpected_rule!(r, first_pair),
            };

            if let Some(comment) = line.next() {
                // Second line component has to be a comment.
                RStmt::TailComment(Box::new(first), comment.as_str().to_string())
            } else {
                first
            }
        }
        r => unexpected_rule!(r, line_pair),
    }
}

fn parse_expression(expression_pair: pest::iterators::Pair<Rule>) -> RExp {
    let mut whole_expression = expression_pair.into_inner();
    let expression = whole_expression.next().unwrap(); // Expression is always non-empty.
    let mut rexp = match expression.as_rule() {
        Rule::constant => RExp::Constant(expression.as_str().to_string()),
        Rule::identifier => RExp::Variable(expression.as_str().to_string()),
        Rule::prefix => {
            let mut prefix_expression = expression.into_inner();
            let operator = prefix_expression.next().unwrap(); // Prefix always has operator.
            let exp = prefix_expression.next().unwrap(); // Prefix always has expression.
            RExp::Prefix(
                operator.as_str().to_string(),
                Box::new(parse_expression(exp)),
            )
        }
        Rule::formula => RExp::Formula(RFormula::OneSided(Box::new(parse_expression(expression)))),
        Rule::function_definition => {
            let mut function = expression.into_inner();
            let args = function.next().unwrap(); // Function always has (possibly empty) arguments.
            let args: Vec<(RIdentifier, Option<RExp>)> = args
                .into_inner()
                .map(|arg| {
                    match arg.as_rule() {
                        Rule::required_parameter => (arg.as_str().into(), None),
                        Rule::parameter_with_default => {
                            let (arg, expression) = arg.into_inner().next_tuple().unwrap(); // Parameter with default always has name and default value.
                            (arg.as_str().into(), Some(parse_expression(expression)))
                        }
                        _ => unreachable!(),
                    }
                })
                .collect();
            let body = function.next().unwrap().into_inner(); // Function always has a body.
            let body: Vec<RStmt> = body.map(parse_line).collect();
            RExp::Function(args, Lines::from(body))
        }
        Rule::expression => parse_expression(expression),
        r => unexpected_rule!(r, expression),
    };

    // Process all indexing expressions that follow.
    for infix in whole_expression {
        match infix.as_rule() {
            Rule::function_call => rexp = parse_function_expression(rexp, infix),
            Rule::column => rexp = RExp::Column(Box::new(rexp), Box::new(parse_expression(infix))),
            Rule::index => {
                let indices = infix
                    .into_inner()
                    .map(|maybe_expression| match maybe_expression.as_rule() {
                        Rule::expression => Some(parse_expression(maybe_expression)),
                        Rule::empty => None,
                        _ => unreachable!(),
                    })
                    .collect();
                rexp = RExp::Index(Box::new(rexp), indices)
            }
            Rule::list_index => {
                let indices = infix
                    .into_inner()
                    .map(|maybe_expression| match maybe_expression.as_rule() {
                        Rule::expression => Some(parse_expression(maybe_expression)),
                        Rule::empty => None,
                        _ => unreachable!(),
                    })
                    .collect();
                rexp = RExp::ListIndex(Box::new(rexp), indices)
            }
            Rule::infix => {
                let mut infix_operator = infix.into_inner();
                let operator = infix_operator.next().unwrap(); // Operator is always present.
                let right = infix_operator.next().unwrap(); // Infix operator always has right-hand side.
                rexp = RExp::Infix(
                    operator.as_str().into(),
                    Box::new(rexp),
                    Box::new(parse_expression(right)),
                );
            }
            Rule::formula => {
                rexp = RExp::Formula(RFormula::TwoSided(
                    Box::new(rexp),
                    Box::new(parse_expression(infix)),
                ));
            }
            r => unexpected_rule!(r, infix),
        }
    }

    rexp
}

fn parse_function_expression(expression: RExp, function_pair: pest::iterators::Pair<Rule>) -> RExp {
    let mut function = function_pair.into_inner();
    let maybe_arguments = function.next();
    let args: Vec<(Option<RIdentifier>, RExp)> = match maybe_arguments {
        Some(args) => {
            args.into_inner()
                .map(|arg| {
                    match arg.as_rule() {
                        Rule::named_argument => {
                            let mut argument = arg.into_inner();
                            let key = argument.next().unwrap(); // Key always exists.
                            let value = argument.next().unwrap(); // Value always exists.
                            let value = parse_expression(value);
                            (Some(key.as_str().to_string()), value)
                        }
                        Rule::unnamed_argument => {
                            let value = arg.into_inner().next().unwrap(); // Argument's value always exists.
                            let value = parse_expression(value);
                            (None, value)
                        }
                        _ => unreachable!(),
                    }
                })
                .collect()
        }
        None => vec![],
    };
    RExp::Call(Box::new(expression), args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn test_parse(code: &'static str) -> Vec<RStmt> {
        parse(code).unwrap_or_else(|e| panic!("{}", e))
    }

    #[test]
    fn parses_comments() {
        let code = "\
#123
hello() # world
# another thing   ";
        let result = test_parse(code);
        let expected = vec![
            RStmt::Comment("#123".into()),
            RStmt::TailComment(
                Box::new(RStmt::Expression(RExp::Call(
                    RExp::boxed_variable("hello"),
                    vec![],
                ))),
                "# world".into(),
            ),
            RStmt::Comment("# another thing   ".into()),
        ];
        assert_eq!(expected, result);
    }

    #[test]
    fn parses_empty_lines() {
        let code = "
# First block
a <- 1

# Second block
b <- 1
c <- 2


# Third block
d <- 1


";
        let result = test_parse(code);
        let expected = vec![
            RStmt::Empty,
            RStmt::Comment("# First block".into()),
            RStmt::Assignment(RExp::variable("a"), vec![], RExp::constant("1")),
            RStmt::Empty,
            RStmt::Comment("# Second block".into()),
            RStmt::Assignment(RExp::variable("b"), vec![], RExp::constant("1")),
            RStmt::Assignment(RExp::variable("c"), vec![], RExp::constant("2")),
            RStmt::Empty,
            RStmt::Empty,
            RStmt::Comment("# Third block".into()),
            RStmt::Assignment(RExp::variable("d"), vec![], RExp::constant("1")),
            RStmt::Empty,
            RStmt::Empty,
        ];
        assert_eq!(expected, result);
    }

    #[test]
    fn parses_assignments() {
        let code = "\
a <- 1
b = 2
a=b=c=1
colnames(something) <- c(\"R\", \"is\", \"crazy\")";
        let result = test_parse(code);
        let expected = vec![
            RStmt::Assignment(RExp::variable("a"), vec![], RExp::constant("1")),
            RStmt::Assignment(RExp::variable("b"), vec![], RExp::constant("2")),
            RStmt::Assignment(
                RExp::variable("a"),
                vec![RExp::variable("b"), RExp::variable("c")],
                RExp::constant("1"),
            ),
            RStmt::Assignment(
                RExp::Call(
                    RExp::boxed_variable("colnames"),
                    vec![(None, RExp::variable("something"))],
                ),
                vec![],
                RExp::Call(
                    RExp::boxed_variable("c"),
                    vec![
                        (None, RExp::constant("\"R\"")),
                        (None, RExp::constant("\"is\"")),
                        (None, RExp::constant("\"crazy\"")),
                    ],
                ),
            ),
        ];
        assert_eq!(expected, result);
    }

    #[test]
    fn parses_function_calls() {
        let code = "\
empty()
single(1)
with_args(1, x, name = value)
break_down(
    \"long\",
    argument=\"chains\"
    )
name::space()
higher_order()(10)";
        let result = test_parse(code);
        let expected = vec![
            RStmt::Expression(RExp::Call(RExp::boxed_variable("empty"), vec![])),
            RStmt::Expression(RExp::Call(
                RExp::boxed_variable("single"),
                vec![(None, RExp::constant("1"))],
            )),
            RStmt::Expression(RExp::Call(
                RExp::boxed_variable("with_args"),
                vec![
                    (None, RExp::constant("1")),
                    (None, RExp::variable("x")),
                    (Some("name".to_string()), RExp::variable("value")),
                ],
            )),
            RStmt::Expression(RExp::Call(
                RExp::boxed_variable("break_down"),
                vec![
                    (None, RExp::constant("\"long\"")),
                    (Some("argument".to_string()), RExp::constant("\"chains\"")),
                ],
            )),
            RStmt::Expression(RExp::Call(RExp::boxed_variable("name::space"), vec![])),
            RStmt::Expression(RExp::Call(
                Box::new(RExp::Call(RExp::boxed_variable("higher_order"), vec![])),
                vec![(None, RExp::constant("10"))],
            )),
        ];
        assert_eq!(expected, result);
    }

    #[test]
    fn parses_strings() {
        let code = "\
'first'
\"second\"
`third`";
        let result = test_parse(code);
        let expected = vec![
            RStmt::Expression(RExp::constant("'first'")),
            RStmt::Expression(RExp::constant("\"second\"")),
            RStmt::Expression(RExp::constant("`third`")),
        ];
        assert_eq!(expected, result);
    }

    #[test]
    fn parses_numbers() {
        let code = "\
1
.20
0.10
-2
2e-30
+3.4e+1";
        let result = test_parse(code);
        let expected = vec![
            RStmt::Expression(RExp::constant("1")),
            RStmt::Expression(RExp::constant(".20")),
            RStmt::Expression(RExp::constant("0.10")),
            RStmt::Expression(RExp::Prefix("-".into(), Box::new(RExp::constant("2")))),
            RStmt::Expression(RExp::constant("2e-30")),
            RStmt::Expression(RExp::Prefix("+".into(), Box::new(RExp::constant("3.4e+1")))),
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
item[[1]]
other[multiple, index, arguments]
list[[1,2]]
get_matrix()$column[1]
item[empty,]";
        let result = test_parse(code);
        let expected = vec![
            RStmt::Expression(RExp::Column(
                Box::new(RExp::variable("item")),
                Box::new(RExp::variable("column")),
            )),
            RStmt::Expression(RExp::Index(
                Box::new(RExp::variable("item")),
                vec![Some(RExp::Column(
                    Box::new(RExp::variable("other")),
                    Box::new(RExp::variable("thing")),
                ))],
            )),
            RStmt::Expression(RExp::ListIndex(
                Box::new(RExp::variable("item")),
                vec![Some(RExp::constant("1"))],
            )),
            RStmt::Expression(RExp::Index(
                Box::new(RExp::variable("other")),
                vec![
                    Some(RExp::variable("multiple")),
                    Some(RExp::variable("index")),
                    Some(RExp::variable("arguments")),
                ],
            )),
            RStmt::Expression(RExp::ListIndex(
                Box::new(RExp::variable("list")),
                vec![Some(RExp::constant("1")), Some(RExp::constant("2"))],
            )),
            RStmt::Expression(RExp::Index(
                Box::new(RExp::Column(
                    Box::new(RExp::Call(RExp::boxed_variable("get_matrix"), vec![])),
                    Box::new(RExp::variable("column")),
                )),
                vec![Some(RExp::constant("1"))],
            )),
            RStmt::Expression(RExp::Index(
                Box::new(RExp::variable("item")),
                vec![Some(RExp::variable("empty")), None],
            )),
        ];
        assert_eq!(expected, result);
    }

    #[test]
    fn parses_formulae() {
        let code = "\
~ one_sided
two ~ sided
~ one + sided + multiple
two ~ sided + 1
~ transform(x)
other ~ transform(x)
lm(y[subk]~factor(x[subk]))";
        let result = test_parse(code);
        let expected = vec![
            RStmt::Expression(RExp::Formula(RFormula::OneSided(Box::new(RExp::Variable(
                "one_sided".into(),
            ))))),
            RStmt::Expression(RExp::Formula(RFormula::TwoSided(
                Box::new(RExp::variable("two")),
                Box::new(RExp::Variable("sided".into())),
            ))),
            RStmt::Expression(RExp::Formula(RFormula::OneSided(Box::new(RExp::Infix(
                "+".into(),
                Box::new(RExp::variable("one")),
                Box::new(RExp::Infix(
                    "+".into(),
                    Box::new(RExp::variable("sided")),
                    Box::new(RExp::variable("multiple")),
                )),
            ))))),
            RStmt::Expression(RExp::Formula(RFormula::TwoSided(
                Box::new(RExp::variable("two")),
                Box::new(RExp::Infix(
                    "+".into(),
                    Box::new(RExp::variable("sided")),
                    Box::new(RExp::constant("1")),
                )),
            ))),
            RStmt::Expression(RExp::Formula(RFormula::OneSided(Box::new(RExp::Call(
                RExp::boxed_variable("transform"),
                vec![(None, RExp::Variable("x".into()))],
            ))))),
            RStmt::Expression(RExp::Formula(RFormula::TwoSided(
                Box::new(RExp::variable("other")),
                Box::new(RExp::Call(
                    RExp::boxed_variable("transform"),
                    vec![(None, RExp::Variable("x".into()))],
                )),
            ))),
            RStmt::Expression(RExp::Call(
                RExp::boxed_variable("lm"),
                vec![(
                    None,
                    RExp::Formula(RFormula::TwoSided(
                        Box::new(RExp::Index(
                            Box::new(RExp::variable("y")),
                            vec![Some(RExp::variable("subk"))],
                        )),
                        Box::new(RExp::Call(
                            RExp::boxed_variable("factor"),
                            vec![(
                                None,
                                RExp::Index(
                                    Box::new(RExp::variable("x")),
                                    vec![Some(RExp::variable("subk"))],
                                ),
                            )],
                        )),
                    )),
                )],
            )),
        ];
        assert_eq!(expected, result);
    }

    #[test]
    fn parses_function_definition() {
        let code = "\
func1 <- function () {
    1
}
func2 <- function (with, arguments)
    { 2 }
func3 <- function (with, default = 'arguments') {
    a <- other()
    a
} ";
        let result = test_parse(code);
        let expected = vec![
            RStmt::Assignment(
                RExp::variable("func1"),
                vec![],
                RExp::Function(
                    vec![],
                    Lines::from(vec![RStmt::Expression(RExp::constant("1"))]),
                ),
            ),
            RStmt::Assignment(
                RExp::variable("func2"),
                vec![],
                RExp::Function(
                    vec![("with".into(), None), ("arguments".into(), None)],
                    Lines::from(vec![RStmt::Expression(RExp::constant("2"))]),
                ),
            ),
            RStmt::Assignment(
                RExp::variable("func3"),
                vec![],
                RExp::Function(
                    vec![
                        ("with".into(), None),
                        ("default".into(), Some(RExp::constant("'arguments'"))),
                    ],
                    Lines::from(vec![
                        RStmt::Assignment(
                            RExp::variable("a"),
                            vec![],
                            RExp::Call(RExp::boxed_variable("other"), vec![]),
                        ),
                        RStmt::Expression(RExp::variable("a")),
                    ]),
                ),
            ),
        ];
        assert_eq!(expected, result);
    }

    #[test]
    fn parses_prefix_operators() {
        let code = "\
x <- !TRUE
y <- negate(!x)
-(1 + 2)";
        let result = test_parse(code);
        let expected = vec![
            RStmt::Assignment(
                RExp::variable("x"),
                vec![],
                RExp::Prefix("!".into(), Box::new(RExp::constant("TRUE"))),
            ),
            RStmt::Assignment(
                RExp::variable("y"),
                vec![],
                RExp::Call(
                    RExp::boxed_variable("negate"),
                    vec![(
                        None,
                        RExp::Prefix("!".into(), Box::new(RExp::variable("x"))),
                    )],
                ),
            ),
            RStmt::Expression(RExp::Prefix(
                "-".into(),
                Box::new(RExp::Infix(
                    "+".into(),
                    Box::new(RExp::constant("1")),
                    Box::new(RExp::constant("2")),
                )),
            )),
        ];
        assert_eq!(expected, result);
    }

    #[test]
    fn parses_infix_operators() {
        let code = "\
1 <= 3
TRUE && FALSE
'a' %custom% 'infix'
1 +
    3";
        let result = test_parse(code);
        let expected = vec![
            RStmt::Expression(RExp::Infix(
                "<=".into(),
                Box::new(RExp::constant("1")),
                Box::new(RExp::constant("3")),
            )),
            RStmt::Expression(RExp::Infix(
                "&&".into(),
                Box::new(RExp::constant("TRUE")),
                Box::new(RExp::constant("FALSE")),
            )),
            RStmt::Expression(RExp::Infix(
                "%custom%".into(),
                Box::new(RExp::constant("'a'")),
                Box::new(RExp::constant("'infix'")),
            )),
            RStmt::Expression(RExp::Infix(
                "+".into(),
                Box::new(RExp::constant("1")),
                Box::new(RExp::constant("3")),
            )),
        ];
        assert_eq!(expected, result);
    }

    #[test]
    fn parses_expression_in_parens() {
        let code = "\
1
(2)
(1 + (2 + 3))
((1 + 2) + 3)";
        let result = test_parse(code);
        let expected = vec![
            RStmt::Expression(RExp::constant("1")),
            RStmt::Expression(RExp::constant("2")),
            RStmt::Expression(RExp::Infix(
                "+".into(),
                Box::new(RExp::constant("1")),
                Box::new(RExp::Infix(
                    "+".into(),
                    Box::new(RExp::constant("2")),
                    Box::new(RExp::constant("3")),
                )),
            )),
            RStmt::Expression(RExp::Infix(
                "+".into(),
                Box::new(RExp::Infix(
                    "+".into(),
                    Box::new(RExp::constant("1")),
                    Box::new(RExp::constant("2")),
                )),
                Box::new(RExp::constant("3")),
            )),
        ];
        assert_eq!(expected, result);
    }

    #[test]
    fn parses_if() {
        let code = "\
if (0 == 1) {
    do_something()

    do_something_else()
}
if (is_ok())
    do_something_again()

if (TRUE)
{
    is_true()
}
else {
    is_false()
}
if (FALSE)
    is_false()
else
    is_true()";
        let result = test_parse(code);
        let expected = vec![
            RStmt::If(
                RExp::Infix(
                    "==".into(),
                    Box::new(RExp::constant("0")),
                    Box::new(RExp::constant("1")),
                ),
                Lines::from(vec![
                    RStmt::Expression(RExp::Call(RExp::boxed_variable("do_something"), vec![])),
                    RStmt::Empty,
                    RStmt::Expression(RExp::Call(
                        RExp::boxed_variable("do_something_else"),
                        vec![],
                    )),
                ]),
                None,
            ),
            RStmt::If(
                RExp::Call(RExp::boxed_variable("is_ok"), vec![]),
                Lines::from(vec![RStmt::Expression(RExp::Call(
                    RExp::boxed_variable("do_something_again"),
                    vec![],
                ))]),
                None,
            ),
            RStmt::Empty,
            RStmt::If(
                RExp::constant("TRUE"),
                Lines::from(vec![RStmt::Expression(RExp::Call(
                    RExp::boxed_variable("is_true"),
                    vec![],
                ))]),
                Some(Lines::from(vec![RStmt::Expression(RExp::Call(
                    RExp::boxed_variable("is_false"),
                    vec![],
                ))])),
            ),
            RStmt::If(
                RExp::constant("FALSE"),
                Lines::from(vec![RStmt::Expression(RExp::Call(
                    RExp::boxed_variable("is_false"),
                    vec![],
                ))]),
                Some(Lines::from(vec![RStmt::Expression(RExp::Call(
                    RExp::boxed_variable("is_true"),
                    vec![],
                ))])),
            ),
        ];
        assert_eq!(expected, result);
    }

    #[test]
    fn parses_for() {
        let code = "\
for (i in something) {
    do_something_with(i)

    do_something_else()
}
for (i in get())
    do_something_again(i)
for(row in 1:15) l[[row]] = row
";
        let result = test_parse(code);
        let expected = vec![
            RStmt::For(
                RExp::variable("i"),
                RExp::variable("something"),
                Lines::from(vec![
                    RStmt::Expression(RExp::Call(
                        RExp::boxed_variable("do_something_with"),
                        vec![(None, RExp::variable("i"))],
                    )),
                    RStmt::Empty,
                    RStmt::Expression(RExp::Call(
                        RExp::boxed_variable("do_something_else"),
                        vec![],
                    )),
                ]),
            ),
            RStmt::For(
                RExp::variable("i"),
                RExp::Call(RExp::boxed_variable("get"), vec![]),
                Lines::from(vec![RStmt::Expression(RExp::Call(
                    RExp::boxed_variable("do_something_again"),
                    vec![(None, RExp::variable("i"))],
                ))]),
            ),
            RStmt::For(
                RExp::variable("row"),
                RExp::Infix(
                    ":".into(),
                    Box::new(RExp::constant("1")),
                    Box::new(RExp::constant("15")),
                ),
                Lines::from(vec![RStmt::Assignment(
                    RExp::ListIndex(
                        Box::new(RExp::variable("l")),
                        vec![Some(RExp::variable("row"))],
                    ),
                    vec![],
                    RExp::variable("row"),
                )]),
            ),
        ];
        assert_eq!(expected, result);
    }

    #[test]
    fn parses_while() {
        let code = "\
while (i < 10) {
    do_stuff()
    i <- i + 1
}
while (true)
    annoy()
";
        let result = test_parse(code);
        let expected = vec![
            RStmt::While(
                RExp::Infix(
                    "<".into(),
                    Box::new(RExp::variable("i")),
                    Box::new(RExp::constant("10")),
                ),
                Lines::from(vec![
                    RStmt::Expression(RExp::Call(RExp::boxed_variable("do_stuff"), vec![])),
                    RStmt::Assignment(
                        RExp::variable("i"),
                        vec![],
                        RExp::Infix(
                            "+".into(),
                            Box::new(RExp::variable("i")),
                            Box::new(RExp::constant("1")),
                        ),
                    ),
                ]),
            ),
            RStmt::While(
                RExp::constant("true"),
                Lines::from(vec![RStmt::Expression(RExp::Call(
                    RExp::boxed_variable("annoy"),
                    vec![],
                ))]),
            ),
        ];
        assert_eq!(expected, result);
    }

    mod extracts_variable_name {
        use crate::parser::RExp;

        #[test]
        fn from_variable() {
            let name = RExp::variable("x").extract_variable_name();
            assert_eq!(Some("x".to_string()), name);
        }

        #[test]
        fn from_column() {
            let name = RExp::Column(Box::new(RExp::variable("x")), Box::new(RExp::variable("a")))
                .extract_variable_name();
            assert_eq!(Some("x".to_string()), name);
        }

        #[test]
        fn from_index() {
            let name = RExp::Index(
                Box::new(RExp::variable("x")),
                vec![Some(RExp::variable("a"))],
            )
            .extract_variable_name();
            assert_eq!(Some("x".to_string()), name);
        }

        #[test]
        fn from_colnames() {
            let name = RExp::Call(
                RExp::boxed_variable("colnames"),
                vec![(None, RExp::variable("x"))],
            )
            .extract_variable_name();
            assert_eq!(Some("x".to_string()), name);
        }

        #[test]
        fn rejects_constants() {
            let name = RExp::constant("x").extract_variable_name();
            assert_eq!(None, name);
        }

        #[test]
        fn rejects_constant_in_column() {
            let name = RExp::Column(Box::new(RExp::constant("x")), Box::new(RExp::variable("a")))
                .extract_variable_name();
            assert_eq!(None, name);
        }
    }

}
