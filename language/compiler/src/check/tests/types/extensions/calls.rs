use crate::tests::{DirRows, TestSession};

#[test]
fn test_extension_method_call_satisfies_generic_receiver_constraint() {
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

extension BoxReadable<T> of Box<T> where T: Readable {
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
interface Readable {
/// @type.symbol symbol=Readable type=Readable
/// @definition.interface symbol=Readable
/// @definition.method symbol=Readable.read source="read(): string" slot=read type=(this: Readable) => string

    read(): string;
    /// @type.symbol symbol=Readable.read source="read(): string" type=(this: Readable) => string

}

struct Box<T> {
/// @generic.template symbol=Box parameters=[T#1]
/// @type.symbol symbol=Box type=Box<T#1>
/// @definition.field symbol=Box.value source="value: T" key=value type=T#1
/// @definition.struct symbol=Box template=LocalGenericTemplateId(0)
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

extension BoxReadable<T> of Box<T> where T: Readable {
/// @generic.template symbol=BoxReadable parameters=[T#2: Readable]
/// @definition.extension symbol=BoxReadable form=inherent target=Box<T#2>
/// @definition.where symbol=BoxReadable source="T: Readable" left=T#2 right=Readable
/// @definition.method symbol=BoxReadable.read slot=read type=(this: Box<T#2>) => string
/// @type.symbol symbol=T#2 source=T type=T#2
/// @resolution.name source=Box target=Box
/// @resolution.name source=T target=T#2
/// @resolution.name source=T target=T#2
/// @resolution.name source=Readable target=Readable

    read(): string {
    /// @type.symbol symbol=BoxReadable.read type=(this: Box<T#2>) => string

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
/// @resolution.member source=boxed.read receiver=Box<Document> kind=symbol target=BoxReadable.read instance=BoxReadable<Document>
/// @resolution.call source=boxed.read() parameters=() return=string kind=symbol target=BoxReadable.read receiver=Box<Document> instance=BoxReadable<Document>

/// @generic.instance id=Box<T#2> symbol=Box arguments=[T#2]
/// @generic.instance id=BoxReadable<Document> symbol=BoxReadable arguments=[Document]
"#,
    );
}

#[test]
fn test_extension_call_requires_satisfied_where_clause() {
    let session = TestSession::single(
        r#"
interface Readable {
    read(): string;
}

struct Box<T> {
    value: T;
}

struct Token {}

extension BoxReadable<T> of Box<T> where T: Readable {
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
interface Readable {
/// @type.symbol symbol=Readable type=Readable
/// @definition.interface symbol=Readable
/// @definition.method symbol=Readable.read source="read(): string" slot=read type=(this: Readable) => string

    read(): string;
    /// @type.symbol symbol=Readable.read source="read(): string" type=(this: Readable) => string

}

struct Box<T> {
/// @generic.template symbol=Box parameters=[T#1]
/// @type.symbol symbol=Box type=Box<T#1>
/// @definition.field symbol=Box.value source="value: T" key=value type=T#1
/// @definition.struct symbol=Box template=LocalGenericTemplateId(0)
/// @type.symbol symbol=T#1 source=T type=T#1

    value: T;
    /// @type.symbol symbol=Box.value source="value: T" type=T#1
    /// @resolution.name source=T target=T#1

}

struct Token {}
/// @type.symbol symbol=Token source="struct Token {}" type=Token
/// @definition.struct symbol=Token source="struct Token {}"

extension BoxReadable<T> of Box<T> where T: Readable {
/// @generic.template symbol=BoxReadable parameters=[T#2: Readable]
/// @definition.extension symbol=BoxReadable form=inherent target=Box<T#2>
/// @definition.where symbol=BoxReadable source="T: Readable" left=T#2 right=Readable
/// @definition.method symbol=BoxReadable.read slot=read type=(this: Box<T#2>) => string
/// @type.symbol symbol=T#2 source=T type=T#2
/// @resolution.name source=Box target=Box
/// @resolution.name source=T target=T#2
/// @resolution.name source=T target=T#2
/// @resolution.name source=Readable target=Readable

    read(): string {
    /// @type.symbol symbol=BoxReadable.read type=(this: Box<T#2>) => string

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

/// @generic.instance id=Box<T#2> symbol=Box arguments=[T#2]
"#,
        r#"
/// @diagnostic.error code=EC300 message="missing member 'read'"
/// @diagnostic.label line=19 column=1 source="boxed.read();"
"#,
    );
}
