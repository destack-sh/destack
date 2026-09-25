use crate::tests::{DirRows, TestSession};

/// A private backing admits constructions and unwraps in its declaring module.
#[test]
fn test_private_backing_admits_its_declaring_module() {
    let session = TestSession::single(
        r#"
newtype Token = private string;

const token = Token("secret");
const text = token as string;
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
newtype Token = string;

const token: Token = Token("secret");
const text: string = token as string;

=== dir ===
newtype Token = private string;
/// @type.symbol symbol=Token source="newtype Token = private string" type=Token
/// @definition.newtype symbol=Token source="newtype Token = private string" backing=string backing_visibility=private constructors=[(string) => Token]

const token = Token("secret");
/// @type.symbol symbol=token source=token type=Token
/// @resolution.pattern source=token kind=binding target=token
/// @resolution.name source=Token target=Token
/// @resolution.construct source="Token(\"secret\")" parameters=(string) arguments=(provided("secret") as string) return=Token kind=newtype target=Token backing=string

const text = token as string;
/// @type.symbol symbol=text source=text type=string
/// @resolution.pattern source=text kind=binding target=text
/// @resolution.name source=token target=token
/// @resolution.place source=token placement="local" lifetime="static" access="immutable"
/// @resolution.access source=token root=token
"#, r#"

"#);
}

/// A private backing rejects constructions from another module.
#[test]
fn test_private_backing_rejects_a_foreign_construction() {
    let session = TestSession::builder()
        .module(
            "token.tspp",
            r#"
export newtype Token = private string;
"#,
        )
        .module(
            "main.tspp",
            r#"
import { Token } from "./token.tspp";

const token = Token("secret");
"#,
        )
        .build();

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
import { Token } from "./token.tspp";

const token: Token = Token("secret");

=== dir ===
import { Token } from "./token.tspp";

const token = Token("secret");
/// @type.symbol symbol=token source=token type=token.Token
/// @resolution.pattern source=token kind=binding target=token
/// @resolution.name source=Token target=token.Token
/// @resolution.construct source="Token(\"secret\")" parameters=(string) arguments=(provided("secret") as string) return=token.Token kind=newtype target=token.Token backing=string
"#, r#"
/// @diagnostic.error id=inaccessible-newtype-backing message="the backing of 'Token' is private"
/// @diagnostic.label line=4 column=15 span="Token(\"secret\")" line_source="const token = Token(\"secret\");"
"#);
}

/// A private backing rejects unwraps from another module.
#[test]
fn test_private_backing_rejects_a_foreign_unwrap() {
    let session = TestSession::builder()
        .module(
            "token.tspp",
            r#"
export newtype Token = private string;
"#,
        )
        .module(
            "main.tspp",
            r#"
import { Token } from "./token.tspp";

declare const token: Token;
const text = token as string;
"#,
        )
        .build();

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Token } from "./token.tspp";

declare const token: Token;
const text: string = token as string;

=== dir ===
import { Token } from "./token.tspp";

declare const token: Token;
/// @type.symbol symbol=token source=token type=token.Token
/// @resolution.pattern source=token kind=binding target=token
/// @resolution.name source=Token target=token.Token

const text = token as string;
/// @type.symbol symbol=text source=text type=string
/// @resolution.pattern source=text kind=binding target=text
/// @resolution.name source=token target=token
/// @resolution.place source=token placement="local" lifetime="static" access="immutable"
/// @resolution.access source=token root=token
"#,
        r#"
/// @diagnostic.error id=inaccessible-newtype-backing message="the backing of 'Token' is private"
/// @diagnostic.label line=5 column=14 span="token" line_source="const text = token as string;"
"#,
    );
}

/// A public backing admits constructions and unwraps from another module.
#[test]
fn test_public_backing_admits_a_foreign_module() {
    let session = TestSession::builder()
        .module(
            "token.tspp",
            r#"
export newtype Token = string;
"#,
        )
        .module(
            "main.tspp",
            r#"
import { Token } from "./token.tspp";

const token = Token("secret");
const text = token as string;
"#,
        )
        .build();

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
import { Token } from "./token.tspp";

const token: Token = Token("secret");
const text: string = token as string;

=== dir ===
import { Token } from "./token.tspp";

const token = Token("secret");
/// @type.symbol symbol=token source=token type=token.Token
/// @resolution.pattern source=token kind=binding target=token
/// @resolution.name source=Token target=token.Token
/// @resolution.construct source="Token(\"secret\")" parameters=(string) arguments=(provided("secret") as string) return=token.Token kind=newtype target=token.Token backing=string

const text = token as string;
/// @type.symbol symbol=text source=text type=string
/// @resolution.pattern source=text kind=binding target=text
/// @resolution.name source=token target=token
/// @resolution.place source=token placement="local" lifetime="static" access="immutable"
/// @resolution.access source=token root=token
"#, r#"

"#);
}
