/// A contextual keyword.
#[derive(Debug, Clone, PartialEq)]
pub enum Keyword {
    Module,
    Struct,
    Enum,
    Union,
    Trait,
    Function,
    Using,
    Const,
    Let,
    If,
    Else,
    While,
    For,
    Loop,
    Break,
    Continue,
    Return,
    Match,
    Try,
    Catch,
}
