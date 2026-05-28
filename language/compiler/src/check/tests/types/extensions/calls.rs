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
    /// @type.symbol symbol=Readable.read type=(this: Readable) => string
}

struct Box<T> {
/// @generic.slot symbol=Box.T index=0 kind=type
/// @type.symbol symbol=Box type=Box<T>

    value: T;
    /// @type.symbol symbol=Box.value type=T
}

struct Document {
/// @type.symbol symbol=Document type=Document

    read(): string {
    /// @type.symbol symbol=Document.read type=(this: Document) => string

        return "ok";
    }
}

extension BoxReadable<T> of Box<T> where T: Readable {
/// @generic.slot symbol=BoxReadable.T index=0 kind=type constraint=Readable
/// @resolution.name source=Box target=Box
/// @extension.entry symbol=BoxReadable form=inherent target=Box<BoxReadable.T>

    read(): string {
    /// @type.symbol symbol=BoxReadable.read type=(this: Box<BoxReadable.T>) => string

        return this.value.read();
        /// @resolution.name source=this target=this
        /// @resolution.member source=this.value receiver=Box<BoxReadable.T> kind=symbol target=Box.value
        /// @resolution.member source=this.value.read receiver=BoxReadable.T kind=symbol target=Readable.read
        /// @resolution.call source="this.value.read()" parameters=() return=string kind=symbol target=Readable.read receiver=BoxReadable.T
    }
}

declare const boxed: Box<Document>;
/// @type.symbol symbol=boxed type=Box<Document>
/// @generic.application source="Box<Document>" id=Box<Document>

const text = boxed.read();
/// @resolution.name source=boxed target=boxed
/// @resolution.member source=boxed.read receiver=Box<Document> kind=symbol target=BoxReadable.read instance=BoxReadable<Document>
/// @resolution.call source="boxed.read()" parameters=() return=string kind=symbol target=BoxReadable.read receiver=Box<Document> instance=BoxReadable<Document>
/// @type.symbol symbol=text type=string

/// @generic.instance id=Box<Document> symbol=Box arguments=[Document]
/// @generic.instance id=BoxReadable<Document> symbol=BoxReadable arguments=[Document]
"#,
    );
}
