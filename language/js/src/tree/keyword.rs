use std::str::FromStr;

use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

macro_rules! keywords {
    ($($variant:ident => $text:literal, $is_reserved:literal;)*) => {
        /// One JavaScript keyword or contextual token.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
        pub enum Keyword {
            $(#[doc = concat!("The `", $text, "` token.")] $variant,)*
        }

        impl Keyword {
            /// Return the canonical JavaScript text.
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text,)*
                }
            }

            /// Return whether this token cannot be used as an identifier.
            pub const fn is_reserved(self) -> bool {
                match self {
                    $(Self::$variant => $is_reserved,)*
                }
            }
        }

        impl FromStr for Keyword {
            type Err = ();

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                match value {
                    $($text => Ok(Self::$variant),)*
                    _ => Err(()),
                }
            }
        }
    };
}

keywords! {
    Public => "public", true;
    Protected => "protected", true;
    Private => "private", true;
    Static => "static", true;
    Accessor => "accessor", false;

    This => "this", true;
    Super => "super", true;

    Import => "import", true;
    Export => "export", true;
    From => "from", false;
    Default => "default", true;
    As => "as", false;

    Const => "const", true;
    Let => "let", true;
    Var => "var", true;
    Class => "class", true;
    Function => "function", true;
    Enum => "enum", true;
    Interface => "interface", true;
    Package => "package", true;
    New => "new", true;
    Delete => "delete", true;

    Extends => "extends", true;
    Implements => "implements", true;
    Assert => "assert", false;
    InstanceOf => "instanceof", true;
    Typeof => "typeof", true;
    In => "in", true;
    Of => "of", false;
    Void => "void", true;

    True => "true", true;
    False => "false", true;
    Null => "null", true;

    If => "if", true;
    Else => "else", true;
    Switch => "switch", true;
    Case => "case", true;

    Do => "do", true;
    While => "while", true;
    For => "for", true;

    Break => "break", true;
    Continue => "continue", true;
    Return => "return", true;
    Yield => "yield", true;
    With => "with", true;
    Debugger => "debugger", true;
    Using => "using", false;

    Try => "try", true;
    Catch => "catch", true;
    Throw => "throw", true;
    Finally => "finally", true;

    Async => "async", false;
    Await => "await", true;
    Get => "get", false;
    Constructor => "constructor", false;
    Set => "set", false;
}
