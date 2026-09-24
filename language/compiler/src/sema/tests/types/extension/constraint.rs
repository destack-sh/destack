use crate::tests::{DirRows, TestSession};

#[test]
fn test_extension_method_satisfies_where_clause() {
    let session = TestSession::single(
        r#"
interface Readable {
    read(&readonly this): string;
}

struct Box<T> {
    value: T;
}

struct Document {
    read(&readonly this): string {
        return "ok";
    }
}

extension<T> of Box<T> where T: Readable {
    read(&readonly this): string {
        return this.value.read();
    }
}

declare const boxed: Box<Document>;
const text = boxed.read();
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Readable {
    read(&readonly this): string;
}

struct Box<out T> {
    value: T;
}

struct Document {
    read(&readonly this): string {
        return "ok";
    }
}

extension<T> of Box<T> where T: Readable {
    read(&readonly this): string {
        return this.value.read<'a>();
    }
}

declare const boxed: Box<Document>;
const text: string = boxed.read<Document, "static">();

=== dir ===
interface Readable {
/// @generic.template symbol=Readable parameters=(this: Readable)
/// @type.symbol symbol=Readable type=Readable
/// @definition.interface symbol=Readable template=(this: Readable)
/// @definition.where symbol=Readable relation=satisfies left=this right=Readable
/// @definition.method symbol=Readable.read source="read(&readonly this): string" slot=read type=<Readable.read.'a>(this: &Readable.read.'a readonly this) => string

    read(&readonly this): string;
    /// @generic.template symbol=Readable.read parent=template#0 parameters=('a)
    /// @type.symbol symbol=Readable.read source="read(&readonly this): string" type=<Readable.read.'a>(this: &Readable.read.'a readonly this) => string
    /// @type.symbol symbol=Readable.read.this source="&readonly this" type=&Readable.read.'a readonly this

}

struct Box<T> {
/// @generic.template symbol=Box parameters=(out T#1)
/// @type.symbol symbol=Box type=Box
/// @definition.struct symbol=Box template=(out T#1)
/// @definition.field symbol=Box.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Box.T source=T type=T#1

    value: T;
    /// @type.symbol symbol=Box.value source="value: T" type=T#1
    /// @resolution.name source=T target=Box.T

}

struct Document {
/// @type.symbol symbol=Document type=Document
/// @definition.struct symbol=Document
/// @definition.method symbol=Document.read slot=read type=<Document.read.'a>(this: &Document.read.'a readonly Document) => string

    read(&readonly this): string {
    /// @generic.template symbol=Document.read parameters=('a)
    /// @type.symbol symbol=Document.read type=<Document.read.'a>(this: &Document.read.'a readonly Document) => string
    /// @type.symbol symbol=Document.read.this source="&readonly this" type=&Document.read.'a readonly Document

        return "ok";
    }
}

extension<T> of Box<T> where T: Readable {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @generic.instance id=Box<T#2> template=Box arguments=(T#2)
/// @definition.extension symbol=<module>#2 form=local target=Box<T#2>
/// @definition.where symbol=<module>#2 source="T: Readable" relation=satisfies left=T#2 right=Readable
/// @definition.method symbol=read slot=read type=<read.'a>(this: &read.'a readonly Box<T#2>) => string
/// @type.symbol symbol=T source=T type=T#2
/// @resolution.name source=Box target=Box
/// @resolution.name source=T target=T
/// @resolution.name source=T target=T
/// @resolution.name source=Readable target=Readable

    read(&readonly this): string {
    /// @generic.template symbol=read parent=template#2 parameters=('a)
    /// @type.symbol symbol=read type=<read.'a>(this: &read.'a readonly Box<T#2>) => string
    /// @type.symbol symbol=read.this source="&readonly this" type=&read.'a readonly Box<T#2>

        return this.value.read();
        /// @resolution.member source=this.value receiver=&read.'a readonly Box<T#2> type=T#2 kind=field target_receiver=&read.'a readonly Box<T#2> key=value target=Box.value target_type=T#2
        /// @resolution.member source=this.value.read receiver=T#2 type=<Readable.read.'a>(this: &Readable.read.'a readonly T#2) => string kind=symbol target_receiver=T#2 target=Readable.read
        /// @resolution.call source=this.value.read() parameters=() return=string regions=(read.'a) kind=symbol target=Readable.read receiver=T#2 adjustments=(borrow(&read.'a readonly T#2)) instance=Readable.read<read.'a>
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&read.'a readonly Box<T#2>
        /// @resolution.place source=this placement=read.'a lifetime=read.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement=read.'a lifetime=read.'a access="readonly"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @generic.instantiation id="Readable.read<T#2, read.'a>" template=Readable.read arguments=(read.'a) owner=read
        /// @generic.instance id="Readable.read<T#2, read.'a>" template=Readable.read arguments=(read.'a)

    }
}

declare const boxed: Box<Document>;
/// @type.symbol symbol=boxed source=boxed type=Box<Document>
/// @resolution.pattern source=boxed kind=binding target=boxed
/// @generic.instance id=Box<Document> template=Box arguments=(Document)
/// @resolution.name source=Box target=Box
/// @resolution.name source=Document target=Document

const text = boxed.read();
/// @type.symbol symbol=text source=text type=string
/// @resolution.pattern source=text kind=binding target=text
/// @resolution.name source=boxed target=boxed
/// @resolution.member source=boxed.read receiver=Box<Document> type=<read.'a>(this: &read.'a readonly Box<Document>) => string kind=symbol target_receiver=Box<Document> target=read
/// @resolution.call source=boxed.read() parameters=() return=string regions=("static" & "local") kind=symbol target=read receiver=Box<Document> adjustments=(borrow(&'static readonly Box<Document>)) instance="Box<Document>.<extension#1>.read<\"static\" & \"local\">"
/// @resolution.place source=boxed placement="local" lifetime="static" access="immutable"
/// @resolution.access source=boxed root=boxed
/// @generic.instantiation id="read<Document, \"static\" & \"local\">" template=read arguments=(Document, "static" & "local")
/// @generic.instantiation id=read<Document> template=read arguments=(Document)
/// @generic.instance id="read<Document, \"bound0\" & \"local\">" template=read arguments=(Document, "bound0" & "local")
"#,
    );
}

#[test]
fn test_extension_method_requires_where_clause() {
    let session = TestSession::single(
        r#"
interface Readable {
    read(): string;
}

struct Box<T> {
    value: T;
}

struct Token {}

extension<T> of Box<T> where T: Readable {
    read(): string {
        return this.value.read();
    }
}

declare const boxed: Box<Token>;
boxed.read();
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
interface Readable {
    read(): string;
}

struct Box<out T> {
    value: T;
}

struct Token {}

extension<T> of Box<T> where T: Readable {
    read(): string {
        return this.value.read();
    }
}

declare const boxed: Box<Token>;
boxed.read();

=== dir ===
interface Readable {
/// @generic.template symbol=Readable parameters=(this: Readable)
/// @type.symbol symbol=Readable type=Readable
/// @definition.interface symbol=Readable template=(this: Readable)
/// @definition.where symbol=Readable relation=satisfies left=this right=Readable
/// @definition.method symbol=Readable.read source="read(): string" slot=read type=() => string

    read(): string;
    /// @type.symbol symbol=Readable.read source="read(): string" type=() => string

}

struct Box<T> {
/// @generic.template symbol=Box parameters=(out T#1)
/// @type.symbol symbol=Box type=Box
/// @definition.struct symbol=Box template=(out T#1)
/// @definition.field symbol=Box.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Box.T source=T type=T#1

    value: T;
    /// @type.symbol symbol=Box.value source="value: T" type=T#1
    /// @resolution.name source=T target=Box.T

}

struct Token {}
/// @type.symbol symbol=Token source="struct Token {}" type=Token
/// @definition.struct symbol=Token source="struct Token {}"

extension<T> of Box<T> where T: Readable {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @definition.extension symbol=<module>#2 form=local target=Box<T#2>
/// @definition.where symbol=<module>#2 source="T: Readable" relation=satisfies left=T#2 right=Readable
/// @definition.method symbol=read slot=read type=<read.'a>(this: &read.'a readonly Box<T#2>) => string
/// @type.symbol symbol=T source=T type=T#2
/// @resolution.name source=Box target=Box
/// @resolution.name source=T target=T
/// @resolution.name source=T target=T
/// @resolution.name source=Readable target=Readable

    read(): string {
    /// @generic.template symbol=read parent=template#2 parameters=('a)
    /// @type.symbol symbol=read type=<read.'a>(this: &read.'a readonly Box<T#2>) => string
    /// @type.symbol symbol=read.this type=&read.'a readonly Box<T#2>

        return this.value.read();
        /// @type.node source=this type=&read.'a readonly Box<T#2>
        /// @type.node source=this.value type=T#2
        /// @type.node source=this.value.read type=() => string
        /// @type.node source=this.value.read() type=string
        /// @resolution.member source=this.value receiver=&read.'a readonly Box<T#2> type=T#2 kind=field target_receiver=&read.'a readonly Box<T#2> key=value target=Box.value target_type=T#2
        /// @resolution.member source=this.value.read receiver=T#2 type=() => string kind=symbol target_receiver=T#2 target=Readable.read
        /// @resolution.call source=this.value.read() parameters=() return=string kind=symbol target=Readable.read receiver=T#2
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&read.'a readonly Box<T#2>
        /// @resolution.place source=this placement=read.'a lifetime=read.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement=read.'a lifetime=read.'a access="readonly"
        /// @resolution.access source=this.value root=this keys=[value]

    }
}

declare const boxed: Box<Token>;
/// @type.symbol symbol=boxed source=boxed type=Box<Token>
/// @resolution.pattern source=boxed kind=binding target=boxed
/// @resolution.name source=Box target=Box
/// @resolution.name source=Token target=Token

boxed.read();
/// @type.node source=boxed type=Box<Token>
/// @type.node source=boxed.read type=<error>
/// @type.node source=boxed.read() type=<error>
/// @resolution.name source=boxed target=boxed
/// @resolution.place source=boxed placement="local" lifetime="static" access="immutable"
/// @resolution.access source=boxed root=boxed
/// @resolution.rejected source=boxed.read
/// @resolution.rejected source=boxed.read()
"#,
        r#"
/// @diagnostic.error id=missing-member message="member 'read' does not exist on type 'Box<Token>'"
/// @diagnostic.label line=19 column=7 span="read" line_source="boxed.read();"
"#,
    );
}

#[test]
fn test_conformance_assumes_the_extension_where_clause() {
    let session = TestSession::single(
        r#"
import { todo } from "destack:error";
import { Equal, Hash } from "destack:ops";

interface Keyed<I> {
    type Output;

    index(key: I): this.Output;
}

struct Table<in out K, in out V> {
    size: usize;
}

extension<K: Hash, V> of Table<K, V> implements Keyed<K> where K: Equal<K> {
    type Output = V | undefined;

    index(key: K): V | undefined {
        todo("Table.index")
    }
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { todo } from "destack:error";
import { Equal, Hash } from "destack:ops";

interface Keyed<in I> {
    type Output;

    index(key: I): this.Output;
}

struct Table<in out K, in out V> {
    size: usize;
}

extension<K: Hash, V> of Table<K, V> implements Keyed<K> where K: Equal<K> {
    type Output = V | undefined;

    index(key: K): V | undefined {
        todo("Table.index" as string | undefined)
    }
}

=== dir ===
import { todo } from "destack:error";
import { Equal, Hash } from "destack:ops";

interface Keyed<I> {
/// @generic.template symbol=Keyed parameters=(in I, this: Keyed<I>)
/// @type.symbol symbol=Keyed type=Keyed
/// @definition.interface symbol=Keyed template=(in I, this: Keyed<I>)
/// @definition.where symbol=Keyed relation=satisfies left=this right=Keyed<I>
/// @definition.associated.type symbol=Keyed.Output source="type Output" key=Output
/// @definition.method symbol=Keyed.index source="index(key: I): this.Output" slot=index type=(I) => this.Output
/// @type.symbol symbol=Keyed.I source=I type=I

    type Output;

    index(key: I): this.Output;
    /// @type.symbol symbol=Keyed.index source="index(key: I): this.Output" type=(I) => this.Output
    /// @type.symbol symbol=Keyed.index.key source="key: I" type=I
    /// @resolution.name source=I target=Keyed.I
    /// @resolution.name source=this.Output target=Keyed.Output

}

struct Table<in out K, in out V> {
/// @generic.template symbol=Table parameters=(in out K#1, in out V#1)
/// @type.symbol symbol=Table type=Table
/// @definition.struct symbol=Table template=(in out K#1, in out V#1)
/// @definition.field symbol=Table.size source="size: usize" key=size type=usize
/// @type.symbol symbol=Table.K source="in out K" type=K#1
/// @type.symbol symbol=Table.V source="in out V" type=V#1

    size: usize;
    /// @type.symbol symbol=Table.size source="size: usize" type=usize

}

extension<K: Hash, V> of Table<K, V> implements Keyed<K> where K: Equal<K> {
/// @generic.template symbol=<module>#2 parameters=(K#2: Hash, V#2)
/// @generic.instance id="Table<K#2, V#2>" template=Table arguments=(K#2, V#2)
/// @generic.instance id=Keyed<K#2> template=Keyed arguments=(K#2)
/// @definition.extension symbol=<module>#2 form=local target=Table<K#2, V#2>
/// @definition.where symbol=<module>#2 source="K: Equal<K>" relation=satisfies left=K#2 right=Equal<K#2>
/// @definition.implements symbol=<module>#2 source=Keyed<K> target=Keyed<K#2>
/// @definition.associated.type symbol=Output source="type Output = V | undefined" key=Output value="V#2 | undefined"
/// @definition.method symbol=index slot=index type=<index.'a>(this: &index.'a readonly Table<K#2, V#2>, K#2) => V#2 | undefined
/// @definition.conformance symbol=<module>#2 member=Output requirement=Keyed.Output
/// @definition.conformance symbol=<module>#2 member=index requirement=Keyed.index
/// @type.symbol symbol=K source="K: Hash" type=K#2
/// @resolution.name source=Hash target=Hash
/// @type.symbol symbol=V source=V type=V#2
/// @resolution.name source=Table target=Table
/// @resolution.name source=K target=K
/// @resolution.name source=V target=V
/// @resolution.name source=Keyed target=Keyed
/// @resolution.name source=K target=K
/// @resolution.name source=K target=K
/// @resolution.name source=Equal target=Equal
/// @generic.instance id=Equal<K#2> template=Equal arguments=(K#2)
/// @resolution.name source=K target=K

    type Output = V | undefined;
    /// @type.symbol symbol=Output source="type Output = V | undefined" type=V#2 | undefined
    /// @resolution.name source=V target=V

    index(key: K): V | undefined {
    /// @generic.template symbol=index parent=template#2 parameters=('a)
    /// @type.symbol symbol=index type=<index.'a>(this: &index.'a readonly Table<K#2, V#2>, K#2) => V#2 | undefined
    /// @type.symbol symbol=index.this type=&index.'a readonly Table<K#2, V#2>
    /// @type.symbol symbol=index.key source="key: K" type=K#2
    /// @resolution.name source=K target=K
    /// @resolution.name source=V target=V

        todo("Table.index")
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"Table.index\")" parameters=(string | undefined) arguments=(provided("Table.index") as string | undefined) return=never kind=symbol target=todo

    }
}
"#,
    );
}

#[test]
fn test_where_assumptions_prove_sibling_where_clauses() {
    let session = TestSession::single(
        r#"
import { Copy } from "destack:memory";

struct Pack<T> {
    value: T;
}

export extension<T> of Pack<T> where T: Copy {
    duplicate(&readonly this): T {
        todo("Pack.duplicate")
    }
}

export extension<T> of Pack<T> where T: Copy {
    twice(&readonly this): T {
        this.duplicate()
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { Copy } from "destack:memory";

struct Pack<out T> {
    value: T;
}

export extension<T> of Pack<T> where T: Copy {
    duplicate(&readonly this): T {
        todo("Pack.duplicate" as string | undefined)
    }
}

export extension<T> of Pack<T> where T: Copy {
    twice(&readonly this): T {
        this.duplicate<T, 'a>()
    }
}

=== dir ===
import { Copy } from "destack:memory";

struct Pack<T> {
/// @generic.template symbol=Pack parameters=(out T#1)
/// @type.symbol symbol=Pack type=Pack
/// @definition.struct symbol=Pack template=(out T#1)
/// @definition.field symbol=Pack.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Pack.T source=T type=T#1

    value: T;
    /// @type.symbol symbol=Pack.value source="value: T" type=T#1
    /// @resolution.name source=T target=Pack.T

}

export extension<T> of Pack<T> where T: Copy {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @definition.extension symbol=<module>#2 form=exported target=Pack<T#2>
/// @definition.where symbol=<module>#2 source="T: Copy" relation=satisfies left=T#2 right=Copy
/// @definition.method symbol=duplicate slot=duplicate type=<duplicate.'a>(this: &duplicate.'a readonly Pack<T#2>) => T#2
/// @type.symbol symbol=T#1 source=T type=T#2
/// @resolution.name source=Pack target=Pack
/// @resolution.name source=T target=T#1
/// @resolution.name source=T target=T#1
/// @resolution.name source=Copy target=Copy

    duplicate(&readonly this): T {
    /// @generic.template symbol=duplicate parent=template#1 parameters=('a)
    /// @type.symbol symbol=duplicate type=<duplicate.'a>(this: &duplicate.'a readonly Pack<T#2>) => T#2
    /// @type.symbol symbol=duplicate.this source="&readonly this" type=&duplicate.'a readonly Pack<T#2>
    /// @resolution.name source=T target=T#1

        todo("Pack.duplicate")
        /// @type.node source="todo(\"Pack.duplicate\")" type=never
        /// @type.node source=todo type=(string | undefined?) => never
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"Pack.duplicate\")" parameters=(string | undefined) arguments=(provided("Pack.duplicate") as string | undefined) return=never kind=symbol target=todo
        /// @type.node source="\"Pack.duplicate\"" type="Pack.duplicate"

    }
}

export extension<T> of Pack<T> where T: Copy {
/// @generic.template symbol=<module>#3 parameters=(T#3)
/// @definition.extension symbol=<module>#3 form=exported target=Pack<T#3>
/// @definition.where symbol=<module>#3 source="T: Copy" relation=satisfies left=T#3 right=Copy
/// @definition.method symbol=twice slot=twice type=<twice.'a>(this: &twice.'a readonly Pack<T#3>) => T#3
/// @type.symbol symbol=T#2 source=T type=T#3
/// @resolution.name source=Pack target=Pack
/// @resolution.name source=T target=T#2
/// @resolution.name source=T target=T#2
/// @resolution.name source=Copy target=Copy

    twice(&readonly this): T {
    /// @generic.template symbol=twice parent=template#2 parameters=('a)
    /// @type.symbol symbol=twice type=<twice.'a>(this: &twice.'a readonly Pack<T#3>) => T#3
    /// @type.symbol symbol=twice.this source="&readonly this" type=&twice.'a readonly Pack<T#3>
    /// @resolution.name source=T target=T#2

        this.duplicate()
        /// @type.node source=this type=&twice.'a readonly Pack<T#3>
        /// @type.node source=this.duplicate type=<duplicate.'a>(this: &duplicate.'a readonly Pack<T#3>) => T#3
        /// @type.node source=this.duplicate() type=T#3
        /// @resolution.member source=this.duplicate receiver=&twice.'a readonly Pack<T#3> type=<duplicate.'a>(this: &duplicate.'a readonly Pack<T#3>) => T#3 kind=symbol target_receiver=&twice.'a readonly Pack<T#3> target=duplicate
        /// @resolution.call source=this.duplicate() parameters=() return=T#3 regions=(twice.'a) kind=symbol target=duplicate receiver=&twice.'a readonly Pack<T#3> instance=Pack<T#3>.<extension#1>.duplicate<twice.'a>
        /// @resolution.receiver source=this kind=this declaration=<module>#3 type=&twice.'a readonly Pack<T#3>
        /// @resolution.place source=this placement=twice.'a lifetime=twice.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @generic.instantiation id="duplicate<T#3, twice.'a>" template=duplicate arguments=(T#3, twice.'a) owner=twice
        /// @generic.instantiation id=duplicate<T#3> template=duplicate arguments=(T#3) owner=twice

    }
}
"#,
        r#"

"#,
    );
}

#[test]
fn test_reject_extension_parameters_the_header_cannot_reach() {
    let session = TestSession::single(
        r#"
struct Box<T> {
    value: T;
}

extension<T, U> of Box<T> {
    first(this): T {
        return this.value;
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Box<out T> {
    value: T;
}

extension<T, U> of Box<T> {
    first(this): T {
        return this.value;
    }
}

=== dir ===
struct Box<T> {
/// @generic.template symbol=Box parameters=(out T#1)
/// @type.symbol symbol=Box type=Box
/// @definition.struct symbol=Box template=(out T#1)
/// @definition.field symbol=Box.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Box.T source=T type=T#1

    value: T;
    /// @type.symbol symbol=Box.value source="value: T" type=T#1
    /// @resolution.name source=T target=Box.T

}

extension<T, U> of Box<T> {
/// @generic.template symbol=<module>#2 parameters=(T#2, U)
/// @definition.extension symbol=<module>#2 form=local target=Box<T#2>
/// @definition.method symbol=first slot=first type=(this: Box<T#2>) => T#2
/// @type.symbol symbol=T source=T type=T#2
/// @type.symbol symbol=U source=U type=U
/// @resolution.name source=Box target=Box
/// @resolution.name source=T target=T

    first(this): T {
    /// @type.symbol symbol=first type=(this: Box<T#2>) => T#2
    /// @type.symbol symbol=first.this source=this type=Box<T#2>
    /// @resolution.name source=T target=T

        return this.value;
        /// @resolution.member source=this.value receiver=Box<T#2> type=T#2 kind=field target_receiver=Box<T#2> key=value target=Box.value target_type=T#2
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Box<T#2>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this.value root=this keys=[value]

    }
}
"#,
        r#"
/// @diagnostic.error id=unconstrained-extension-parameter message="extension parameter 'U' is not constrained by the extension target or an implemented interface"
/// @diagnostic.label line=6 column=14 span="U" line_source="extension<T, U> of Box<T> {"
"#,
    );
}

#[test]
fn test_accept_extension_parameters_reached_through_bounds() {
    let session = TestSession::single(
        r#"
newtype interface Give<T> {
    give(this): T;
}

struct Box<T> {
    value: T;
}

extension<T, I: Give<Box<T>>> of I {
    unwrap(this): T {
        return this.give().value;
    }
}

struct Token {}

extension of Token implements Give<Box<boolean>> {
    give(this): Box<boolean> {
        return Box<boolean> { value: true };
    }
}

function open(value: Token): boolean {
    return value.unwrap();
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked(), r#"
=== annotated ===
newtype interface Give<out T> {
    give(this): T;
}

struct Box<out T> {
    value: T;
}

extension<T, I: Give<Box<T>>> of I {
    unwrap(this): T {
        return this.give<Box<T>>().value;
    }
}

struct Token {}

extension of Token implements Give<Box<boolean>> {
    give(this): Box<boolean> {
        return Box<boolean> { value: true };
    }
}

function open(value: Token): boolean {
    return value.unwrap<boolean, Token>();
}

=== dir ===
newtype interface Give<T> {
/// @generic.template symbol=Give parameters=(out T#1, this: Give<T#1>)
/// @type.symbol symbol=Give type=Give
/// @definition.interface symbol=Give template=(out T#1, this: Give<T#1>) nominal=true
/// @definition.where symbol=Give relation=satisfies left=this right=Give<T#1>
/// @definition.method symbol=Give.give source="give(this): T" slot=give type=(this: this) => T#1
/// @type.symbol symbol=Give.T source=T type=T#1

    give(this): T;
    /// @type.symbol symbol=Give.give source="give(this): T" type=(this: this) => T#1
    /// @type.symbol symbol=Give.give.this source=this type=this
    /// @resolution.name source=T target=Give.T

}

struct Box<T> {
/// @generic.template symbol=Box parameters=(out T#2)
/// @type.symbol symbol=Box type=Box
/// @definition.struct symbol=Box template=(out T#2)
/// @definition.field symbol=Box.value source="value: T" key=value type=T#2
/// @type.symbol symbol=Box.T source=T type=T#2

    value: T;
    /// @type.symbol symbol=Box.value source="value: T" type=T#2
    /// @resolution.name source=T target=Box.T

}

extension<T, I: Give<Box<T>>> of I {
/// @generic.template symbol=<module>#2 parameters=(T#3, I: Give<Box<T#3>>)
/// @definition.extension symbol=<module>#2 form=local target=I
/// @definition.method symbol=unwrap slot=unwrap type=(this: I) => T#3
/// @type.symbol symbol=T source=T type=T#3
/// @type.symbol symbol=I source="I: Give<Box<T>>" type=I
/// @resolution.name source=Give target=Give
/// @generic.instance id=Give<Box<T#3>> template=Give arguments=(Box<T#3>)
/// @resolution.name source=Box target=Box
/// @generic.instance id=Box<T#3> template=Box arguments=(T#3)
/// @resolution.name source=T target=T
/// @resolution.name source=I target=I

    unwrap(this): T {
    /// @type.symbol symbol=unwrap type=(this: I) => T#3
    /// @type.symbol symbol=unwrap.this source=this type=I
    /// @resolution.name source=T target=T

        return this.give().value;
        /// @resolution.member source=this.give receiver=I type=(this: I) => Box<T#3> kind=symbol target_receiver=I target=Give.give
        /// @resolution.member source=this.give().value receiver=Box<T#3> type=T#3 kind=field target_receiver=Box<T#3> key=value target=Box.value target_type=T#3
        /// @resolution.call source=this.give() parameters=() return=Box<T#3> kind=symbol target=Give.give receiver=I instance=Give<Box<T#3>>.give
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=I
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.give().value placement="local" lifetime="frame" access="exclusive"
        /// @generic.instantiation id="Give.give<I, Box<T#3>>" template=Give.give arguments=(Box<T#3>) owner=unwrap
        /// @generic.instantiation id=Give.give<Box<T#3>> template=Give.give arguments=(Box<T#3>) owner=unwrap
        /// @generic.instance id="Give.give<I, Box<T#3>>" template=Give.give arguments=(Box<T#3>)

    }
}

struct Token {}
/// @type.symbol symbol=Token source="struct Token {}" type=Token
/// @definition.struct symbol=Token source="struct Token {}"

extension of Token implements Give<Box<boolean>> {
/// @generic.instance id=Box<boolean> template=Box arguments=(boolean)
/// @generic.instance id=Give<Box<boolean>> template=Give arguments=(Box<boolean>)
/// @definition.extension symbol=<module>#3 form=local target=Token
/// @definition.implements symbol=<module>#3 source=Give<Box<boolean>> target=Give<Box<boolean>>
/// @definition.method symbol=give slot=give type=(this: Token) => Box<boolean>
/// @definition.conformance symbol=<module>#3 member=give requirement=Give.give
/// @resolution.name source=Token target=Token
/// @resolution.name source=Give target=Give
/// @resolution.name source=Box target=Box

    give(this): Box<boolean> {
    /// @type.symbol symbol=give type=(this: Token) => Box<boolean>
    /// @type.symbol symbol=give.this source=this type=Token
    /// @resolution.name source=Box target=Box

        return Box<boolean> { value: true };
        /// @resolution.name source=Box target=Box

    }
}

function open(value: Token): boolean {
/// @type.symbol symbol=open type=(Token) => boolean
/// @type.symbol symbol=open.value source="value: Token" type=Token
/// @resolution.name source=Token target=Token

    return value.unwrap();
    /// @resolution.name source=value target=open.value
    /// @resolution.member source=value.unwrap receiver=Token type=(this: Token) => boolean kind=symbol target_receiver=Token target=unwrap
    /// @resolution.call source=value.unwrap() parameters=() return=boolean kind=symbol target=unwrap receiver=Token instance=Token.<extension#1>.unwrap
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=open.value
    /// @generic.instantiation id="unwrap<boolean, Token>" template=unwrap arguments=(boolean, Token)
    /// @generic.instance id="unwrap<boolean, Token>" template=unwrap arguments=(boolean, Token)

}
"#);
}
