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
const text: string = boxed.read();

=== checked ===
interface Readable {
/// @type.symbol symbol=Readable type=Readable
/// @definition.interface symbol=Readable
/// @definition.method symbol=Readable.read source="read(): string" slot=read type=(this: Readable) => string

    read(): string;
    /// @type.symbol symbol=Readable.read source="read(): string" type=(this: Readable) => string

}

struct Box<T> {
/// @generic.template symbol=Box parameters=(T#1)
/// @type.symbol symbol=Box type=Box<T#1>
/// @definition.struct symbol=Box template=LocalGenericTemplateId(0)
/// @definition.field symbol=Box.value source="value: T" key=value type=T#1
/// @type.symbol symbol=T#1 source=T type=T#1

    value: T;
    /// @type.symbol symbol=Box.value source="value: T" type=T#1
    /// @resolution.name source=T target=T#1

}

struct Document {
/// @type.symbol symbol=Document type=Document
/// @definition.struct symbol=Document
/// @definition.method symbol=Document.read slot=read type=(this: Document) => string

    read(): string {
    /// @type.symbol symbol=Document.read type=(this: Document) => string

        return "ok";
    }
}

extension<T> of Box<T> where T: Readable {
/// @generic.template parameters=(T#2: Readable)
/// @definition.extension form=inherent target=Box<T#2>
/// @definition.where source="T: Readable" relation=satisfies left=T#2 right=Readable
/// @definition.method symbol=read slot=read type=(this: Box<T#2>) => string
/// @type.symbol symbol=T#2 source=T type=T#2
/// @resolution.name source=Box target=Box
/// @resolution.name source=T target=T#2
/// @resolution.name source=T target=T#2
/// @resolution.name source=Readable target=Readable

    read(): string {
    /// @type.symbol symbol=read type=(this: Box<T#2>) => string

        return this.value.read();
        /// @resolution.name source=this target=this#3
        /// @resolution.member source=this.value receiver=Box<T#2> kind=symbol target=Box.value instance=Box<T#2>
        /// @resolution.member source=this.value.read receiver=T#2 kind=symbol target=Readable.read
        /// @resolution.call source=this.value.read() parameters=() return=string kind=symbol target=Readable.read receiver=T#2

    }
}

declare const boxed: Box<Document>;
/// @type.symbol symbol=boxed source=boxed type=Box<Document>
/// @resolution.name source=Box target=Box
/// @resolution.name source=Document target=Document

const text = boxed.read();
/// @type.symbol symbol=text source=text type=string
/// @resolution.name source=boxed target=boxed
/// @resolution.member source=boxed.read receiver=Box<Document> kind=symbol target=read instance=<extension><Document>
/// @resolution.call source=boxed.read() parameters=() return=string kind=symbol target=read receiver=Box<Document> instance=<extension><Document>

/// @generic.instance id=Box<T#2> template=Box arguments=(T#2)
/// @generic.instance id=<extension><Document> arguments=(Document)
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

=== checked ===
interface Readable {
/// @type.symbol symbol=Readable type=Readable
/// @definition.interface symbol=Readable
/// @definition.method symbol=Readable.read source="read(): string" slot=read type=(this: Readable) => string

    read(): string;
    /// @type.symbol symbol=Readable.read source="read(): string" type=(this: Readable) => string

}

struct Box<T> {
/// @generic.template symbol=Box parameters=(T#1)
/// @type.symbol symbol=Box type=Box<T#1>
/// @definition.struct symbol=Box template=LocalGenericTemplateId(0)
/// @definition.field symbol=Box.value source="value: T" key=value type=T#1
/// @type.symbol symbol=T#1 source=T type=T#1

    value: T;
    /// @type.symbol symbol=Box.value source="value: T" type=T#1
    /// @resolution.name source=T target=T#1

}

struct Token {}
/// @type.symbol symbol=Token source="struct Token {}" type=Token
/// @definition.struct symbol=Token source="struct Token {}"

extension<T> of Box<T> where T: Readable {
/// @generic.template parameters=(T#2: Readable)
/// @definition.extension form=inherent target=Box<T#2>
/// @definition.where source="T: Readable" relation=satisfies left=T#2 right=Readable
/// @definition.method symbol=read slot=read type=(this: Box<T#2>) => string
/// @type.symbol symbol=T#2 source=T type=T#2
/// @resolution.name source=Box target=Box
/// @resolution.name source=T target=T#2
/// @resolution.name source=T target=T#2
/// @resolution.name source=Readable target=Readable

    read(): string {
    /// @type.symbol symbol=read type=(this: Box<T#2>) => string

        return this.value.read();
        /// @type.node source=this type=Box<T#2>
        /// @type.node source=this.value type=T#2
        /// @type.node source=this.value.read() type=string
        /// @resolution.name source=this target=this#2
        /// @resolution.member source=this.value receiver=Box<T#2> kind=symbol target=Box.value instance=Box<T#2>
        /// @resolution.member source=this.value.read receiver=T#2 kind=symbol target=Readable.read
        /// @resolution.call source=this.value.read() parameters=() return=string kind=symbol target=Readable.read receiver=T#2

    }
}

declare const boxed: Box<Token>;
/// @type.symbol symbol=boxed source=boxed type=Box<Token>
/// @resolution.name source=Box target=Box
/// @resolution.name source=Token target=Token

boxed.read();
/// @type.node source=boxed type=Box<Token>
/// @type.node source=boxed.read() type=<error>
/// @resolution.name source=boxed target=boxed

/// @generic.instance id=Box<T#2> template=Box arguments=(T#2)
"#,
        r#"
/// @diagnostic.error code=EC300 message="missing member 'read'"
/// @diagnostic.label line=19 column=1 source="boxed.read();"
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

struct Table<K, V> {
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

interface Keyed<I> {
    type Output;

    index(key: I): this.Output;
}

struct Table<K, V> {
    size: usize;
}

extension<K: Hash, V> of Table<K, V> implements Keyed<K> where K: Equal<K> {
    type Output = V | undefined;

    index(key: K): V | undefined {
        todo("Table.index")
    }
}

=== checked ===
import { todo } from "destack:error";
import { Equal, Hash } from "destack:ops";

interface Keyed<I> {
/// @generic.template symbol=Keyed parameters=(I)
/// @type.symbol symbol=Keyed type=Keyed
/// @definition.interface symbol=Keyed template=(I)
/// @definition.where symbol=Keyed relation=satisfies left=this right=Keyed<I>
/// @definition.associated.type symbol=Keyed.Output source="type Output" key=Output
/// @definition.method symbol=Keyed.index source="index(key: I): this.Output" slot=index type=(this: Keyed<I>, I) => this.Output
/// @type.symbol symbol=Keyed.I source=I type=I

    type Output;

    index(key: I): this.Output;
    /// @type.symbol symbol=Keyed.index source="index(key: I): this.Output" type=(this: Keyed<I>, I) => this.Output
    /// @type.symbol symbol=Keyed.index.key source="key: I" type=I
    /// @resolution.name source=I target=Keyed.I

}

struct Table<K, V> {
/// @generic.template symbol=Table parameters=(K#1, V#1)
/// @type.symbol symbol=Table type=Table
/// @definition.struct symbol=Table template=(K#1, V#1)
/// @definition.field symbol=Table.size source="size: usize" key=size type=usize
/// @type.symbol symbol=Table.K source=K type=K#1
/// @type.symbol symbol=Table.V source=V type=V#1

    size: usize;
    /// @type.symbol symbol=Table.size source="size: usize" type=usize

}

extension<K: Hash, V> of Table<K, V> implements Keyed<K> where K: Equal<K> {
/// @generic.template symbol=<module>#2 parameters=(K#2: ops.hash.Hash, V#2)
/// @definition.extension symbol=<module>#2 form=local target=Table<K#2, V#2>
/// @definition.where symbol=<module>#2 source="K: Equal<K>" relation=satisfies left=K#2 right=ops.equality.Equal<K#2>
/// @definition.implements symbol=<module>#2 source=Keyed<K> target=Keyed arguments=(K#2)
/// @definition.associated.type symbol=Output source="type Output = V | undefined" key=Output value="V#2 | undefined"
/// @definition.method symbol=index slot=index type=(this: Table<K#2, V#2>, K#2) => V#2 | undefined
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
    /// @type.symbol symbol=index type=(this: Table<K#2, V#2>, K#2) => V#2 | undefined
    /// @type.symbol symbol=index.key source="key: K" type=K#2
    /// @resolution.name source=K target=K
    /// @resolution.name source=V target=V

        todo("Table.index")
        /// @resolution.name source=todo target=error.panic.todo
        /// @resolution.call source="todo(\"Table.index\")" parameters=(string) arguments=(provided("Table.index") as string) return=never kind=symbol target=error.panic.todo

    }
}

/// @generic.instance id="Table<K#2, V#2>" template=Table arguments=(K#2, V#2)
/// @generic.instance id=Keyed<I> template=Keyed arguments=(I)
"#,
    );
}
