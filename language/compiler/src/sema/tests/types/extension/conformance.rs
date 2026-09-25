use crate::tests::{DirRows, TestSession};

#[test]
fn test_check_calls_interface_default_through_extension_conformance() {
    let session = TestSession::single(
        r#"
newtype interface Sized {
    length(this): int32;

    isEmpty(this): boolean {
        return this.length() == 0;
    }
}

struct Buffer {
    length: int32;
}

extension of Buffer implements Sized {
    length(this): int32 {
        return this.length;
    }
}

const buffer = Buffer { length: 3 };
const empty = buffer.isEmpty();
empty satisfies boolean;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface Sized {
    length(this): int32;

    isEmpty(this): boolean {
        return this.length() == 0;
    }
}

struct Buffer {
    length: int32;
}

extension of Buffer implements Sized {
    length(this): int32 {
        return this.length;
    }
}

const buffer: Buffer = Buffer { length: 3 };
const empty: boolean = buffer.isEmpty();
empty satisfies boolean;

=== dir ===
newtype interface Sized {
/// @generic.template symbol=Sized parameters=(this: Sized)
/// @type.symbol symbol=Sized type=Sized
/// @definition.interface symbol=Sized template=(this: Sized) nominal=true
/// @definition.where symbol=Sized relation=satisfies left=this right=Sized
/// @definition.method symbol=Sized.isEmpty slot=isEmpty type=(this: this) => boolean
/// @definition.method symbol=Sized.length source="length(this): int32" slot=length type=(this: this) => int32

    length(this): int32;
    /// @type.symbol symbol=Sized.length source="length(this): int32" type=(this: this) => int32
    /// @type.symbol symbol=Sized.length.this source=this type=this

    isEmpty(this): boolean {
    /// @type.symbol symbol=Sized.isEmpty type=(this: this) => boolean
    /// @type.symbol symbol=Sized.isEmpty.this source=this type=this

        return this.length() == 0;
        /// @resolution.name source=this target=Sized.isEmpty.this
        /// @resolution.member source=this.length receiver=this type=(this: this) => int32 kind=symbol target_receiver=this target=Sized.length
        /// @resolution.call source=this.length() parameters=() return=int32 kind=symbol target=Sized.length receiver=this
        /// @resolution.operator source="this.length() == 0" type=boolean operator="==" kind=builtin operands=[this.length() as int32 families=(integer), 0 as int32 families=(integer)]
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @generic.instantiation id=Sized.length<this> template=Sized.length arguments=() owner=Sized
        /// @generic.instance id=Sized.length<this> template=Sized.length arguments=()

    }
}

struct Buffer {
/// @type.symbol symbol=Buffer type=Buffer
/// @definition.struct symbol=Buffer
/// @definition.field symbol=Buffer.length source="length: int32" key=length type=int32

    length: int32;
    /// @type.symbol symbol=Buffer.length source="length: int32" type=int32

}

extension of Buffer implements Sized {
/// @definition.extension symbol=<module>#2 form=local target=Buffer
/// @definition.implements symbol=<module>#2 source=Sized target=Sized
/// @definition.method symbol=length slot=length type=(this: Buffer) => int32
/// @definition.conformance symbol=<module>#2 member=Sized.isEmpty requirement=Sized.isEmpty
/// @definition.conformance symbol=<module>#2 member=length requirement=Sized.length
/// @resolution.name source=Buffer target=Buffer
/// @resolution.name source=Sized target=Sized

    length(this): int32 {
    /// @type.symbol symbol=length type=(this: Buffer) => int32
    /// @type.symbol symbol=length.this source=this type=Buffer

        return this.length;
        /// @resolution.member source=this.length receiver=Buffer type=int32 kind=field target_receiver=Buffer key=length target=Buffer.length target_type=int32
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Buffer
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.length placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this.length root=this keys=[length]

    }
}

const buffer = Buffer { length: 3 };
/// @type.symbol symbol=buffer source=buffer type=Buffer
/// @resolution.pattern source=buffer kind=binding target=buffer
/// @resolution.name source=Buffer target=Buffer

const empty = buffer.isEmpty();
/// @type.symbol symbol=empty source=empty type=boolean
/// @resolution.pattern source=empty kind=binding target=empty
/// @resolution.name source=buffer target=buffer
/// @resolution.member source=buffer.isEmpty receiver=Buffer type=(this: Buffer) => boolean kind=symbol target_receiver=Buffer target=Sized.isEmpty
/// @resolution.call source=buffer.isEmpty() parameters=() return=boolean kind=symbol target=Sized.isEmpty receiver=Buffer
/// @resolution.place source=buffer placement="local" lifetime="static" access="immutable"
/// @resolution.access source=buffer root=buffer

empty satisfies boolean;
/// @resolution.name source=empty target=empty
/// @resolution.place source=empty placement="local" lifetime="static" access="immutable"
/// @resolution.access source=empty root=empty
"#,
    );
}

#[test]
fn test_check_prefers_extension_members_over_interface_defaults() {
    let session = TestSession::single(
        r#"
newtype interface Sized {
    length(this): int32;

    isEmpty(this): boolean {
        return this.length() == 0;
    }
}

struct Buffer {
    length: int32;
}

extension of Buffer implements Sized {
    length(this): int32 {
        return this.length;
    }

    isEmpty(this): boolean {
        return false;
    }
}

const buffer = Buffer { length: 0 };
const empty = buffer.isEmpty();
empty satisfies boolean;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface Sized {
    length(this): int32;

    isEmpty(this): boolean {
        return this.length() == 0;
    }
}

struct Buffer {
    length: int32;
}

extension of Buffer implements Sized {
    length(this): int32 {
        return this.length;
    }

    isEmpty(this): boolean {
        return false;
    }
}

const buffer: Buffer = Buffer { length: 0 };
const empty: boolean = buffer.isEmpty();
empty satisfies boolean;

=== dir ===
newtype interface Sized {
/// @generic.template symbol=Sized parameters=(this: Sized)
/// @type.symbol symbol=Sized type=Sized
/// @definition.interface symbol=Sized template=(this: Sized) nominal=true
/// @definition.where symbol=Sized relation=satisfies left=this right=Sized
/// @definition.method symbol=Sized.isEmpty slot=isEmpty type=(this: this) => boolean
/// @definition.method symbol=Sized.length source="length(this): int32" slot=length type=(this: this) => int32

    length(this): int32;
    /// @type.symbol symbol=Sized.length source="length(this): int32" type=(this: this) => int32
    /// @type.symbol symbol=Sized.length.this source=this type=this

    isEmpty(this): boolean {
    /// @type.symbol symbol=Sized.isEmpty type=(this: this) => boolean
    /// @type.symbol symbol=Sized.isEmpty.this source=this type=this

        return this.length() == 0;
        /// @resolution.name source=this target=Sized.isEmpty.this
        /// @resolution.member source=this.length receiver=this type=(this: this) => int32 kind=symbol target_receiver=this target=Sized.length
        /// @resolution.call source=this.length() parameters=() return=int32 kind=symbol target=Sized.length receiver=this
        /// @resolution.operator source="this.length() == 0" type=boolean operator="==" kind=builtin operands=[this.length() as int32 families=(integer), 0 as int32 families=(integer)]
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @generic.instantiation id=Sized.length<this> template=Sized.length arguments=() owner=Sized
        /// @generic.instance id=Sized.length<this> template=Sized.length arguments=()

    }
}

struct Buffer {
/// @type.symbol symbol=Buffer type=Buffer
/// @definition.struct symbol=Buffer
/// @definition.field symbol=Buffer.length source="length: int32" key=length type=int32

    length: int32;
    /// @type.symbol symbol=Buffer.length source="length: int32" type=int32

}

extension of Buffer implements Sized {
/// @definition.extension symbol=<module>#2 form=local target=Buffer
/// @definition.implements symbol=<module>#2 source=Sized target=Sized
/// @definition.method symbol=isEmpty slot=isEmpty type=(this: Buffer) => boolean
/// @definition.method symbol=length slot=length type=(this: Buffer) => int32
/// @definition.conformance symbol=<module>#2 member=isEmpty requirement=Sized.isEmpty
/// @definition.conformance symbol=<module>#2 member=length requirement=Sized.length
/// @resolution.name source=Buffer target=Buffer
/// @resolution.name source=Sized target=Sized

    length(this): int32 {
    /// @type.symbol symbol=length type=(this: Buffer) => int32
    /// @type.symbol symbol=length.this source=this type=Buffer

        return this.length;
        /// @resolution.member source=this.length receiver=Buffer type=int32 kind=field target_receiver=Buffer key=length target=Buffer.length target_type=int32
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Buffer
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.length placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this.length root=this keys=[length]

    }

    isEmpty(this): boolean {
    /// @type.symbol symbol=isEmpty type=(this: Buffer) => boolean
    /// @type.symbol symbol=isEmpty.this source=this type=Buffer

        return false;
    }
}

const buffer = Buffer { length: 0 };
/// @type.symbol symbol=buffer source=buffer type=Buffer
/// @resolution.pattern source=buffer kind=binding target=buffer
/// @resolution.name source=Buffer target=Buffer

const empty = buffer.isEmpty();
/// @type.symbol symbol=empty source=empty type=boolean
/// @resolution.pattern source=empty kind=binding target=empty
/// @resolution.name source=buffer target=buffer
/// @resolution.member source=buffer.isEmpty receiver=Buffer type=(this: Buffer) => boolean kind=symbol target_receiver=Buffer target=isEmpty
/// @resolution.call source=buffer.isEmpty() parameters=() return=boolean kind=symbol target=isEmpty receiver=Buffer
/// @resolution.place source=buffer placement="local" lifetime="static" access="immutable"
/// @resolution.access source=buffer root=buffer

empty satisfies boolean;
/// @resolution.name source=empty target=empty
/// @resolution.place source=empty placement="local" lifetime="static" access="immutable"
/// @resolution.access source=empty root=empty
"#,
    );
}
