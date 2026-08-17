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
        return this.value.read();
    }
}

declare const boxed: Box<Document>;
const text: string = boxed.read<Document>();

=== dir ===
interface Readable {
/// @type.symbol symbol=Readable type=Readable
/// @definition.interface symbol=Readable
/// @definition.method symbol=Readable.read source="read(&readonly this): string" slot=read type=<Readable.read.'a>(this: &Readable.read.'a readonly Readable) => string

    read(&readonly this): string;
    /// @generic.template symbol=Readable.read parent=template#0 parameters=('a)
    /// @type.symbol symbol=Readable.read source="read(&readonly this): string" type=<Readable.read.'a>(this: &Readable.read.'a readonly Readable) => string
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
    /// @type.symbol symbol=Document.read.this source="&readonly this" type=&Document.read.'a readonly this

        return "ok";
    }
}

extension<T> of Box<T> where T: Readable {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @definition.extension symbol=<module>#2 form=local target=Box<T#2>
/// @definition.where symbol=<module>#2 source="T: Readable" relation=satisfies left=T#2 right=Readable
/// @definition.method symbol=read slot=read type=<read.'a>(this: &read.'a readonly this) => string
/// @type.symbol symbol=T source=T type=T#2
/// @resolution.name source=Box target=Box
/// @resolution.name source=T target=T
/// @resolution.name source=T target=T
/// @resolution.name source=Readable target=Readable

    read(&readonly this): string {
    /// @generic.template symbol=read parent=template#2 parameters=('a)
    /// @type.symbol symbol=read type=<read.'a>(this: &read.'a readonly this) => string
    /// @type.symbol symbol=read.this source="&readonly this" type=&read.'a readonly this

        return this.value.read();
        /// @resolution.member source=this.value receiver=&read.'a readonly Box<T#2> type=T#2 kind=field target_receiver=&read.'a readonly Box<T#2> key=value target=Box.value target_type=T#2
        /// @resolution.member source=this.value.read receiver=T#2 type=<Readable.read.'a>(this: &Readable.read.'a readonly T#2) => string kind=symbol target_receiver=T#2 target=Readable.read
        /// @resolution.call source=this.value.read() parameters=() return=string kind=symbol target=Readable.read receiver=T#2 adjustments=(borrow(&read.'a readonly T#2))
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&read.'a readonly Box<T#2>
        /// @resolution.place source=this placement="local" lifetime=read.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement="local" lifetime=read.'a access="readonly"
        /// @resolution.access source=this.value root=this keys=[value]

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
/// @resolution.call source=boxed.read() parameters=() return=string kind=symbol target=read receiver=Box<Document> adjustments=(borrow(&'static readonly Box<Document>)) instance=Box<Document>.<extension#1>.read
/// @resolution.place source=boxed placement="local" lifetime="static" access="readonly"
/// @resolution.access source=boxed root=boxed
/// @generic.instantiation id=read<Document> template=read arguments=(Document)
/// @generic.instance id=read<Document> template=read arguments=(Document)
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
/// @type.symbol symbol=Readable type=Readable
/// @definition.interface symbol=Readable
/// @definition.method symbol=Readable.read source="read(): string" slot=read type=(this: this) => string

    read(): string;
    /// @type.symbol symbol=Readable.read source="read(): string" type=(this: this) => string

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
/// @definition.method symbol=read slot=read type=<read.'a>(this: &read.'a readonly this) => string
/// @type.symbol symbol=T source=T type=T#2
/// @resolution.name source=Box target=Box
/// @resolution.name source=T target=T
/// @resolution.name source=T target=T
/// @resolution.name source=Readable target=Readable

    read(): string {
    /// @generic.template symbol=read parent=template#2 parameters=('a)
    /// @type.symbol symbol=read type=<read.'a>(this: &read.'a readonly this) => string

        return this.value.read();
        /// @type.node source=this type=&read.'a readonly Box<T#2>
        /// @type.node source=this.value type=T#2
        /// @type.node source=this.value.read type=(this: T#2) => string
        /// @type.node source=this.value.read() type=string
        /// @resolution.member source=this.value receiver=&read.'a readonly Box<T#2> type=T#2 kind=field target_receiver=&read.'a readonly Box<T#2> key=value target=Box.value target_type=T#2
        /// @resolution.member source=this.value.read receiver=T#2 type=(this: T#2) => string kind=symbol target_receiver=T#2 target=Readable.read
        /// @resolution.call source=this.value.read() parameters=() return=string kind=symbol target=Readable.read receiver=T#2
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&read.'a readonly Box<T#2>
        /// @resolution.place source=this placement="local" lifetime=read.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement="local" lifetime=read.'a access="readonly"
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
/// @resolution.place source=boxed placement="local" lifetime="static" access="readonly"
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
/// @generic.template symbol=Keyed parameters=(in I)
/// @type.symbol symbol=Keyed type=Keyed
/// @definition.interface symbol=Keyed template=(in I)
/// @definition.where symbol=Keyed relation=satisfies left=this right=Keyed<I>
/// @definition.associated.type symbol=Keyed.Output source="type Output" key=Output
/// @definition.method symbol=Keyed.index source="index(key: I): this.Output" slot=index type=(this: this, I) => this.Output
/// @type.symbol symbol=Keyed.I source=I type=I

    type Output;

    index(key: I): this.Output;
    /// @type.symbol symbol=Keyed.index source="index(key: I): this.Output" type=(this: this, I) => this.Output
    /// @type.symbol symbol=Keyed.index.key source="key: I" type=I
    /// @resolution.name source=I target=Keyed.I

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
/// @generic.template symbol=<module>#2 parameters=(K#2: ops.hash.Hash, V#2)
/// @definition.extension symbol=<module>#2 form=local target=Table<K#2, V#2>
/// @definition.where symbol=<module>#2 source="K: Equal<K>" relation=satisfies left=K#2 right=ops.equality.Equal<K#2>
/// @definition.implements symbol=<module>#2 source=Keyed<K> target=Keyed<K#2>
/// @definition.associated.type symbol=Output source="type Output = V | undefined" key=Output value="V#2 | undefined"
/// @definition.method symbol=index slot=index type=<index.'a>(this: &index.'a readonly this, K#2) => V#2 | undefined
/// @definition.conformance symbol=<module>#2 member=Output requirement=Keyed.Output
/// @definition.conformance symbol=<module>#2 member=index requirement=Keyed.index
/// @type.symbol symbol=K source="K: Hash" type=K#2
/// @resolution.name source=Hash target=ops.hash.Hash
/// @type.symbol symbol=V source=V type=V#2
/// @resolution.name source=Table target=Table
/// @resolution.name source=K target=K
/// @resolution.name source=V target=V
/// @resolution.name source=Keyed target=Keyed
/// @resolution.name source=K target=K
/// @resolution.name source=K target=K
/// @resolution.name source=Equal target=ops.equality.Equal
/// @resolution.name source=K target=K

    type Output = V | undefined;
    /// @type.symbol symbol=Output source="type Output = V | undefined" type=V#2 | undefined
    /// @resolution.name source=V target=V

    index(key: K): V | undefined {
    /// @generic.template symbol=index parent=template#2 parameters=('a)
    /// @type.symbol symbol=index type=<index.'a>(this: &index.'a readonly this, K#2) => V#2 | undefined
    /// @type.symbol symbol=index.key source="key: K" type=K#2
    /// @resolution.name source=K target=K
    /// @resolution.name source=V target=V

        todo("Table.index")
        /// @resolution.name source=todo target=error.panic.todo
        /// @resolution.call source="todo(\"Table.index\")" parameters=(string | undefined) arguments=(provided("Table.index") as string | undefined) return=never kind=symbol target=error.panic.todo

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
        this.duplicate<T>()
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
/// @definition.where symbol=<module>#2 source="T: Copy" relation=satisfies left=T#2 right=memory.capability.Copy
/// @definition.method symbol=duplicate slot=duplicate type=<duplicate.'a>(this: &duplicate.'a readonly this) => T#2
/// @type.symbol symbol=T#1 source=T type=T#2
/// @resolution.name source=Pack target=Pack
/// @resolution.name source=T target=T#1
/// @resolution.name source=T target=T#1
/// @resolution.name source=Copy target=memory.capability.Copy

    duplicate(&readonly this): T {
    /// @generic.template symbol=duplicate parent=template#1 parameters=('a)
    /// @type.symbol symbol=duplicate type=<duplicate.'a>(this: &duplicate.'a readonly this) => T#2
    /// @type.symbol symbol=duplicate.this source="&readonly this" type=&duplicate.'a readonly this
    /// @resolution.name source=T target=T#1

        todo("Pack.duplicate")
        /// @type.node source="todo(\"Pack.duplicate\")" type=never
        /// @type.node source=todo type=(string | undefined?) => never
        /// @resolution.name source=todo target=error.panic.todo
        /// @resolution.call source="todo(\"Pack.duplicate\")" parameters=(string | undefined) arguments=(provided("Pack.duplicate") as string | undefined) return=never kind=symbol target=error.panic.todo
        /// @type.node source="\"Pack.duplicate\"" type="Pack.duplicate"

    }
}

export extension<T> of Pack<T> where T: Copy {
/// @generic.template symbol=<module>#3 parameters=(T#3)
/// @definition.extension symbol=<module>#3 form=exported target=Pack<T#3>
/// @definition.where symbol=<module>#3 source="T: Copy" relation=satisfies left=T#3 right=memory.capability.Copy
/// @definition.method symbol=twice slot=twice type=<twice.'a>(this: &twice.'a readonly this) => T#3
/// @type.symbol symbol=T#2 source=T type=T#3
/// @resolution.name source=Pack target=Pack
/// @resolution.name source=T target=T#2
/// @resolution.name source=T target=T#2
/// @resolution.name source=Copy target=memory.capability.Copy

    twice(&readonly this): T {
    /// @generic.template symbol=twice parent=template#2 parameters=('a)
    /// @type.symbol symbol=twice type=<twice.'a>(this: &twice.'a readonly this) => T#3
    /// @type.symbol symbol=twice.this source="&readonly this" type=&twice.'a readonly this
    /// @resolution.name source=T target=T#2

        this.duplicate()
        /// @type.node source=this type=&twice.'a readonly Pack<T#3>
        /// @type.node source=this.duplicate type=<duplicate.'a>(this: &duplicate.'a readonly &twice.'a readonly Pack<T#3>) => T#3
        /// @type.node source=this.duplicate() type=T#3
        /// @resolution.member source=this.duplicate receiver=&twice.'a readonly Pack<T#3> type=<duplicate.'a>(this: &duplicate.'a readonly &twice.'a readonly Pack<T#3>) => T#3 kind=symbol target_receiver=&twice.'a readonly Pack<T#3> target=duplicate
        /// @resolution.call source=this.duplicate() parameters=() return=T#3 kind=symbol target=duplicate receiver=&twice.'a readonly Pack<T#3> adjustments=(&twice.'a readonly Pack<T#3> => direct -> Pack<T#3>, borrow(&twice.'a readonly Pack<T#3>)) instance=Pack<T#3>.<extension#1>.duplicate
        /// @resolution.receiver source=this kind=this declaration=<module>#3 type=&twice.'a readonly Pack<T#3>
        /// @resolution.place source=this placement="local" lifetime=twice.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @generic.instantiation id=duplicate<T#3> template=duplicate arguments=(T#3) owner=twice

    }
}
"#,
        r#"

"#,
    );
}
