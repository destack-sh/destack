use dyst_source::{PathId, StringId};
use dyst_token::TokenType;

use crate::{
    Argument, Block, Definition, Node, NodeId, NodeType, Pattern, Runtime, ScalarLiteral,
    ScopedMutability, TypeLiteral, Visibility,
};

/// The operator group (for precedence parsing).
///
/// Precedence:
/// ```
/// x() x[] x{} x? x!         // postfix
/// !x -x -%x ~x *x &x ..x    // prefix
/// * / % *% *|               // multiplication
/// + - +% -% +| -|           // addition
/// << >> <<|                 // shift
/// & ^ |                     // elementwise
/// == != < > <= >=           // comparison
/// && || ?? as               // logical
/// =                         // assignment
/// *= /= %= **= *%= *|=      // assignment multiplication
/// += -= +%= -%= +|= -|=     // assignment addition
/// <<= >>= <<|=              // assignment shift
/// &= ^= |=                  // assignment elementwise
/// &&= ||=                   // assignment logical
/// ```
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum OperatorPrecedence {
    /// Unary postfix operators.
    /// `x() x[] x{} x? x!`
    Postfix = 240,
    /// Unary prefix operators.
    /// `!x -x -%x ~x &x *x ..x`
    Prefix = 230,
    /// Multiplication-related binary operators.
    /// `* / % ** *% *|`
    Multiplication = 220,
    /// Addition-related binary operators.
    /// `+ - +% -% +| -|`
    Addition = 210,
    /// Shift-related binary operators.
    /// `<< >> <<|`
    Shift = 200,
    /// Elementwise-related binary operators.
    /// `& ^ |`
    Elementwise = 190,
    /// Comparison-related binary operators.
    /// `== != < > <= >=`
    Comparison = 180,
    /// Logical-related binary operators.
    /// `&& ||`
    Logical = 170,
    /// Assignment-related binary operators.
    /// `=`
    Assignment = 160,
    /// Assignment multiplication-related binary operators.
    /// `*= /= %= **= *%= *|=`
    AssignmentMultiplication = 150,
    /// Assignment addition-related binary operators.
    /// `+= -= +%= -%= +|= -|=`
    AssignmentAddition = 140,
    /// Assignment shift-related binary operators.
    /// `<<= >>= <<|=`
    AssignmentShift = 130,
    /// Assignment elementwise-related binary operators.
    /// `&= ^= |=`
    AssignmentElementwise = 120,
    /// Assignment logical-related binary operators.
    /// `&&= ||=`
    AssignmentLogical = 110,
}

/// A UnaryOperator is unary operator.
/// Relative order matches precedence. Also see OperatorPrecedence.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum UnaryOperator {
    /// `!`
    Not = 237,
    /// `-`
    Negate = 236,
    /// `-%`
    WrappingNegate = 235,
    /// `~`
    ElementwiseNot = 234,
    /// `*`
    Dereference = 233,
    /// `$`
    Virtual = 232,
    /// `..`
    Spread = 231,
}

/// A BinaryOperator is an infix binary operator.
/// Relative order matches precedence. Also see OperatorPrecedence.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BinaryOperator {
    // multiplication
    /// `*`
    Multiply = 224,
    /// `*%`
    WrappingMultiply = 223,
    /// `*|`
    SaturatingMultiply = 222,
    /// `/`
    Divide = 221,
    /// `%`
    Remainder = 220,

    // addition
    /// `+`
    Add = 215,
    /// `+%`
    WrappingAdd = 214,
    /// `+|`
    SaturatingAdd = 213,
    /// `-`
    Subtract = 212,
    /// `-%`
    WrappingSubtract = 211,
    /// `-|`
    SaturatingSubtract = 210,

    // shift
    /// `<<`
    ShiftLeft = 202,
    /// `<<|`
    SaturatingShiftLeft = 201,
    /// `>>`
    ShiftRight = 200,

    // elementwise
    /// `&`
    ElementwiseAnd = 192,
    /// `^`
    ElementwiseXor = 191,
    /// `|`
    ElementwiseOr = 190,

    // comparison
    /// `==`
    Equal = 185,
    /// `!=`
    NotEqual = 184,
    /// `<`
    LessThan = 183,
    /// `<=`
    LessThanOrEqual = 182,
    /// `>`
    GreaterThan = 181,
    /// `>=`
    GreaterThanOrEqual = 180,

    // logical
    /// `&&`
    And = 173,
    /// `||`
    Or = 172,
    /// `??`
    Coalesce = 171,
    /// `as`
    Cast = 170,
}

/// An AssignOperator is assignment type.
/// Relative order matches precedence. Also see OperatorPrecedence.
///
/// Examples:
/// ```
/// x = 1
/// x += 1
/// x >>= 1
/// x &= 1
/// x |= 1
/// ```
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum AssignOperator {
    /// `=`
    Assign = 160,

    // assignment multiplication
    /// `*=`
    MultiplyAssign = 154,
    /// `*%=`
    WrappingMultiplyAssign = 153,
    /// `*|=`
    SaturatingMultiplyAssign = 152,
    /// `/=`
    DivideAssign = 151,
    /// `%=`
    RemainderAssign = 150,

    // assignment addition
    /// `+=`
    AddAssign = 145,
    /// `+%=`
    WrappingAddAssign = 144,
    /// `+|=`
    SaturatingAddAssign = 143,
    /// `-=`
    SubtractAssign = 142,
    /// `-%=`
    WrappingSubtractAssign = 141,
    /// `-|=`
    SaturatingSubtractAssign = 140,

    // assignment shift
    /// `<<=`
    ShiftLeftAssign = 132,
    /// `<<|=`
    SaturatingShiftLeftAssign = 131,
    /// `>>=`
    ShiftRightAssign = 130,

    // assignment elementwise
    /// `&=`
    ElementwiseAndAssign = 122,
    /// `^=`
    ElementwiseXorAssign = 121,
    /// `|=`
    ElementwiseOrAssign = 120,

    // assignment logical
    /// `&&=`
    AndAssign = 111,
    /// `||=`
    OrAssign = 110,
}

/// An InfixOperator is an umbrella for either a binary or assignment operator.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum InfixOperator {
    /// A binary operator.
    Binary(BinaryOperator),
    /// An assignment operator.
    Assign(AssignOperator),
}

impl BinaryOperator {
    /// Get the precedence of the binary operator.
    #[inline]
    pub fn precedence_group(&self) -> OperatorPrecedence {
        match self {
            // multiplication
            BinaryOperator::Multiply => OperatorPrecedence::Multiplication,
            BinaryOperator::WrappingMultiply => OperatorPrecedence::Multiplication,
            BinaryOperator::SaturatingMultiply => OperatorPrecedence::Multiplication,
            BinaryOperator::Divide => OperatorPrecedence::Multiplication,
            BinaryOperator::Remainder => OperatorPrecedence::Multiplication,

            // addition
            BinaryOperator::Add => OperatorPrecedence::Addition,
            BinaryOperator::WrappingAdd => OperatorPrecedence::Addition,
            BinaryOperator::SaturatingAdd => OperatorPrecedence::Addition,
            BinaryOperator::Subtract => OperatorPrecedence::Addition,
            BinaryOperator::WrappingSubtract => OperatorPrecedence::Addition,
            BinaryOperator::SaturatingSubtract => OperatorPrecedence::Addition,

            // shift
            BinaryOperator::ShiftLeft => OperatorPrecedence::Shift,
            BinaryOperator::SaturatingShiftLeft => OperatorPrecedence::Shift,
            BinaryOperator::ShiftRight => OperatorPrecedence::Shift,

            // elementwise
            BinaryOperator::ElementwiseAnd => OperatorPrecedence::Elementwise,
            BinaryOperator::ElementwiseXor => OperatorPrecedence::Elementwise,
            BinaryOperator::ElementwiseOr => OperatorPrecedence::Elementwise,

            // comparison
            BinaryOperator::Equal => OperatorPrecedence::Comparison,
            BinaryOperator::NotEqual => OperatorPrecedence::Comparison,
            BinaryOperator::LessThan => OperatorPrecedence::Comparison,
            BinaryOperator::LessThanOrEqual => OperatorPrecedence::Comparison,
            BinaryOperator::GreaterThan => OperatorPrecedence::Comparison,
            BinaryOperator::GreaterThanOrEqual => OperatorPrecedence::Comparison,

            // logical
            BinaryOperator::And => OperatorPrecedence::Logical,
            BinaryOperator::Or => OperatorPrecedence::Logical,
            BinaryOperator::Coalesce => OperatorPrecedence::Logical,
            BinaryOperator::Cast => OperatorPrecedence::Logical,
        }
    }

    /// Get the precedence of the binary operator.
    pub fn precedence(self) -> u8 {
        // just transmute the enum value to an u8
        self as u8
    }

    /// Convert a TokenType to a BinaryOperator (if a direct mapping exists).
    #[inline]
    pub fn from_token(token_str: &str, token_type: TokenType) -> Option<BinaryOperator> {
        match token_type {
            // multiplication
            TokenType::Multiply => Some(BinaryOperator::Multiply),
            TokenType::WrappingMultiply => Some(BinaryOperator::WrappingMultiply),
            TokenType::SaturatingMultiply => Some(BinaryOperator::SaturatingMultiply),
            TokenType::Divide => Some(BinaryOperator::Divide),
            TokenType::Remainder => Some(BinaryOperator::Remainder),

            // addition
            TokenType::Add => Some(BinaryOperator::Add),
            TokenType::WrappingAdd => Some(BinaryOperator::WrappingAdd),
            TokenType::SaturatingAdd => Some(BinaryOperator::SaturatingAdd),
            TokenType::Subtract => Some(BinaryOperator::Subtract),
            TokenType::WrappingSubtract => Some(BinaryOperator::WrappingSubtract),
            TokenType::SaturatingSubtract => Some(BinaryOperator::SaturatingSubtract),

            // shift
            TokenType::ShiftLeft => Some(BinaryOperator::ShiftLeft),
            TokenType::SaturatingShiftLeft => Some(BinaryOperator::SaturatingShiftLeft),

            // elementwise
            TokenType::ElementwiseAnd => Some(BinaryOperator::ElementwiseAnd),
            TokenType::ElementwiseXor => Some(BinaryOperator::ElementwiseXor),
            TokenType::ElementwiseOr => Some(BinaryOperator::ElementwiseOr),

            // comparison
            TokenType::Equal => Some(BinaryOperator::Equal),
            TokenType::NotEqual => Some(BinaryOperator::NotEqual),
            TokenType::LessThan => Some(BinaryOperator::LessThan),
            TokenType::LessThanOrEqual => Some(BinaryOperator::LessThanOrEqual),
            TokenType::GreaterThan => Some(BinaryOperator::GreaterThan),
            TokenType::GreaterThanOrEqual => Some(BinaryOperator::GreaterThanOrEqual),

            // logical
            TokenType::LogicalAnd => Some(BinaryOperator::And),
            TokenType::LogicalOr => Some(BinaryOperator::Or),
            TokenType::Coalesce => Some(BinaryOperator::Coalesce),
            TokenType::Identifier if token_str == "as" => Some(BinaryOperator::Cast),
            _ => None,
        }
    }
}

impl UnaryOperator {
    /// Get the precedence of the unary operator.
    #[inline]
    pub fn precedence_group(&self) -> OperatorPrecedence {
        OperatorPrecedence::Prefix
    }

    /// Get the precedence of the unary operator.
    #[inline]
    pub fn precedence(self) -> u8 {
        // just transmute the enum value to an u8
        self as u8
    }

    /// Covnert a TokenType to a UnaryOperator (if a direct mapping exists).
    #[inline]
    pub fn from_token_type(token_type: TokenType) -> Option<UnaryOperator> {
        match token_type {
            TokenType::Not => Some(UnaryOperator::Not),
            TokenType::Subtract => Some(UnaryOperator::Negate),
            TokenType::WrappingSubtract => Some(UnaryOperator::WrappingNegate),
            TokenType::Multiply => Some(UnaryOperator::Dereference),
            TokenType::ElementwiseNot => Some(UnaryOperator::ElementwiseNot),
            TokenType::Virtual => Some(UnaryOperator::Virtual),
            TokenType::Range => Some(UnaryOperator::Spread),
            TokenType::RangeWide => Some(UnaryOperator::Spread),
            _ => None,
        }
    }

    /// Convert a UnaryOperator to a TokenType (if a direct mapping exists).
    #[inline]
    pub fn as_token_type(&self) -> TokenType {
        match self {
            UnaryOperator::Not => TokenType::Not,
            UnaryOperator::Negate => TokenType::Subtract,
            UnaryOperator::WrappingNegate => TokenType::WrappingSubtract,
            UnaryOperator::ElementwiseNot => TokenType::ElementwiseNot,
            UnaryOperator::Dereference => TokenType::Multiply,
            UnaryOperator::Virtual => TokenType::Virtual,
            UnaryOperator::Spread => TokenType::Range,
        }
    }
}

impl AssignOperator {
    /// Get the precedence of the assignment type.
    #[inline]
    pub fn precedence_group(&self) -> OperatorPrecedence {
        match self {
            // assignment
            AssignOperator::Assign => OperatorPrecedence::Assignment,

            // assignment multiplication
            AssignOperator::MultiplyAssign
            | AssignOperator::WrappingMultiplyAssign
            | AssignOperator::SaturatingMultiplyAssign
            | AssignOperator::DivideAssign
            | AssignOperator::RemainderAssign => OperatorPrecedence::AssignmentMultiplication,

            // assignment addition
            AssignOperator::AddAssign
            | AssignOperator::WrappingAddAssign
            | AssignOperator::SaturatingAddAssign
            | AssignOperator::SubtractAssign
            | AssignOperator::WrappingSubtractAssign
            | AssignOperator::SaturatingSubtractAssign => OperatorPrecedence::AssignmentAddition,

            // assignment shift
            AssignOperator::ShiftLeftAssign
            | AssignOperator::SaturatingShiftLeftAssign
            | AssignOperator::ShiftRightAssign => OperatorPrecedence::AssignmentShift,

            // assignment elementwise
            AssignOperator::ElementwiseAndAssign
            | AssignOperator::ElementwiseXorAssign
            | AssignOperator::ElementwiseOrAssign => OperatorPrecedence::AssignmentElementwise,

            // assignment logical
            AssignOperator::AndAssign | AssignOperator::OrAssign => {
                OperatorPrecedence::AssignmentLogical
            }
        }
    }

    /// Get the precedence of the assignment type.
    #[inline]
    pub fn precedence(self) -> u8 {
        // just transmute the enum value to an u8
        self as u8
    }

    /// Convert a TokenType to an AssignOperator (if a direct mapping exists).
    #[inline]
    pub fn from_token(token_type: TokenType) -> Option<AssignOperator> {
        match token_type {
            TokenType::Assign => Some(AssignOperator::Assign),

            // addition
            TokenType::AddAssign => Some(AssignOperator::AddAssign),
            TokenType::WrappingAddAssign => Some(AssignOperator::WrappingAddAssign),
            TokenType::SaturatingAddAssign => Some(AssignOperator::SaturatingAddAssign),
            TokenType::SubtractAssign => Some(AssignOperator::SubtractAssign),
            TokenType::WrappingSubtractAssign => Some(AssignOperator::WrappingSubtractAssign),
            TokenType::SaturatingSubtractAssign => Some(AssignOperator::SaturatingSubtractAssign),

            // multiplication
            TokenType::MultiplyAssign => Some(AssignOperator::MultiplyAssign),
            TokenType::WrappingMultiplyAssign => Some(AssignOperator::WrappingMultiplyAssign),
            TokenType::SaturatingMultiplyAssign => Some(AssignOperator::SaturatingMultiplyAssign),
            TokenType::DivideAssign => Some(AssignOperator::DivideAssign),
            TokenType::RemainderAssign => Some(AssignOperator::RemainderAssign),

            // shift
            TokenType::ShiftLeftAssign => Some(AssignOperator::ShiftLeftAssign),
            TokenType::SaturatingShiftLeftAssign => Some(AssignOperator::SaturatingShiftLeftAssign),
            TokenType::ShiftRightAssign => Some(AssignOperator::ShiftRightAssign),

            // elementwise
            TokenType::ElementwiseAndAssign => Some(AssignOperator::ElementwiseAndAssign),
            TokenType::ElementwiseOrAssign => Some(AssignOperator::ElementwiseOrAssign),
            TokenType::ElementwiseXorAssign => Some(AssignOperator::ElementwiseXorAssign),

            // logical
            TokenType::LogicalAndAssign => Some(AssignOperator::AndAssign),
            TokenType::LogicalOrAssign => Some(AssignOperator::OrAssign),

            _ => None,
        }
    }

    /// Convert an AssignOperator to a TokenType (if a direct mapping exists).
    #[inline]
    pub fn as_token_type(&self) -> TokenType {
        match self {
            AssignOperator::Assign => TokenType::Assign,

            // addition
            AssignOperator::AddAssign => TokenType::AddAssign,
            AssignOperator::WrappingAddAssign => TokenType::WrappingAddAssign,
            AssignOperator::SaturatingAddAssign => TokenType::SaturatingAddAssign,
            AssignOperator::SubtractAssign => TokenType::SubtractAssign,
            AssignOperator::WrappingSubtractAssign => TokenType::WrappingSubtractAssign,
            AssignOperator::SaturatingSubtractAssign => TokenType::SaturatingSubtractAssign,

            // multiplication
            AssignOperator::MultiplyAssign => TokenType::MultiplyAssign,
            AssignOperator::WrappingMultiplyAssign => TokenType::WrappingMultiplyAssign,
            AssignOperator::SaturatingMultiplyAssign => TokenType::SaturatingMultiplyAssign,
            AssignOperator::DivideAssign => TokenType::DivideAssign,
            AssignOperator::RemainderAssign => TokenType::RemainderAssign,

            // shift
            AssignOperator::ShiftLeftAssign => TokenType::ShiftLeftAssign,
            AssignOperator::SaturatingShiftLeftAssign => TokenType::SaturatingShiftLeftAssign,
            AssignOperator::ShiftRightAssign => TokenType::ShiftRightAssign,

            // elementwise
            AssignOperator::ElementwiseAndAssign => TokenType::ElementwiseAndAssign,
            AssignOperator::ElementwiseOrAssign => TokenType::ElementwiseOrAssign,
            AssignOperator::ElementwiseXorAssign => TokenType::ElementwiseXorAssign,

            // logical
            AssignOperator::AndAssign => TokenType::LogicalAndAssign,
            AssignOperator::OrAssign => TokenType::LogicalOrAssign,
        }
    }
}

impl InfixOperator {
    /// Get the precedence of the infix operator.
    #[inline]
    pub fn precedence_group(&self) -> OperatorPrecedence {
        match self {
            InfixOperator::Binary(binary_operator) => binary_operator.precedence_group(),
            InfixOperator::Assign(assign_operator) => assign_operator.precedence_group(),
        }
    }

    /// Get the precedence of the infix operator.
    #[inline]
    pub fn precedence(self) -> u8 {
        match self {
            InfixOperator::Binary(binary_operator) => binary_operator.precedence(),
            InfixOperator::Assign(assign_operator) => assign_operator.precedence(),
        }
    }
}

/// An Expression is a generic container for value-producing forms.
///
/// Some Expressions are "place Expressions" and can be read from and written to,
///  that is, they have a place in memory we can point to and get the address of.
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// Definition (with a name or anonymous).
    Definition(NodeId<Definition>),

    /// Block of Statements.
    Block(NodeId<Block>),

    /// A With is a with declaration for context management.
    /// With can declare the use of an item in a scope and refine type bounds.
    ///
    /// Examples:
    /// ```
    /// with T: int32
    /// with Foo
    /// with Foo as Bar
    /// with Foo, Bar
    /// with Foo.Bar
    /// with !Bar
    /// with (
    ///    !Bar,
    ///    Time<F> // optional comma
    ///    F: Numeric
    ///    T > Y
    /// )
    /// ```
    With { clauses: Vec<NodeId<WithClause>> },

    /// A Use is a use declaration for dependency management.
    /// Use can be used as statement for the containing scope or in block form.
    /// `use` includes all or some items from a definition in the relevant scope.
    ///
    /// Examples:
    /// ```
    /// use foo
    /// use foo, bar
    /// use foo.bar
    /// use foo.{bar, baz}
    /// use foo.{} // valid but linted
    /// use foo as baz
    ///
    /// use Heap {
    ///   ...
    /// }
    ///
    /// use Time, !Disk, !Network, !Allocation {
    ///   ...
    /// }
    ///
    /// use someLock() {
    ///
    /// }
    /// ```
    Use {
        visibility: Option<Visibility>,
        clauses: Vec<NodeId<UseClause>>,
        body: Option<NodeId<Block>>,
    },

    /// Let or var binding for constant or mutable variables.
    /// Both let and var may destructure and pattern match.
    ///
    /// Examples:
    /// ```
    /// let x = 1
    /// let x: int32 = 1
    /// let (x, y) = foo()
    /// if let Some(x) = someFunction() {
    ///     ...
    /// }
    /// var x = 1
    /// var x: int32 = 1
    /// var x: int32 // implicitly uninitialized, must be set before use
    /// if var Some(x) = someFunction() {
    ///     ...
    /// }
    ///
    /// let t? = foo() else { return }
    /// let t = foo() ?? return;
    Let {
        mutability: ScopedMutability,
        visibility: Option<Visibility>,
        pattern: NodeId<Pattern>,
        r#type: Option<NodeId<Expression>>,
        value: Option<NodeId<Expression>>,
    },

    /// If/then/else expression.
    /// Then and else must be blocks.
    ///
    /// Examples:
    /// ```
    /// // if
    /// @if x > 0 {
    ///     print("positive")
    /// }
    ///
    /// // if else
    /// if x > 0 {
    ///     print("positive")
    /// } else {
    ///     print("not positive")
    /// }
    ///
    /// // if else if
    /// if x > 0 {
    ///     print("positive")
    /// } else if x == 0 {
    ///     print("zero")
    /// } else {
    ///     print("negative")
    /// }
    /// ```
    If {
        runtime: Option<Runtime>,
        condition: NodeId<Expression>,
        then_block: NodeId<Block>,
        else_block: Option<NodeId<Expression>>,
    },

    /// A While is while loop.
    ///
    /// Examples:
    /// ```
    /// @while x > 1 {
    ///     y = 2
    /// }
    ///
    /// while y < 10 l: {
    ///     y = 2
    ///     break :l
    /// }
    /// ```
    While {
        runtime: Option<Runtime>,
        condition: NodeId<Expression>,
        body: NodeId<Block>,
    },

    /// A For is a for loop over an iterator with a pattern.
    ///
    /// Examples:
    /// ```
    /// @for x in 1..10 {
    ///     y = 2
    /// }
    ///
    /// for x in 1..10 a: {
    ///     if y > 5 {
    ///         continue :a
    ///     }
    ///     y = 2
    /// }
    /// ```
    For {
        runtime: Option<Runtime>,
        pattern: NodeId<Pattern>,
        iterator: NodeId<Expression>,
        body: NodeId<Block>,
    },

    /// A Loop is an unconditional loop.
    ///
    /// Examples:
    /// ```
    /// loop {
    ///     y = getNext()
    ///     if y < 0 {
    ///         break
    ///     }
    /// }
    /// ```
    Loop {
        runtime: Option<Runtime>,
        body: NodeId<Block>,
    },

    /// A Try is try/catch statement.
    /// The try expression may be a single statement or a block of statements.
    /// Any error Result within the try expression aborts the try expression and:
    ///  1. If there is a catch, jumps to the catch pattern matching for handling.
    ///  2. If there is no catch, the error is propagated to the caller explicitly.
    ///
    /// Examples:
    /// ```
    /// try fileOperation() // implicitly unwraps the Result, returns Error case
    ///
    /// try { // implicitly unwraps all Results inside
    ///     let a = riskyOperationA() // a is Result.Ok(_) from riskyOperationA
    ///     riskyOperationB(a)
    /// } // no catch needed if containing function has compatible Result type (Into suffices)
    ///
    /// try { // explicitly unwraps all Results inside
    ///     ...
    /// } catch e { // match all errors
    ///     NumericError(x) => Error(@format("bad number: {x}"))
    ///     FormatError => Error(@format("bad format {e}"))
    ///     // it's exhaustive! otherwise `_ =>` like in match (it is a match)
    /// }
    /// ```
    Try {
        runtime: Option<Runtime>,
        r#try: NodeId<Expression>,
        catch: Option<NodeId<Expression>>,
    },

    /// A Match is match expression with case patterns.
    /// The clauses must be exhaustive and return the same type.
    /// Match statements are Expressions and also used in catch patterns.
    /// Like other statements, match cases do not need to be terminated with a colon/semicolon.
    ///
    /// Examples:
    /// ```
    /// match <expr> {
    ///     (x, y) => {
    ///         ...
    ///     }
    ///     (x, y, z) => {
    ///         ...
    ///     }
    /// }
    /// ```
    Match {
        runtime: Option<Runtime>,
        value: NodeId<Expression>,
        cases: Vec<NodeId<MatchCase>>,
    },

    /// A Break is break statement.
    ///
    /// Examples:
    /// ```
    /// break
    /// break :label
    /// break :label 17
    /// break 15
    /// ```
    Break {
        label: Option<StringId>,
        value: Option<NodeId<Expression>>,
    },

    /// A Continue is continue statement.
    ///
    /// Examples:
    /// ```
    /// continue
    /// continue :label
    /// ```
    Continue { label: Option<StringId> },

    /// Defer expression until scope exit.
    ///
    /// Examples:
    /// ```
    /// defer someFunction()
    ///
    /// defer {
    ///     someFunction()
    ///     someOtherFunction()
    /// }
    ///
    /// defer :label {
    ///     someOtherFunction()
    /// }
    ///
    /// defer catch e {
    ///     _ => someErrorHandler(e)
    /// }
    /// ```/// Defer expression until scope exit..
    Defer {
        expression: Option<NodeId<Expression>>,
        catch: Option<NodeId<Expression>>,
    },

    /// Return expression.
    ///
    /// Examples:
    /// ```
    /// return
    /// return 17
    /// ```
    Return { value: Option<NodeId<Expression>> },

    // NOTE #Incomplete: multiply parameterized Expression Paths?
    //  (like `Foo<int32, boolean>.Bar<Yes: true>`)
    /// Alias reference to some path, statically parameterized.
    Path {
        path: PathId,
        static_arguments: Option<Vec<NodeId<Argument>>>,
    },

    /// Literal scalar value.
    ///
    /// Examples:
    /// ```
    /// true
    /// false
    /// 1
    /// 0x21
    /// 1.0
    /// "Hello, world!"
    /// 'a'
    /// b'a'
    /// b"abc"
    /// 0x1234
    /// ```
    ScalarLiteral(ScalarLiteral),

    /// Type literal.
    ///
    /// Examples:
    /// ```
    /// !
    /// $
    /// _
    /// undefined
    /// void
    /// null
    /// int2
    /// float64
    /// boolean
    /// Self
    /// ```
    TypeLiteral(TypeLiteral),

    /// A RangeLiteral is range of values.
    ///
    /// Examples:
    /// ```
    /// 1..3
    /// 1..n // exclusive
    /// 1..=n // inclusive
    /// ```
    RangeLiteral {
        start: NodeId<Expression>,
        end: NodeId<Expression>,
        is_inclusive: bool,
    },

    /// An ArrayLiteral is literal array of homogeneous elements node.
    ///
    /// Examples:
    /// ```
    /// [] // empty array
    /// [1, 2, ] // trailing comma is allowed
    /// // multi-line array with implicit comma
    /// [
    ///   1 // comma is optional here
    ///   2 // comma is optional here too
    /// ]
    /// [10, false, "Hi"] // hetereogenous array is invalid but legal in AST
    /// ```
    ArrayLiteral { elements: Vec<NodeId<Expression>> },

    /// A TupleLiteral is an anonymous tuple of heterogeneous elements.
    /// For named tuple "literals", see the Call node.
    ///
    /// Examples:
    /// ```
    /// (1, 2, 3)
    /// (1.0, 2.0, 3.0)
    /// (x: int32, y: boolean)
    /// ```
    TupleLiteral { elements: Vec<NodeId<Argument>> },

    /// A StructLiteral is literal struct of heterogeneous fields node.
    /// Struct literals always have an explicit type prefix (unlike tuple literals).
    ///
    /// Examples:
    /// ```
    /// Vector2 { x: 1, y: 2 }
    /// some_module.MyUnion.OptionB { a: true }
    /// ```
    StructLiteral {
        r#type: NodeId<Expression>,
        fields: Vec<NodeId<Argument>>,
    },

    /// Parenthesized expression.
    Parenthesized { expression: NodeId<Expression> },

    /// Unary operation.
    Unary {
        operator: UnaryOperator,
        right: NodeId<Expression>,
    },

    /// Reference operation.
    Reference {
        mutability: ScopedMutability,
        right: NodeId<Expression>,
    },

    /// Member access.
    ///
    /// Examples:
    /// ```
    /// foo.bar
    /// ```
    Member {
        receiver: NodeId<Expression>,
        path: PathId,
    },

    /// Index into a receiver expression.
    ///
    /// Examples:
    /// ```
    /// T[] // special declarative
    /// foo[1]
    /// foo[1..3]
    /// foo["bar"]
    /// foo().result[0][variable+1]
    /// foo.1 // for member access tuple
    Index {
        receiver: NodeId<Expression>,
        index: Option<NodeId<Expression>>,
    },

    /// A Call is call to a function OR an instantiation of a tuple type.
    /// The static arguments are expressed in the receiver, not the call.
    ///
    /// The function may or may not be declared as comptime (with a `@ prefix),
    ///  but the call must be prefixed with a `@` to qualify as a static call.
    ///
    /// Examples:
    /// ```
    /// foo()
    /// @foo(1, 2, 3)
    /// @foo(Vector2 {x: 1, y: 2}, (true, 3))
    /// Bar(1, 2, 3)
    /// MyUnion.Baz(2, 3)
    /// ```
    Call {
        runtime: Option<Runtime>,
        receiver: NodeId<Expression>,
        dynamic_arguments: Vec<NodeId<Argument>>,
    },

    /// Maybe unwrap an expression with `?` and propagate.
    Maybe(NodeId<Expression>),

    /// Force unwrap an expression with `!` and propagate.
    Must(NodeId<Expression>),

    /// Binary operation.
    Binary {
        left: NodeId<Expression>,
        operator: BinaryOperator,
        right: NodeId<Expression>,
    },

    /// Assignment operation.
    Assign {
        left: NodeId<Expression>,
        operator: AssignOperator,
        right: NodeId<Expression>,
    },

    /// Error placeholder.
    Error,
}

impl Expression {}

impl Node for Expression {
    const KIND: NodeType = NodeType::Expression;
}

/// A UseClause is a single clause in a use declaration.
///
/// Examples:
/// ```
/// foo
/// foo as bar
/// foo.bar as baz
/// foo.{baz, qux}
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct UseClause {
    /// The target to use (like `foo.bar` in `use foo.bar.{baz, qux}`)
    pub target: NodeId<Expression>,
    /// The alias to use for the definition (like `bar` in `use foo as bar`)
    pub alias: Option<StringId>,
    /// The items to use from the target (like `{baz, qux}` in `use foo.bar.{baz, qux}`)
    pub items: Option<Vec<NodeId<UseItem>>>,
}

impl Node for UseClause {
    const KIND: NodeType = NodeType::UseClause;
}

/// A UseItem is an item to use in a use clause.
///
/// Examples:
/// ```
/// baz
/// qux as quux
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct UseItem {
    /// The source name of the item (like `foo` in `foo as bar`)
    pub name: StringId,
    /// The alias to use for the item (like `bar` in `foo as bar`)
    pub alias: Option<StringId>,
}

impl Node for UseItem {
    const KIND: NodeType = NodeType::UseItem;
}

/// A WithClause is a single clause in a with declaration.
/// It can be a type assertion (`T: Y`) or a use declaration (`Foo` or `Foo.Bar as Zeb`).
/// Only positive declarations should have aliases (checked later).
///
/// Examples:
/// ```
/// // declaration
/// Foo
/// Foo as Bar
/// Foo.Bar as Baz
///
/// // assertion
/// T: int32
/// Self: geom.Mesh<T>
/// T.Item: Copy
/// T > Y
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum WithClause {
    Declaration {
        /// The item to use (like `Foo.Bar` in `with Foo.Bar`)
        target: NodeId<Expression>,
    },
    Assertion {
        /// The target to assert (like `T` in `with T: int32`)
        target: NodeId<Expression>,
        /// The assertion type (like `int32` in `with T: int32`)
        assertion: NodeId<Expression>,
    },
}

impl Node for WithClause {
    const KIND: NodeType = NodeType::WithClause;
}

/// A MatchCase is a match case inside a Match expression.
/// MatchCases can be any Pattern and can have an optional `if` guard.
///
/// Examples:
/// ```
/// 2 => parse_int(2)
/// (x, y) if x > y => {
///     ...
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum MatchCase {
    /// A match case with an expression body.
    Expression {
        pattern: NodeId<Pattern>,
        body: NodeId<Expression>,
        guard: Option<NodeId<Expression>>,
    },
    /// A match case with a block body.
    Block {
        pattern: NodeId<Pattern>,
        body: NodeId<Block>,
        guard: Option<NodeId<Expression>>,
    },
}

impl Node for MatchCase {
    const KIND: NodeType = NodeType::MatchCase;
}
