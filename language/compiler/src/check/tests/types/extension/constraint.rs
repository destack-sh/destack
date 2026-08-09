use crate::tests::{DirRows, TestSession};

#[test]
fn test_extension_method_satisfies_where_clause() {
    let session = TestSession::single(
        r#"
interface Readable {
    read(): string;
}

struct Box<T> {
    value: T;
}

struct Document {
    read(): string {
        return "ok";
    }
}

extension<T> of Box<T> where T: Readable {
    read(): string {
        return this.value.read();
    }
}

declare const boxed: Box<Document>;
const text = boxed.read();
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Readable {
    read(): string;
}

struct Box<out T> {
    value: T;
}

struct Document {
    read(): string {
        return "ok";
    }
}

extension<T> of Box<T> where T: Readable {
    read(): string {
        return this.value.read();
    }
}

declare const boxed: Box<Document>;
const text: string = boxed.read<Document>();

=== checked ===
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

struct Document {
/// @type.symbol symbol=Document type=Document
/// @definition.struct symbol=Document
/// @definition.method symbol=Document.read slot=read type=<Document.read.'a>(this: &Document.read.'a exclusive this) => string

    read(): string {
    /// @generic.template symbol=Document.read parameters=('a)
    /// @type.symbol symbol=Document.read type=<Document.read.'a>(this: &Document.read.'a exclusive this) => string

        return "ok";
    }
}

extension<T> of Box<T> where T: Readable {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @definition.extension symbol=<module>#2 form=local target=Box<T#2>
/// @definition.where symbol=<module>#2 source="T: Readable" relation=satisfies left=T#2 right=Readable
/// @definition.method symbol=read slot=read type=<read.'a>(this: &read.'a exclusive this) => string
/// @type.symbol symbol=T source=T type=T#2
/// @resolution.name source=Box target=Box
/// @resolution.name source=T target=T
/// @resolution.name source=T target=T
/// @resolution.name source=Readable target=Readable

    read(): string {
    /// @generic.template symbol=read parent=template#2 parameters=('a)
    /// @type.symbol symbol=read type=<read.'a>(this: &read.'a exclusive this) => string

        return this.value.read();
        /// @resolution.member source=this.value receiver=&read.'a exclusive Box<T#2> type=T#2 kind=field target_receiver=&read.'a exclusive Box<T#2> key=value target=Box.value target_type=T#2
        /// @resolution.member source=this.value.read receiver=T#2 type=(this: T#2) => string kind=symbol target_receiver=T#2 target=Readable.read
        /// @resolution.call source=this.value.read() parameters=() return=string kind=symbol target=Readable.read receiver=T#2
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&read.'a exclusive Box<T#2>
        /// @resolution.place source=this placement="local" lifetime=read.'a access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement="local" lifetime=read.'a access="exclusive"
        /// @resolution.access source=this.value root=this keys=[value]

    }
}

declare const boxed: Box<Document>;
/// @type.symbol symbol=boxed source=boxed type=Box<Document>
/// @resolution.pattern source=boxed kind=binding target=boxed
/// @resolution.name source=Box target=Box
/// @resolution.name source=Document target=Document

const text = boxed.read();
/// @type.symbol symbol=text source=text type=string
/// @resolution.pattern source=text kind=binding target=text
/// @resolution.name source=boxed target=boxed
/// @resolution.member source=boxed.read receiver=Box<Document> type=<read.'a>(this: &read.'a exclusive Box<Document>) => string kind=symbol target_receiver=Box<Document> target=read
/// @resolution.call source=boxed.read() parameters=() return=string kind=symbol target=read receiver=Box<Document> adjustments=(borrow(&'static exclusive Box<Document>)) instance=Box<Document>.<extension#1>.read
/// @resolution.place source=boxed placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=boxed root=boxed
/// @generic.instance source=boxed.read() id=Box<Document>.<extension#1>.read

/// @generic.instance id=Box<Document> template=Box arguments=(Document)
/// @generic.instance id=Box<Document>.<extension#1>.read template=read arguments=(Document)
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

    session.assert_dir_checked_and_diagnostics(
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

=== checked ===
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
/// @definition.method symbol=read slot=read type=<read.'a>(this: &read.'a exclusive this) => string
/// @type.symbol symbol=T source=T type=T#2
/// @resolution.name source=Box target=Box
/// @resolution.name source=T target=T
/// @resolution.name source=T target=T
/// @resolution.name source=Readable target=Readable

    read(): string {
    /// @generic.template symbol=read parent=template#2 parameters=('a)
    /// @type.symbol symbol=read type=<read.'a>(this: &read.'a exclusive this) => string

        return this.value.read();
        /// @type.node source=this type=&read.'a exclusive Box<T#2>
        /// @type.node source=this.value type=T#2
        /// @type.node source=this.value.read type=(this: T#2) => string
        /// @type.node source=this.value.read() type=string
        /// @resolution.member source=this.value receiver=&read.'a exclusive Box<T#2> type=T#2 kind=field target_receiver=&read.'a exclusive Box<T#2> key=value target=Box.value target_type=T#2
        /// @resolution.member source=this.value.read receiver=T#2 type=(this: T#2) => string kind=symbol target_receiver=T#2 target=Readable.read
        /// @resolution.call source=this.value.read() parameters=() return=string kind=symbol target=Readable.read receiver=T#2
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&read.'a exclusive Box<T#2>
        /// @resolution.place source=this placement="local" lifetime=read.'a access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement="local" lifetime=read.'a access="exclusive"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @generic.instance source=this id=Box<T#2>

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
/// @resolution.place source=boxed placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=boxed root=boxed
/// @resolution.rejected source=boxed.read
/// @resolution.rejected source=boxed.read()
/// @generic.instance source=boxed id=Box<Token>

/// @generic.instance id=Box<T#2> template=Box arguments=(T#2)
/// @generic.instance id=Box<Token> template=Box arguments=(Token)
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
    session.assert_dir_checked(
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

=== checked ===
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
/// @definition.method symbol=index slot=index type=<index.'a>(this: &index.'a exclusive this, K#2) => V#2 | undefined
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
    /// @type.symbol symbol=index type=<index.'a>(this: &index.'a exclusive this, K#2) => V#2 | undefined
    /// @type.symbol symbol=index.key source="key: K" type=K#2
    /// @resolution.name source=K target=K
    /// @resolution.name source=V target=V

        todo("Table.index")
        /// @resolution.name source=todo target=error.panic.todo
        /// @resolution.call source="todo(\"Table.index\")" parameters=(string | undefined) arguments=(provided("Table.index") as string | undefined) return=never kind=symbol target=error.panic.todo

    }
}

/// @generic.instance id=Keyed<I> template=Keyed arguments=(I)
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

    session.assert_dir_checked_and_diagnostics(
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

=== checked ===
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
        /// @type.node source=this.duplicate type=<duplicate.'a>(this: &duplicate.'a readonly &twice.'a readonly Pack<T#3>) => T#3 reduced=<duplicate.'a>(this: &duplicate.'a readonly Pack<T#3>) => T#3
        /// @type.node source=this.duplicate() type=T#3
        /// @resolution.member source=this.duplicate receiver=&twice.'a readonly Pack<T#3> type=<duplicate.'a>(this: &duplicate.'a readonly &twice.'a readonly Pack<T#3>) => T#3 kind=symbol target_receiver=&twice.'a readonly Pack<T#3> target=duplicate
        /// @resolution.call source=this.duplicate() parameters=() return=T#3 kind=symbol target=duplicate receiver=&twice.'a readonly Pack<T#3> instance=Pack<T#3>.<extension#1>.duplicate
        /// @resolution.receiver source=this kind=this declaration=<module>#3 type=&twice.'a readonly Pack<T#3>
        /// @resolution.place source=this placement="local" lifetime=twice.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @generic.instance source=this id=Pack<T#3>
        /// @generic.instance source=this.duplicate id=Pack<T#3>
        /// @generic.instance source=this.duplicate() id=Pack<T#3>.<extension#1>.duplicate

    }
}

/// @generic.instance id=Pack<T#3> template=Pack arguments=(T#3)
/// @generic.instance id=Pack<T#3>.<extension#1>.duplicate template=duplicate arguments=(T#3)
"#,
        r#""#,
    );
}
