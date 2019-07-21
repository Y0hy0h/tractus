
use itertools::Itertools;
use pest::Parser;
#[derive(Parser)]
#[grammar = "r.pest"]
struct RParser;

#[derive(PartialEq, Debug)]
pub enum RStmt {
    Empty,
    Comment(String),
    Assignment(RExp, RExp),
    Library(RIdentifier),
    Expression(RExp),
}

#[derive(PartialEq, Debug)]
pub enum RExp {
    Constant(String),
    Variable(RIdentifier),
    Call(RIdentifier, Vec<(Option<RIdentifier>, RExp)>),
    Column(Box<RExp>, Box<RExp>),
    Index(Box<RExp>, Vec<RExp>),
    Formula(RFormula),
    Function(Vec<(RIdentifier, Option<RExp>)>, String),
    Infix(String, Box<RExp>, Box<RExp>),
}

impl RExp {
    pub fn constant(content: &'static str) -> RExp {
        RExp::Constant(content.to_string())
    }

    pub fn variable(content: &'static str) -> RExp {
        RExp::Variable(content.to_string())
    }
}

#[derive(PartialEq, Debug)]
pub enum RFormula {
    OneSided(RFormulaExpression),
    TwoSided(RIdentifier, RFormulaExpression),
}

#[derive(PartialEq, Debug)]
pub enum RFormulaExpression {
    Variable(RIdentifier),
    Plus(Box<RFormulaExpression>, RIdentifier),
    Minus(Box<RFormulaExpression>, RIdentifier),
    Colon(Box<RFormulaExpression>, RIdentifier),
    Star(Box<RFormulaExpression>, RIdentifier),
    In(Box<RFormulaExpression>, RIdentifier),
    Hat(Box<RFormulaExpression>, RIdentifier),
}

pub type RIdentifier = String;

type Error = pest::error::Error<Rule>;

pub fn parse(code: &str) -> Result<Vec<RStmt>, Error> {
    let mut parse_result = RParser::parse(Rule::file, code)?;

    let file = parse_result.next().unwrap();
    Ok(file
        .into_inner()
        .filter_map(|token| match token.as_rule() {
            Rule::statement => {
                let maybe_line = token.into_inner().next();
                match maybe_line {
                    None => None, // empty line
                    Some(line) => {
                        match line.as_rule() {
                            Rule::comment => Some(RStmt::Comment(line.as_str().to_string())),
                            Rule::expression => {
                                Some(RStmt::Expression(parse_expression(line))) // Expression is always non-empty.
                            }
                            Rule::assignment => {
                                let mut assignment = line.into_inner();
                                let left = assignment.next().unwrap(); // Left-hand side always exists.
                                let right = assignment.next().unwrap(); // Righ-hand side always exists.
                                let left = parse_expression(left);
                                let right = parse_expression(right);
                                Some(RStmt::Assignment(left, right))
                            }
                            Rule::library => {
                                let name = line.into_inner().next().unwrap(); // Library name always exists.
                                Some(RStmt::Library(name.as_str().into()))
                            }
                            _ => unreachable!(),
                        }
                    }
                }
            }
            Rule::EOI => None,
            _ => unreachable!(),
        })
        .collect())
}

fn parse_expression(expression_pair: pest::iterators::Pair<'_, Rule>) -> RExp {
    let mut whole_expression = expression_pair.into_inner();
    let expression = whole_expression.next().unwrap(); // Expression is always non-empty.
    let mut rexp = match expression.as_rule() {
        Rule::constant => RExp::Constant(expression.as_str().to_string()),
        Rule::identifier => RExp::Variable(expression.as_str().to_string()),
        Rule::function_call => {
            let mut function = expression.into_inner();
            let name = function.next().unwrap(); // Function name always exists.
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
            RExp::Call(name.as_str().into(), args)
        }
        Rule::formula => {
            let formula_kind = expression.into_inner().next().unwrap(); // Formula always has a kind.
            println!("{:?}", formula_kind);
            match formula_kind.as_rule() {
                Rule::one_sided => {
                    let right = formula_kind.into_inner();
                    RExp::Formula(RFormula::OneSided(parse_formula_expression(right)))
                }
                Rule::two_sided => {
                    let mut formula = formula_kind.into_inner();
                    let left = formula.next().unwrap(); // Two-sided formula always has left side.
                    RExp::Formula(RFormula::TwoSided(
                        left.as_str().into(),
                        parse_formula_expression(formula),
                    ))
                }
                _ => unreachable!(),
            }
        }
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
            let body = function.next().unwrap(); // Function always has a body.
            RExp::Function(args, body.as_str().into())
        }
        _ => unreachable!(),
    };

    // Process all indexing expressions that follow.
    for infix in whole_expression {
        match infix.as_rule() {
            Rule::column => rexp = RExp::Column(Box::new(rexp), Box::new(parse_expression(infix))),
            Rule::index => {
                let indices = infix.into_inner().map(parse_expression).collect();
                rexp = RExp::Index(Box::new(rexp), indices)
            }
            Rule::infix => {
                let mut infix_operator = infix.into_inner();
                let operator = infix_operator.next().unwrap(); // Operator is always present.
                let right = infix_operator.next().unwrap(); // Infix operator always has right-hand side.
                rexp = RExp::Infix(operator.as_str().into(), Box::new(rexp), Box::new(parse_expression(right)));
            }
            _ => unreachable!(),
        }
    }

    rexp
}

fn parse_formula_expression(
    mut expression: pest::iterators::Pairs<'_, Rule>,
) -> RFormulaExpression {
    let first = expression.next().unwrap();
    let mut result = RFormulaExpression::Variable(first.as_str().into()); // Right-hand side of formula always has at least one element.
    for (operator, right) in expression.tuples() {
        match operator.as_str() {
            "+" => result = RFormulaExpression::Plus(Box::new(result), right.as_str().into()),
            _ => unreachable!(),
        }
    }
    result
}