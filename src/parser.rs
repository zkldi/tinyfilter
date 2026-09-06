use crate::eval::{Evaluator, RuntimeValue};
use crate::lexer::{Token, TokenKind, lex};
use crate::value::{Context, Value};
use crate::{Error, LimitKind, MAX_DEPTH, MAX_NODES, Result};

pub(crate) type NodeId = usize;

macro_rules! define_functions {
	($($variant:ident => $name:literal);+ $(;)?) => {
		#[derive(Debug, Clone, Copy, PartialEq, Eq)]
		pub(crate) enum Function {
			$($variant),+
		}

		impl Function {
			#[cfg(test)]
			pub(crate) const ALL: &[Self] = &[$(Self::$variant),+];

			fn from_name(name: &str) -> Option<Self> {
				Some(match name {
					$($name => Self::$variant,)+
					_ => return None,
				})
			}

			#[cfg(test)]
			pub(crate) const fn name(self) -> &'static str {
				match self {
					$(Self::$variant => $name,)+
				}
			}
		}
	};
}

define_functions! {
    Num => "num";
    Type => "type";
    Contains => "contains";
    StartsWith => "startsWith";
    EndsWith => "endsWith";
    Min => "min";
    Max => "max";
    Abs => "abs";
    Ceil => "ceil";
    Floor => "floor";
    Round => "round";
}

#[derive(Debug, Clone)]
pub(crate) struct Program {
    pub(crate) nodes: Vec<Node>,
    pub(crate) root: NodeId,
}

impl Program {
    pub(crate) fn eval<'a>(&'a self, context: &'a Context) -> Result<RuntimeValue<'a>> {
        Evaluator::new(self, context).run()
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Node {
    Literal(Value),
    Ident(String),
    Unary(UnaryOp, NodeId),
    Binary(BinaryOp, NodeId, NodeId),
    Index(NodeId, NodeId),
    Otherwise(NodeId, NodeId),
    Conditional {
        condition: NodeId,
        consequent: NodeId,
        alternative: NodeId,
    },
    Call {
        function: Function,
        args: Vec<NodeId>,
    },
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum UnaryOp {
    Exists,
    Not,
    Positive,
    Negative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Power,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
}

impl BinaryOp {
    fn precedence(self) -> u8 {
        match self {
            Self::Or => 1,
            Self::And => 2,
            Self::Equal
            | Self::NotEqual
            | Self::Less
            | Self::LessEqual
            | Self::Greater
            | Self::GreaterEqual => 3,
            Self::Add | Self::Subtract => 5,
            Self::Multiply | Self::Divide | Self::Modulo => 6,
            Self::Power => 7,
        }
    }
}

pub(crate) fn compile(source: &str) -> Result<Program> {
    Parser::new(lex(source)?).parse()
}

struct Parser {
    tokens: Vec<Token>,
    position: usize,
    nodes: Vec<Node>,
    depth: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
            nodes: Vec::new(),
            depth: 0,
        }
    }

    fn parse(mut self) -> Result<Program> {
        let root = self.expression(0)?;
        self.expect(TokenKind::Eof, "unexpected token after expression")?;
        Ok(Program {
            nodes: self.nodes,
            root,
        })
    }

    fn expression(&mut self, minimum_precedence: u8) -> Result<NodeId> {
        self.enter()?;
        let result = self.expression_inner(minimum_precedence);
        self.depth -= 1;
        result
    }

    fn expression_inner(&mut self, minimum_precedence: u8) -> Result<NodeId> {
        let mut left = self.prefix()?;
        loop {
            left = match self.postfix(left)? {
                Some(node) => node,
                None => break,
            };
        }

        while let Some((operator, consumed)) = self.binary_operator() {
            let precedence = operator.precedence();
            if precedence < minimum_precedence {
                break;
            }
            self.position += consumed;
            let next_precedence = if operator == BinaryOp::Power {
                precedence
            } else {
                precedence + 1
            };
            let right = self.expression(next_precedence)?;
            left = self.push(Node::Binary(operator, left, right))?;
        }
        Ok(left)
    }

    fn prefix(&mut self) -> Result<NodeId> {
        if self.at(&TokenKind::Exists) {
            self.advance();
            let value = self.expression(8)?;
            return self.push(Node::Unary(UnaryOp::Exists, value));
        }
        if self.at(&TokenKind::Not) {
            self.advance();
            let value = self.expression(8)?;
            return self.push(Node::Unary(UnaryOp::Not, value));
        }
        if self.at(&TokenKind::Plus) {
            self.advance();
            let value = self.expression(8)?;
            return self.push(Node::Unary(UnaryOp::Positive, value));
        }
        if self.at(&TokenKind::Minus) {
            self.advance();
            let value = self.expression(8)?;
            return self.push(Node::Unary(UnaryOp::Negative, value));
        }
        self.primary()
    }

    fn primary(&mut self) -> Result<NodeId> {
        let token = self.current().clone();
        match token.kind {
            TokenKind::Num(value) => {
                self.advance();
                self.push(Node::Literal(Value::num(value)?))
            }
            TokenKind::String(value) => {
                self.advance();
                self.push(Node::Literal(Value::string(value)))
            }
            TokenKind::Bool(value) => {
                self.advance();
                self.push(Node::Literal(Value::bool(value)))
            }
            TokenKind::Nil => {
                self.advance();
                self.push(Node::Literal(Value::nil()))
            }
            TokenKind::If => self.if_expression(),
            TokenKind::Ident(name) => {
                self.advance();
                if self.at(&TokenKind::LeftParen) {
                    let args = self.call_arguments()?;
                    self.call(name, args)
                } else {
                    self.push(Node::Ident(name))
                }
            }
            TokenKind::LeftParen => {
                self.advance();
                let value = self.expression(0)?;
                self.expect(TokenKind::RightParen, "expected )")?;
                Ok(value)
            }
            _ => Err(Error::parse(token.offset, "expected expression")),
        }
    }

    fn postfix(&mut self, value: NodeId) -> Result<Option<NodeId>> {
        if self.at(&TokenKind::Dot) {
            self.advance();
            if self.at(&TokenKind::LeftBracket) {
                self.advance();
                let index = self.expression(0)?;
                self.expect(TokenKind::RightBracket, "expected ]")?;
                return self.push(Node::Index(value, index)).map(Some);
            }
            let name = self.take_ident("expected member name")?;
            let index = self.push(Node::Literal(Value::string(name)))?;
            return self.push(Node::Index(value, index)).map(Some);
        }
        if self.at(&TokenKind::LeftBracket) {
            self.advance();
            let index = self.expression(0)?;
            self.expect(TokenKind::RightBracket, "expected ]")?;
            return self.push(Node::Index(value, index)).map(Some);
        }
        if self.at(&TokenKind::Otherwise) {
            self.advance();
            let fallback = self.expression(8)?;
            return self.push(Node::Otherwise(value, fallback)).map(Some);
        }
        Ok(None)
    }

    fn if_expression(&mut self) -> Result<NodeId> {
        self.advance();
        let condition = self.expression(0)?;
        self.expect(TokenKind::LeftBrace, "expected { after if condition")?;
        let consequent = self.expression(0)?;
        self.expect(TokenKind::RightBrace, "expected } after if branch")?;
        if !self.at(&TokenKind::Else) {
            return Err(Error::parse(self.current().offset, "expected else"));
        }
        self.advance();
        let alternative = if self.at(&TokenKind::If) {
            self.if_expression()?
        } else {
            self.expect(TokenKind::LeftBrace, "expected { after else")?;
            let value = self.expression(0)?;
            self.expect(TokenKind::RightBrace, "expected } after else branch")?;
            value
        };
        self.push(Node::Conditional {
            condition,
            consequent,
            alternative,
        })
    }

    fn call_arguments(&mut self) -> Result<Vec<NodeId>> {
        self.expect(TokenKind::LeftParen, "expected (")?;
        let mut args = Vec::new();
        while !self.at(&TokenKind::RightParen) {
            args.push(self.expression(0)?);
            if !self.at(&TokenKind::Comma) {
                break;
            }
            self.advance();
        }
        self.expect(TokenKind::RightParen, "expected )")?;
        Ok(args)
    }

    fn call(&mut self, name: String, args: Vec<NodeId>) -> Result<NodeId> {
        let Some(function) = Function::from_name(&name) else {
            return Err(Error::UnknownFunction("unknown function"));
        };
        self.push(Node::Call { function, args })
    }

    fn binary_operator(&self) -> Option<(BinaryOp, usize)> {
        let operator = match &self.current().kind {
            TokenKind::Or => BinaryOp::Or,
            TokenKind::And => BinaryOp::And,
            TokenKind::EqualEqual => BinaryOp::Equal,
            TokenKind::NotEqual => BinaryOp::NotEqual,
            TokenKind::Less => BinaryOp::Less,
            TokenKind::LessEqual => BinaryOp::LessEqual,
            TokenKind::Greater => BinaryOp::Greater,
            TokenKind::GreaterEqual => BinaryOp::GreaterEqual,
            TokenKind::Plus => BinaryOp::Add,
            TokenKind::Minus => BinaryOp::Subtract,
            TokenKind::Star => BinaryOp::Multiply,
            TokenKind::Slash => BinaryOp::Divide,
            TokenKind::Percent => BinaryOp::Modulo,
            TokenKind::Power => BinaryOp::Power,
            _ => return None,
        };
        Some((operator, 1))
    }

    fn push(&mut self, node: Node) -> Result<NodeId> {
        if self.nodes.len() == MAX_NODES {
            return Err(Error::limit(LimitKind::SyntaxNodes, MAX_NODES));
        }
        let id = self.nodes.len();
        self.nodes.push(node);
        Ok(id)
    }

    fn enter(&mut self) -> Result<()> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            self.depth -= 1;
            return Err(Error::limit(LimitKind::NestingDepth, MAX_DEPTH));
        }
        Ok(())
    }

    fn current(&self) -> &Token {
        &self.tokens[self.position]
    }

    fn advance(&mut self) {
        if !matches!(self.current().kind, TokenKind::Eof) {
            self.position += 1;
        }
    }

    fn at(&self, expected: &TokenKind) -> bool {
        std::mem::discriminant(&self.current().kind) == std::mem::discriminant(expected)
    }

    fn take_ident(&mut self, message: &'static str) -> Result<String> {
        if let TokenKind::Ident(value) = self.current().kind.clone() {
            self.advance();
            Ok(value)
        } else {
            Err(Error::parse(self.current().offset, message))
        }
    }

    fn expect(&mut self, expected: TokenKind, message: &'static str) -> Result<()> {
        if self.at(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(Error::parse(self.current().offset, message))
        }
    }
}
