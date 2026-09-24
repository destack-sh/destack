use crate::tests::{DirRows, TestSession};

/// Keep unrelated associated types out of nullish coalescing.
#[test]
fn test_coalesce_ignores_unrelated_output_associated_type() {
    let session = TestSession::single(
        r#"
import { Add } from "destack:ops";

struct Token {}

extension of Token implements Add<Token> {
    type Output = string;

    add(other: Token): string {
        return "";
    }
}

declare const token: Token | undefined;
declare const fallback: Token;
const selected = token ?? fallback;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Add } from "destack:ops";

struct Token {}

extension of Token implements Add<Token> {
    type Output = string;

    add(other: Token): string {
        return "";
    }
}

declare const token: Token | undefined;
declare const fallback: Token;
const selected: Token = token ?? fallback;

=== dir ===
import { Add } from "destack:ops";

struct Token {}
/// @type.symbol symbol=Token source="struct Token {}" type=Token
/// @definition.struct symbol=Token source="struct Token {}"

extension of Token implements Add<Token> {
/// @generic.instance id=Add<Token> template=Add arguments=(Token)
/// @definition.extension symbol=<module>#2 form=local target=Token
/// @definition.implements symbol=<module>#2 source=Add<Token> target=Add<Token>
/// @definition.associated.type symbol=Output source="type Output = string" key=Output value=string
/// @definition.method symbol=add slot=add type=<add.'a>(this: &add.'a readonly Token, Token) => string
/// @definition.conformance symbol=<module>#2 member=Output requirement=Add.Output
/// @definition.conformance symbol=<module>#2 member=add requirement=Add.add
/// @resolution.name source=Token target=Token
/// @resolution.name source=Add target=Add
/// @resolution.name source=Token target=Token

    type Output = string;
    /// @type.symbol symbol=Output source="type Output = string" type=string

    add(other: Token): string {
    /// @generic.template symbol=add parent=template#0 parameters=('a)
    /// @type.symbol symbol=add type=<add.'a>(this: &add.'a readonly Token, Token) => string
    /// @type.symbol symbol=add.this type=&add.'a readonly Token
    /// @type.symbol symbol=add.other source="other: Token" type=Token
    /// @resolution.name source=Token target=Token

        return "";
    }
}

declare const token: Token | undefined;
/// @type.symbol symbol=token source=token type=Token | undefined
/// @resolution.pattern source=token kind=binding target=token
/// @resolution.name source=Token target=Token

declare const fallback: Token;
/// @type.symbol symbol=fallback source=fallback type=Token
/// @resolution.pattern source=fallback kind=binding target=fallback
/// @resolution.name source=Token target=Token

const selected = token ?? fallback;
/// @type.symbol symbol=selected source=selected type=Token
/// @resolution.pattern source=selected kind=binding target=selected
/// @resolution.name source=token target=token
/// @resolution.operator source="token ?? fallback" type=Token operator="??" kind=builtin operands=[token as Token | undefined, fallback as Token]
/// @resolution.place source=token placement="local" lifetime="static" access="immutable"
/// @resolution.access source=token root=token
/// @resolution.name source=fallback target=fallback
/// @resolution.place source=fallback placement="local" lifetime="static" access="immutable"
/// @resolution.access source=fallback root=fallback
"#,
    );
}

/// Project the output selected by a genuine `Try` implementation.
#[test]
fn test_coalesce_projects_try_output() {
    let session = TestSession::single(
        r#"
import { ControlFlow, Try } from "destack:ops";

struct Attempt {
    value: int32;
}

extension of Attempt implements Try {
    type Output = int32;
    type Residual = string;

    static fromOutput(output: int32): Attempt {
        return Attempt { value: output };
    }

    static fromResidual(residual: string): Attempt {
        return Attempt { value: 0 };
    }

    branch(): ControlFlow<string, int32> {
        return ControlFlow.continue(this.value);
    }
}

declare const attempt: Attempt;
const selected = attempt ?? 0;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { ControlFlow, Try } from "destack:ops";

struct Attempt {
    value: int32;
}

extension of Attempt implements Try {
    type Output = int32;
    type Residual = string;

    static fromOutput(output: int32): Attempt {
        return Attempt { value: output };
    }

    static fromResidual(residual: string): Attempt {
        return Attempt { value: 0 };
    }

    branch(): ControlFlow<string, int32> {
        return ControlFlow.continue<string, int32>(this.value);
    }
}

declare const attempt: Attempt;
const selected: int32 = attempt ?? 0;

=== dir ===
import { ControlFlow, Try } from "destack:ops";

struct Attempt {
/// @type.symbol symbol=Attempt type=Attempt
/// @definition.struct symbol=Attempt
/// @definition.field symbol=Attempt.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Attempt.value source="value: int32" type=int32

}

extension of Attempt implements Try {
/// @definition.extension symbol=<module>#2 form=local target=Attempt
/// @definition.implements symbol=<module>#2 source=Try target=Try
/// @definition.associated.type symbol=Output source="type Output = int32" key=Output value=int32
/// @definition.associated.type symbol=Residual source="type Residual = string" key=Residual value=string
/// @definition.method symbol=branch slot=branch type=<branch.'a>(this: &branch.'a readonly Attempt) => ControlFlow<string, int32>
/// @definition.method symbol=fromOutput slot=fromOutput static=true type=(int32) => Attempt
/// @definition.method symbol=fromResidual slot=fromResidual static=true type=(string) => Attempt
/// @definition.conformance symbol=<module>#2 member=Output requirement=Try.Output
/// @definition.conformance symbol=<module>#2 member=Residual requirement=Try.Residual
/// @definition.conformance symbol=<module>#2 member=Try.Failure requirement=Try.Failure
/// @definition.conformance symbol=<module>#2 member=branch requirement=Try.branch
/// @definition.conformance symbol=<module>#2 member=fromOutput requirement=Try.fromOutput
/// @definition.conformance symbol=<module>#2 member=fromResidual requirement=FromResidual.fromResidual
/// @resolution.name source=Attempt target=Attempt
/// @resolution.name source=Try target=Try

    type Output = int32;
    /// @type.symbol symbol=Output source="type Output = int32" type=int32

    type Residual = string;
    /// @type.symbol symbol=Residual source="type Residual = string" type=string

    static fromOutput(output: int32): Attempt {
    /// @type.symbol symbol=fromOutput type=(int32) => Attempt
    /// @type.symbol symbol=fromOutput.output source="output: int32" type=int32
    /// @resolution.name source=Attempt target=Attempt

        return Attempt { value: output };
        /// @resolution.name source=Attempt target=Attempt
        /// @resolution.name source=output target=fromOutput.output
        /// @resolution.place source=output placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=output root=fromOutput.output

    }

    static fromResidual(residual: string): Attempt {
    /// @type.symbol symbol=fromResidual type=(string) => Attempt
    /// @type.symbol symbol=fromResidual.residual source="residual: string" type=string
    /// @resolution.name source=Attempt target=Attempt

        return Attempt { value: 0 };
        /// @resolution.name source=Attempt target=Attempt

    }

    branch(): ControlFlow<string, int32> {
    /// @generic.template symbol=branch parent=template#0 parameters=('a)
    /// @type.symbol symbol=branch type=<branch.'a>(this: &branch.'a readonly Attempt) => ControlFlow<string, int32>
    /// @type.symbol symbol=branch.this type=&branch.'a readonly Attempt
    /// @generic.instance id="ControlFlow<string, int32>" template=ControlFlow arguments=(string, int32)
    /// @generic.instance id=Break<string> template=Break arguments=(string)
    /// @generic.instance id=Continue<int32> template=Continue arguments=(int32)
    /// @resolution.name source=ControlFlow target=ControlFlow

        return ControlFlow.continue(this.value);
        /// @resolution.name source=ControlFlow target=ControlFlow
        /// @resolution.member source=ControlFlow.continue receiver=ControlFlow type=(C) => ControlFlow<B, C> kind=symbol target_receiver=ControlFlow target=continue
        /// @resolution.call source=ControlFlow.continue(this.value) parameters=(int32) arguments=(provided(this.value) as int32) return=ControlFlow<string, int32> kind=symbol target=continue instance="ControlFlow<string, int32>.<extension#1>.continue"
        /// @generic.instantiation id="continue<string, int32>" template=continue arguments=(string, int32)
        /// @generic.instance id="continue<string, int32>" template=continue arguments=(string, int32)
        /// @resolution.member source=this.value receiver=&branch.'a readonly Attempt type=int32 kind=field target_receiver=&branch.'a readonly Attempt key=value target=Attempt.value target_type=int32
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&branch.'a readonly Attempt
        /// @resolution.place source=this placement=branch.'a lifetime=branch.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement=branch.'a lifetime=branch.'a access="readonly"
        /// @resolution.access source=this.value root=this keys=[value]

    }
}

declare const attempt: Attempt;
/// @type.symbol symbol=attempt source=attempt type=Attempt
/// @resolution.pattern source=attempt kind=binding target=attempt
/// @resolution.name source=Attempt target=Attempt

const selected = attempt ?? 0;
/// @type.symbol symbol=selected source=selected type=int32
/// @resolution.pattern source=selected kind=binding target=selected
/// @resolution.name source=attempt target=attempt
/// @resolution.operator source="attempt ?? 0" type=int32 operator="??" kind=builtin operands=[attempt as Attempt, 0 as 0 families=(integer)]
/// @resolution.place source=attempt placement="local" lifetime="static" access="immutable"
/// @resolution.access source=attempt root=attempt
"#,
    );
}

/// Propagate a residual through a `Result` representation.
#[test]
fn test_propagate_try_through_result_representations() {
    let session = TestSession::single(
        r#"
import { Result } from "destack:error";

function passthrough(value: Result<int32, string>): Result<int32, string> {
    const total = value?;

    return Result.ok(total);
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked().with_node_types(), r#"
=== annotated ===
import { Result } from "destack:error";

function passthrough(value: Result<int32, string>): Result<int32, string> {
    const total: int32 = value?;

    return Result.ok<int32, string>(total);
}

=== dir ===
import { Result } from "destack:error";

function passthrough(value: Result<int32, string>): Result<int32, string> {
/// @type.symbol symbol=passthrough type=(Result<int32, string>) => Result<int32, string>
/// @generic.instance id="Result<int32, string>" template=Result arguments=(int32, string)
/// @generic.instance id=Err<string> template=Err arguments=(string)
/// @generic.instance id=Ok<int32> template=Ok arguments=(int32)
/// @type.symbol symbol=passthrough.value source="value: Result<int32, string>" type=Result<int32, string>
/// @resolution.name source=Result target=Result
/// @resolution.name source=Result target=Result

    const total = value?;
    /// @type.symbol symbol=passthrough.total source=total type=int32
    /// @resolution.pattern source=total kind=binding target=passthrough.total
    /// @type.node source=value? type=int32
    /// @resolution.name source=value target=passthrough.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=passthrough.value
    /// @resolution.residual source=value? target=callable residual=TryResidual<Result<int32, string>> branch="branch(parameters=(), arguments=(), return=ControlFlow<Result<never, string>, int32>)" from_residual="fromResidual(parameters=(Result<never, string>), arguments=(supplied(0) as Result<never, string>), return=Result<int32, string>)"
    /// @generic.instantiation id="branch<int32, string>" template=branch arguments=(int32, string)
    /// @generic.instantiation id="fromResidual<int32, string, string>" template=fromResidual arguments=(int32, string, string)
    /// @generic.instance id="Break<Result<never, string>>" template=Break arguments=(Result<never, string>)
    /// @generic.instance id="ControlFlow<Result<never, string>, int32>" template=ControlFlow arguments=(Result<never, string>, int32)
    /// @generic.instance id="Result<never, string>" template=Result arguments=(never, string)
    /// @generic.instance id="branch<int32, string>" template=branch arguments=(int32, string)
    /// @generic.instance id="break<Result<never, string>, int32>" template=break arguments=(Result<never, string>, int32)
    /// @generic.instance id="continue<Result<never, string>, int32>" template=continue arguments=(Result<never, string>, int32)
    /// @generic.instance id="err#1<int32, string>" template=err#1 arguments=(int32, string)
    /// @generic.instance id="err#1<never, string>" template=err#1 arguments=(never, string)
    /// @generic.instance id="fromResidual<int32, string, string>" template=fromResidual arguments=(int32, string, string)
    /// @generic.instance id=Continue<int32> template=Continue arguments=(int32)
    /// @generic.instance id=Ok<never> template=Ok arguments=(never)
    /// @generic.instance id=from<string> template=from arguments=(string)

    return Result.ok(total);
    /// @type.node source=Result.ok type=(T#1) => Result<T#1, E#1>
    /// @type.node source=Result.ok(total) type=Result<int32, string>
    /// @resolution.name source=Result target=Result
    /// @resolution.member source=Result.ok receiver=Result type=(T#1) => Result<T#1, E#1> kind=symbol target_receiver=Result target=ok#1
    /// @resolution.call source=Result.ok(total) parameters=(int32) arguments=(provided(total) as int32) return=Result<int32, string> kind=symbol target=ok#1 instance="Result<int32, string>.<extension#1>.ok#1"
    /// @generic.instantiation id="ok#1<int32, string>" template=ok#1 arguments=(int32, string)
    /// @generic.instance id="ok#1<int32, string>" template=ok#1 arguments=(int32, string)
    /// @resolution.name source=total target=passthrough.total
    /// @resolution.place source=total placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=total root=passthrough.total

}
"#);
}

/// A union return type implements the residual interface as a whole through its extension.
#[test]
fn test_try_an_optional_inside_an_optional_returning_function() {
    let session = TestSession::single(
        r#"
function value(maybe: int32 | undefined): int32 | undefined {
    maybe?;

    return 0;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function value(maybe: int32 | undefined): int32 | undefined {
    maybe?;

    return 0 as int32 | undefined;
}

=== dir ===
function value(maybe: int32 | undefined): int32 | undefined {
/// @type.symbol symbol=value type=(int32 | undefined) => int32 | undefined
/// @type.symbol symbol=value.maybe source="maybe: int32 | undefined" type=int32 | undefined

    maybe?;
    /// @resolution.name source=maybe target=value.maybe
    /// @resolution.place source=maybe placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=maybe root=value.maybe
    /// @resolution.residual source=maybe? target=callable residual=TryResidual<int32 | undefined>

    return 0;
}
"#,
        r#"
"#,
    );
}
