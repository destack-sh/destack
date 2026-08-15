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

    session.assert_dir_checked(
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

=== checked ===
import { Add } from "destack:ops";

struct Token {}
/// @type.symbol symbol=Token source="struct Token {}" type=Token
/// @definition.struct symbol=Token source="struct Token {}"

extension of Token implements Add<Token> {
/// @definition.extension symbol=<module>#2 form=local target=Token
/// @definition.implements symbol=<module>#2 source=Add<Token> target=ops.plus.Add<Token>
/// @definition.associated.type symbol=Output source="type Output = string" key=Output value=string
/// @definition.method symbol=add slot=add type=<add.'a>(this: &add.'a exclusive this, Token) => string
/// @definition.conformance symbol=<module>#2 member=Output requirement=ops.plus.Add.Output
/// @definition.conformance symbol=<module>#2 member=add requirement=ops.plus.Add.add
/// @resolution.name source=Token target=Token
/// @resolution.name source=Add target=ops.plus.Add
/// @resolution.name source=Token target=Token

    type Output = string;
    /// @type.symbol symbol=Output source="type Output = string" type=string

    add(other: Token): string {
    /// @generic.template symbol=add parent=template#0 parameters=('a)
    /// @type.symbol symbol=add type=<add.'a>(this: &add.'a exclusive this, Token) => string
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
/// @resolution.place source=token placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=token root=token
/// @resolution.name source=fallback target=fallback
/// @resolution.place source=fallback placement="local" lifetime="static" access="exclusive"
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

    session.assert_dir_checked(
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

=== checked ===
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
/// @definition.implements symbol=<module>#2 source=Try target=ops.try.Try
/// @definition.associated.type symbol=Output source="type Output = int32" key=Output value=int32
/// @definition.associated.type symbol=Residual source="type Residual = string" key=Residual value=string
/// @definition.method symbol=branch slot=branch type=<branch.'a>(this: &branch.'a exclusive this) => ops.try.ControlFlow<string, int32>
/// @definition.method symbol=fromOutput slot=fromOutput static=true type=(int32) => Attempt
/// @definition.method symbol=fromResidual slot=fromResidual static=true type=(string) => Attempt
/// @definition.conformance symbol=<module>#2 member=Output requirement=ops.try.Try.Output
/// @definition.conformance symbol=<module>#2 member=Residual requirement=ops.try.Try.Residual
/// @definition.conformance symbol=<module>#2 member=branch requirement=ops.try.Try.branch
/// @definition.conformance symbol=<module>#2 member=fromOutput requirement=ops.try.Try.fromOutput
/// @definition.conformance symbol=<module>#2 member=fromResidual requirement=ops.try.FromResidual.fromResidual
/// @resolution.name source=Attempt target=Attempt
/// @resolution.name source=Try target=ops.try.Try

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
    /// @type.symbol symbol=branch type=<branch.'a>(this: &branch.'a exclusive this) => ops.try.ControlFlow<string, int32>
    /// @resolution.name source=ControlFlow target=ops.try.ControlFlow

        return ControlFlow.continue(this.value);
        /// @resolution.name source=ControlFlow target=ops.try.ControlFlow
        /// @resolution.member source=ControlFlow.continue receiver=ops.try.ControlFlow type=(ops.try.C) => ops.try.ControlFlow<ops.try.B, ops.try.C> kind=symbol target_receiver=ops.try.ControlFlow target=ops.try.continue
        /// @resolution.call source=ControlFlow.continue(this.value) parameters=(int32) arguments=(provided(this.value) as int32) return=ops.try.ControlFlow<string, int32> kind=symbol target=ops.try.continue instance="ops.try.ControlFlow<string, int32>.<extension#1>.continue"
        /// @generic.instance source=ControlFlow.continue(this.value) id="ops.try.ControlFlow<string, int32>.<extension#1>.continue"
        /// @resolution.member source=this.value receiver=&branch.'a exclusive Attempt type=int32 kind=field target_receiver=&branch.'a exclusive Attempt key=value target=Attempt.value target_type=int32
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&branch.'a exclusive Attempt
        /// @resolution.place source=this placement="local" lifetime=branch.'a access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement="local" lifetime=branch.'a access="exclusive"
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
/// @resolution.place source=attempt placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=attempt root=attempt

/// @generic.instance id="ops.try.ControlFlow<string, int32>" template=ops.try.ControlFlow arguments=(string, int32)
/// @generic.instance id="ops.try.ControlFlow<string, int32>.<extension#1>.continue" template=ops.try.continue arguments=(string, int32)
"#,
    );
}
