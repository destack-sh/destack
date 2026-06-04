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

    read(): string;
    /// @type.symbol symbol=Readable.read source="read(): string" type=() => string

}

struct Box<T> {
/// @generic.template symbol=Box parameters=[T#1]
/// @type.symbol symbol=Box type=Box<T#1>
/// @nominal.field symbol=Box.value source="value: T" key=value type=T#1
/// @nominal.struct symbol=Box template=LocalGenericTemplateId(0)
/// @type.symbol symbol=T#1 source=T type=T#1

    value: T;
    /// @type.symbol symbol=Box.value source="value: T" type=T#1
    /// @resolution.name source=T target=T#1

}

struct Document {
/// @type.symbol symbol=Document type=Document
/// @nominal.struct symbol=Document
/// @nominal.method symbol=Document.read slot=read type=(this: Document) => string

    read(): string {
    /// @type.symbol symbol=Document.read type=(this: Document) => string

        return "ok";
    }
}

extension BoxReadable<T> of Box<T> where T: Readable {
/// @generic.template symbol=BoxReadable parameters=[T#2: Readable]
/// @extension.entry symbol=BoxReadable form=inherent target=Box<T#2>
/// @type.symbol symbol=T#2 source=T type=T#2
/// @resolution.name source=Box target=Box
/// @resolution.name source=T target=T#2
/// @resolution.name source=T target=T#2
/// @resolution.name source=Readable target=Readable

    read(): string {
    /// @type.symbol symbol=BoxReadable.read type=(this: Box<T#2>) => string

        return this.value.read();
        /// @generic.instance source=this.value id=Box<T#2>
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
/// @generic.instance source=boxed.read id=BoxReadable<Document>
/// @generic.instance source=boxed.read() id=BoxReadable<Document>
/// @resolution.name source=boxed target=boxed
/// @resolution.member source=boxed.read receiver=Box<Document> kind=symbol target=BoxReadable.read instance=BoxReadable<Document>
/// @resolution.call source=boxed.read() parameters=() return=string kind=symbol target=BoxReadable.read receiver=Box<Document> instance=BoxReadable<Document>

/// @generic.instance id=Box<T#2> symbol=Box arguments=[T#2]
/// @generic.instance id=BoxReadable<Document> symbol=BoxReadable arguments=[Document]
"#,
    );
}
