use crate::tests::{DirRows, TestSession};

#[test]
fn test_const_type_parameter_preserves_scalar_literal_precision() {
    let session = TestSession::single(
        r#"
declare function id<const T>(value: T): T;

const value = id("ready");
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function id<const T>(value: T): T;

const value: "ready" = id<"ready">("ready");

=== checked ===
declare function id<const T>(value: T): T;
/// @generic.template symbol=id parameters=(const T)
/// @type.symbol symbol=id source="declare function id<const T>(value: T): T" type=<const T>(T) => T
/// @type.symbol symbol=id.T source="const T" type=T
/// @type.symbol symbol=value#1 source="value: T" type=T
/// @resolution.name source=T target=id.T
/// @resolution.name source=T target=id.T

const value = id("ready");
/// @type.symbol symbol=value#2 source=value type="ready"
/// @resolution.name source=id target=id
/// @resolution.call source="id(\"ready\")" parameters=("ready") return="ready" kind=symbol target=id instance="id<\"ready\">"
/// @generic.instance source="id(\"ready\")" id="id<\"ready\">"
/// @generic.instance id="id<\"ready\">" template=id arguments=("ready")
"#,
    );
}

#[test]
fn test_const_type_parameter_preserves_array_literal_precision() {
    let session = TestSession::single(
        r#"
declare function id<const T>(value: T): T;

const values = id([1, 2]);
const first = values[0];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function id<const T>(value: T): T;

const values: readonly [1, 2] = id<readonly [1, 2]>([1, 2]);
const first: 1 = values[0];

=== checked ===
declare function id<const T>(value: T): T;
/// @generic.template symbol=id parameters=(const T)
/// @type.symbol symbol=id source="declare function id<const T>(value: T): T" type=<const T>(T) => T
/// @type.symbol symbol=id.T source="const T" type=T
/// @type.symbol symbol=value source="value: T" type=T
/// @resolution.name source=T target=id.T
/// @resolution.name source=T target=id.T

const values = id([1, 2]);
/// @type.symbol symbol=values source=values type=readonly [1, 2]
/// @resolution.name source=id target=id
/// @resolution.call source="id([1, 2])" parameters=(readonly [1, 2]) return=readonly [1, 2] kind=symbol target=id instance="id<readonly [1, 2]>"
/// @generic.instance source="id([1, 2])" id="id<readonly [1, 2]>"

const first = values[0];
/// @type.symbol symbol=first source=first type=1
/// @resolution.name source=values target=values
/// @resolution.member source=values[0] receiver=readonly [1, 2] kind=element index=0
/// @generic.instance id="id<readonly [1, 2]>" template=id arguments=(readonly [1, 2])
"#,
    );
}

#[test]
fn test_plain_type_parameter_widens_array_literal_precision() {
    let session = TestSession::single(
        r#"
declare function id<T>(value: T): T;

const values = id([1, 2]);
const first = values[0];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function id<T>(value: T): T;

const values: float64[] = id<float64[]>([1, 2]);
const first: float64 = values[0];

=== checked ===
declare function id<T>(value: T): T;
/// @generic.template symbol=id parameters=(T)
/// @type.symbol symbol=id source="declare function id<T>(value: T): T" type=<T>(T) => T
/// @type.symbol symbol=id.T source=T type=T
/// @type.symbol symbol=value source="value: T" type=T
/// @resolution.name source=T target=id.T
/// @resolution.name source=T target=id.T

const values = id([1, 2]);
/// @type.symbol symbol=values source=values type=Array<float64>
/// @resolution.name source=id target=id
/// @resolution.call source="id([1, 2])" parameters=(Array<float64>) return=Array<float64> kind=symbol target=id instance="id<Array<float64>>"
/// @generic.instance source="id([1, 2])" id="id<Array<float64>>"

const first = values[0];
/// @type.symbol symbol=first source=first type=float64
/// @resolution.name source=values target=values
/// @resolution.call source=values[0] parameters=(usize) return=float64 kind=symbol target=collections.array.index#8 receiver=Array<float64>
/// @generic.instance id="id<Array<float64>>" template=id arguments=(Array<float64>)
"#,
    );
}

#[test]
fn test_const_type_parameter_preserves_object_literal_precision() {
    let session = TestSession::single(
        r#"
declare function id<const T>(value: T): T;

const value = id({ kind: "ready", level: 1 });
const kind = value.kind;
const level = value.level;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function id<const T>(value: T): T;

const value: { readonly kind: "ready"; readonly level: 1 } = id<{
    readonly kind: "ready";
    readonly level: 1;
}>({ kind: "ready", level: 1 });
const kind: "ready" = value.kind;
const level: 1 = value.level;

=== checked ===
declare function id<const T>(value: T): T;
/// @generic.template symbol=id parameters=(const T)
/// @type.symbol symbol=id source="declare function id<const T>(value: T): T" type=<const T>(T) => T
/// @type.symbol symbol=id.T source="const T" type=T
/// @type.symbol symbol=value#1 source="value: T" type=T
/// @resolution.name source=T target=id.T
/// @resolution.name source=T target=id.T

const value = id({ kind: "ready", level: 1 });
/// @type.symbol symbol=value#2 source=value type=Managed<{ readonly kind: "ready"; readonly level: 1 }>
/// @resolution.name source=id target=id
/// @resolution.call source="id({ kind: \"ready\", level: 1 })" parameters=(Managed<{ readonly kind: "ready"; readonly level: 1 }>) return=Managed<{ readonly kind: "ready"; readonly level: 1 }> kind=symbol target=id instance="id<Managed<{ readonly kind: \"ready\"; readonly level: 1 }>>"
/// @generic.instance source="id({ kind: \"ready\", level: 1 })" id="id<Managed<{ readonly kind: \"ready\"; readonly level: 1 }>>"

const kind = value.kind;
/// @type.symbol symbol=kind source=kind type="ready"
/// @resolution.name source=value target=value#2
/// @resolution.member source=value.kind receiver=Managed<{ readonly kind: "ready"; readonly level: 1 }> kind=field key=kind

const level = value.level;
/// @type.symbol symbol=level source=level type=1
/// @resolution.name source=value target=value#2
/// @resolution.member source=value.level receiver=Managed<{ readonly kind: "ready"; readonly level: 1 }> kind=field key=level
/// @generic.instance id="id<Managed<{ readonly kind: \"ready\"; readonly level: 1 }>>" template=id arguments=(Managed<{ readonly kind: "ready"; readonly level: 1 }>)
"#,
    );
}

#[test]
fn test_plain_type_parameter_widens_object_literal_precision() {
    let session = TestSession::single(
        r#"
declare function id<T>(value: T): T;

const value = id({ kind: "ready", level: 1 });
const kind = value.kind;
const level = value.level;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function id<T>(value: T): T;

const value: { kind: string; level: float64 } = id<{ kind: string; level: float64 }>({
    kind: "ready",
    level: 1,
});
const kind: string = value.kind;
const level: float64 = value.level;

=== checked ===
declare function id<T>(value: T): T;
/// @generic.template symbol=id parameters=(T)
/// @type.symbol symbol=id source="declare function id<T>(value: T): T" type=<T>(T) => T
/// @type.symbol symbol=id.T source=T type=T
/// @type.symbol symbol=value#1 source="value: T" type=T
/// @resolution.name source=T target=id.T
/// @resolution.name source=T target=id.T

const value = id({ kind: "ready", level: 1 });
/// @type.symbol symbol=value#2 source=value type=Managed<{ kind: string; level: float64 }>
/// @resolution.name source=id target=id
/// @resolution.call source="id({ kind: \"ready\", level: 1 })" parameters=(Managed<{ kind: string; level: float64 }>) return=Managed<{ kind: string; level: float64 }> kind=symbol target=id instance="id<Managed<{ kind: string; level: float64 }>>"
/// @generic.instance source="id({ kind: \"ready\", level: 1 })" id="id<Managed<{ kind: string; level: float64 }>>"

const kind = value.kind;
/// @type.symbol symbol=kind source=kind type=string
/// @resolution.name source=value target=value#2
/// @resolution.member source=value.kind receiver=Managed<{ kind: string; level: float64 }> kind=field key=kind

const level = value.level;
/// @type.symbol symbol=level source=level type=float64
/// @resolution.name source=value target=value#2
/// @resolution.member source=value.level receiver=Managed<{ kind: string; level: float64 }> kind=field key=level
/// @generic.instance id="id<Managed<{ kind: string; level: float64 }>>" template=id arguments=(Managed<{ kind: string; level: float64 }>)
"#,
    );
}

#[test]
fn test_recursive_constraint_member_lookup_reports_missing_member() {
    let session = TestSession::single(
        r#"
function read<T: T | { name: string }>(value: T): string {
    return value.name;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function read<T: T | { name: string }>(value: T): string {
    return value.name;
}

=== checked ===
function read<T: T | { name: string }>(value: T): string {
/// @generic.template symbol=read parameters=(T: T | { name: string })
/// @type.symbol symbol=read type=<T: T | { name: string }>(T) => string
/// @type.symbol symbol=read.T source="T: T | { name: string }" type=T
/// @resolution.name source=T target=read.T
/// @type.symbol symbol=value source="value: T" type=T
/// @resolution.name source=T target=read.T

    return value.name;
    /// @resolution.name source=value target=value

}
"#,
        r#"
/// @diagnostic.error code=EC300 message="member 'name' does not exist on type 'T'"
/// @diagnostic.label line=3 column=12 source="return value.name;"
"#,
    );
}

#[test]
fn test_parameter_satisfies_its_own_declared_bound() {
    let session = TestSession::single(
        r#"
interface Equal<T> {
    equals(other: T): boolean;
}

class Bucket<K: Equal<K>> {
    key: K;

    constructor(key: K) {
        this.key = key;
    }

    pair(): Bucket<K> {
        return new Bucket<K>(this.key);
    }
}
"#,
    );

    session.assert_dir_checked_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
interface Equal<T> {
    equals(other: T): boolean;
}

class Bucket<K: Equal<K>> {
    key: K;

    constructor(key: K): Bucket<K> {
        this.key = key;
    }

    pair(): Bucket<K> {
        return new Bucket<K>(this.key);
    }
}

=== checked ===
interface Equal<T> {
/// @generic.template symbol=Equal parameters=(T)
/// @type.symbol symbol=Equal type=Equal
/// @definition.interface symbol=Equal template=(T)
/// @definition.where symbol=Equal relation=satisfies left=this right=Equal<T>
/// @definition.method symbol=Equal.equals source="equals(other: T): boolean" slot=equals type=(this: Equal<T>, T) => boolean
/// @type.symbol symbol=Equal.T source=T type=T

    equals(other: T): boolean;
    /// @type.symbol symbol=Equal.equals source="equals(other: T): boolean" type=(this: Equal<T>, T) => boolean
    /// @type.symbol symbol=Equal.equals.other source="other: T" type=T
    /// @resolution.name source=T target=Equal.T

}

class Bucket<K: Equal<K>> {
/// @generic.template symbol=Bucket parameters=(K: Equal<K>)
/// @type.symbol symbol=Bucket type=Bucket
/// @definition.class symbol=Bucket template=(K: Equal<K>)
/// @definition.field symbol=Bucket.key source="key: K" key=key type=K
/// @definition.method symbol=Bucket.constructor slot=constructor role=constructor type=(K) => Bucket<K>
/// @definition.method symbol=Bucket.pair slot=pair type=(this: Bucket<K>) => Bucket<K>
/// @type.symbol symbol=Bucket.K source="K: Equal<K>" type=K
/// @resolution.name source=Equal target=Equal
/// @resolution.name source=K target=Bucket.K

    key: K;
    /// @type.symbol symbol=Bucket.key source="key: K" type=K
    /// @resolution.name source=K target=Bucket.K

    constructor(key: K) {
    /// @type.symbol symbol=Bucket.constructor type=(K) => Bucket<K>
    /// @type.symbol symbol=Bucket.constructor.key source="key: K" type=K
    /// @resolution.name source=K target=Bucket.K

        this.key = key;
        /// @resolution.receiver source=this kind=this declaration=Bucket type=Bucket<K>
        /// @resolution.pattern.assign source=this.key kind=place place=field(Bucket.key) type=K
        /// @resolution.name source=key target=Bucket.constructor.key

    }

    pair(): Bucket<K> {
    /// @type.symbol symbol=Bucket.pair type=(this: Bucket<K>) => Bucket<K>
    /// @resolution.name source=Bucket target=Bucket
    /// @resolution.name source=K target=Bucket.K

        return new Bucket<K>(this.key);
        /// @resolution.construct source="new Bucket<K>(this.key)" parameters=(K) arguments=(provided(this.key) as K) return=Bucket<K> kind=class target=Bucket constructor=Bucket.constructor instance=Bucket<K>
        /// @generic.instance source="new Bucket<K>(this.key)" id=Bucket<K>
        /// @resolution.name source=Bucket target=Bucket
        /// @resolution.name source=K target=Bucket.K
        /// @resolution.member source=this.key receiver=Bucket<K> kind=symbol target=Bucket.key
        /// @resolution.receiver source=this kind=this declaration=Bucket type=Bucket<K>

    }
}

/// @generic.instance id=Bucket<K> template=Bucket arguments=(K)
/// @generic.instance id=Equal<T> template=Equal arguments=(T)
"#, "");
}

#[test]
fn test_extension_where_clause_satisfies_call_bound() {
    let session = TestSession::single(
        r#"
interface Equal<T> {
    equals(other: T): boolean;
}

declare function probe<T: Equal<T>>(value: T): boolean;

class Box<K> {
    key: K;

    constructor(key: K) {
        this.key = key;
    }
}

extension<K> of Box<K> where K: Equal<K> {
    check(): boolean {
        return probe(this.key);
    }
}
"#,
    );

    session.assert_dir_checked_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
interface Equal<T> {
    equals(other: T): boolean;
}

declare function probe<T: Equal<T>>(value: T): boolean;

class Box<K> {
    key: K;

    constructor(key: K): Box<K> {
        this.key = key;
    }
}

extension<K> of Box<K> where K: Equal<K> {
    check(): boolean {
        return probe<K>(this.key);
    }
}

=== checked ===
interface Equal<T> {
/// @generic.template symbol=Equal parameters=(T#1)
/// @type.symbol symbol=Equal type=Equal
/// @definition.interface symbol=Equal template=(T#1)
/// @definition.where symbol=Equal relation=satisfies left=this right=Equal<T#1>
/// @definition.method symbol=Equal.equals source="equals(other: T): boolean" slot=equals type=(this: Equal<T#1>, T#1) => boolean
/// @type.symbol symbol=Equal.T source=T type=T#1

    equals(other: T): boolean;
    /// @type.symbol symbol=Equal.equals source="equals(other: T): boolean" type=(this: Equal<T#1>, T#1) => boolean
    /// @type.symbol symbol=Equal.equals.other source="other: T" type=T#1
    /// @resolution.name source=T target=Equal.T

}

declare function probe<T: Equal<T>>(value: T): boolean;
/// @generic.template symbol=probe parameters=(T#2: Equal<T#2>)
/// @type.symbol symbol=probe source="declare function probe<T: Equal<T>>(value: T): boolean" type=<T#2: Equal<T#2>>(T#2) => boolean
/// @type.symbol symbol=probe.T source="T: Equal<T>" type=T#2
/// @resolution.name source=Equal target=Equal
/// @resolution.name source=T target=probe.T
/// @type.symbol symbol=probe.value source="value: T" type=T#2
/// @resolution.name source=T target=probe.T

class Box<K> {
/// @generic.template symbol=Box parameters=(K#1)
/// @type.symbol symbol=Box type=Box
/// @definition.class symbol=Box template=(K#1)
/// @definition.field symbol=Box.key source="key: K" key=key type=K#1
/// @definition.method symbol=Box.constructor slot=constructor role=constructor type=(K#1) => Box<K#1>
/// @type.symbol symbol=Box.K source=K type=K#1

    key: K;
    /// @type.symbol symbol=Box.key source="key: K" type=K#1
    /// @resolution.name source=K target=Box.K

    constructor(key: K) {
    /// @type.symbol symbol=Box.constructor type=(K#1) => Box<K#1>
    /// @type.symbol symbol=Box.constructor.key source="key: K" type=K#1
    /// @resolution.name source=K target=Box.K

        this.key = key;
        /// @resolution.receiver source=this kind=this declaration=Box type=Box<K#1>
        /// @resolution.pattern.assign source=this.key kind=place place=field(Box.key) type=K#1
        /// @resolution.name source=key target=Box.constructor.key

    }
}

extension<K> of Box<K> where K: Equal<K> {
/// @generic.template symbol=<module>#2 parameters=(K#2)
/// @definition.extension symbol=<module>#2 form=local target=Box<K#2>
/// @definition.where symbol=<module>#2 source="K: Equal<K>" relation=satisfies left=K#2 right=Equal<K#2>
/// @definition.method symbol=check slot=check type=(this: Box<K#2>) => boolean
/// @type.symbol symbol=K source=K type=K#2
/// @resolution.name source=Box target=Box
/// @resolution.name source=K target=K
/// @resolution.name source=K target=K
/// @resolution.name source=Equal target=Equal
/// @resolution.name source=K target=K

    check(): boolean {
    /// @type.symbol symbol=check type=(this: Box<K#2>) => boolean

        return probe(this.key);
        /// @resolution.name source=probe target=probe
        /// @resolution.call source=probe(this.key) parameters=(K#2) arguments=(provided(this.key) as K#2) return=boolean kind=symbol target=probe instance=probe<K#2>
        /// @generic.instance source=probe(this.key) id=probe<K#2>
        /// @resolution.member source=this.key receiver=Box<K#2> kind=symbol target=Box.key
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Box<K#2>

    }
}

/// @generic.instance id=Box<K#1> template=Box arguments=(K#1)
/// @generic.instance id=Box<K#2> template=Box arguments=(K#2)
/// @generic.instance id=Equal<T#1> template=Equal arguments=(T#1)
/// @generic.instance id=probe<K#2> template=probe arguments=(K#2)
"#, "");
}

#[test]
fn test_method_where_clause_satisfies_call_bound() {
    let session = TestSession::single(
        r#"
interface Equal<T> {
    equals(other: T): boolean;
}

declare function probe<T: Equal<T>>(value: T): boolean;

class Box<K> {
    key: K;

    constructor(key: K) {
        this.key = key;
    }

    check(): boolean where K: Equal<K> {
        return probe(this.key);
    }
}
"#,
    );

    session.assert_dir_checked_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
interface Equal<T> {
    equals(other: T): boolean;
}

declare function probe<T: Equal<T>>(value: T): boolean;

class Box<K> {
    key: K;

    constructor(key: K): Box<K> {
        this.key = key;
    }

    check(): boolean where K: Equal<K> {
        return probe<K>(this.key);
    }
}

=== checked ===
interface Equal<T> {
/// @generic.template symbol=Equal parameters=(T#1)
/// @type.symbol symbol=Equal type=Equal
/// @definition.interface symbol=Equal template=(T#1)
/// @definition.where symbol=Equal relation=satisfies left=this right=Equal<T#1>
/// @definition.method symbol=Equal.equals source="equals(other: T): boolean" slot=equals type=(this: Equal<T#1>, T#1) => boolean
/// @type.symbol symbol=Equal.T source=T type=T#1

    equals(other: T): boolean;
    /// @type.symbol symbol=Equal.equals source="equals(other: T): boolean" type=(this: Equal<T#1>, T#1) => boolean
    /// @type.symbol symbol=Equal.equals.other source="other: T" type=T#1
    /// @resolution.name source=T target=Equal.T

}

declare function probe<T: Equal<T>>(value: T): boolean;
/// @generic.template symbol=probe parameters=(T#2: Equal<T#2>)
/// @type.symbol symbol=probe source="declare function probe<T: Equal<T>>(value: T): boolean" type=<T#2: Equal<T#2>>(T#2) => boolean
/// @type.symbol symbol=probe.T source="T: Equal<T>" type=T#2
/// @resolution.name source=Equal target=Equal
/// @resolution.name source=T target=probe.T
/// @type.symbol symbol=probe.value source="value: T" type=T#2
/// @resolution.name source=T target=probe.T

class Box<K> {
/// @generic.template symbol=Box parameters=(K)
/// @type.symbol symbol=Box type=Box
/// @definition.class symbol=Box template=(K)
/// @definition.field symbol=Box.key source="key: K" key=key type=K
/// @definition.method symbol=Box.check slot=check type=(this: Box<K>) => boolean
/// @definition.method symbol=Box.constructor slot=constructor role=constructor type=(K) => Box<K>
/// @type.symbol symbol=Box.K source=K type=K

    key: K;
    /// @type.symbol symbol=Box.key source="key: K" type=K
    /// @resolution.name source=K target=Box.K

    constructor(key: K) {
    /// @type.symbol symbol=Box.constructor type=(K) => Box<K>
    /// @type.symbol symbol=Box.constructor.key source="key: K" type=K
    /// @resolution.name source=K target=Box.K

        this.key = key;
        /// @resolution.receiver source=this kind=this declaration=Box type=Box<K>
        /// @resolution.pattern.assign source=this.key kind=place place=field(Box.key) type=K
        /// @resolution.name source=key target=Box.constructor.key

    }

    check(): boolean where K: Equal<K> {
    /// @generic.template symbol=Box.check parent=template#2 parameters=()
    /// @type.symbol symbol=Box.check type=(this: Box<K>) => boolean
    /// @resolution.name source=K target=Box.K
    /// @resolution.name source=Equal target=Equal
    /// @resolution.name source=K target=Box.K

        return probe(this.key);
        /// @resolution.name source=probe target=probe
        /// @resolution.call source=probe(this.key) parameters=(K) arguments=(provided(this.key) as K) return=boolean kind=symbol target=probe instance=probe<K>
        /// @generic.instance source=probe(this.key) id=probe<K>
        /// @resolution.member source=this.key receiver=Box<K> kind=symbol target=Box.key
        /// @resolution.receiver source=this kind=this declaration=Box type=Box<K>

    }
}

/// @generic.instance id=Box<K> template=Box arguments=(K)
/// @generic.instance id=Equal<T#1> template=Equal arguments=(T#1)
/// @generic.instance id=probe<K> template=probe arguments=(K)
"#, "");
}

#[test]
fn test_extension_method_calls_module_function() {
    let session = TestSession::single(
        r#"
declare function probe<T>(value: T): boolean;

class Box<K> {
    key: K;

    constructor(key: K) {
        this.key = key;
    }
}

extension<K> of Box<K> {
    check(): boolean {
        return probe(this.key);
    }
}
"#,
    );

    session.assert_dir_checked_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
declare function probe<T>(value: T): boolean;

class Box<K> {
    key: K;

    constructor(key: K): Box<K> {
        this.key = key;
    }
}

extension<K> of Box<K> {
    check(): boolean {
        return probe<K>(this.key);
    }
}

=== checked ===
declare function probe<T>(value: T): boolean;
/// @generic.template symbol=probe parameters=(T)
/// @type.symbol symbol=probe source="declare function probe<T>(value: T): boolean" type=<T>(T) => boolean
/// @type.symbol symbol=probe.T source=T type=T
/// @type.symbol symbol=probe.value source="value: T" type=T
/// @resolution.name source=T target=probe.T

class Box<K> {
/// @generic.template symbol=Box parameters=(K#1)
/// @type.symbol symbol=Box type=Box
/// @definition.class symbol=Box template=(K#1)
/// @definition.field symbol=Box.key source="key: K" key=key type=K#1
/// @definition.method symbol=Box.constructor slot=constructor role=constructor type=(K#1) => Box<K#1>
/// @type.symbol symbol=Box.K source=K type=K#1

    key: K;
    /// @type.symbol symbol=Box.key source="key: K" type=K#1
    /// @resolution.name source=K target=Box.K

    constructor(key: K) {
    /// @type.symbol symbol=Box.constructor type=(K#1) => Box<K#1>
    /// @type.symbol symbol=Box.constructor.key source="key: K" type=K#1
    /// @resolution.name source=K target=Box.K

        this.key = key;
        /// @resolution.receiver source=this kind=this declaration=Box type=Box<K#1>
        /// @resolution.pattern.assign source=this.key kind=place place=field(Box.key) type=K#1
        /// @resolution.name source=key target=Box.constructor.key

    }
}

extension<K> of Box<K> {
/// @generic.template symbol=<module>#2 parameters=(K#2)
/// @definition.extension symbol=<module>#2 form=local target=Box<K#2>
/// @definition.method symbol=check slot=check type=(this: Box<K#2>) => boolean
/// @type.symbol symbol=K source=K type=K#2
/// @resolution.name source=Box target=Box
/// @resolution.name source=K target=K

    check(): boolean {
    /// @type.symbol symbol=check type=(this: Box<K#2>) => boolean

        return probe(this.key);
        /// @resolution.name source=probe target=probe
        /// @resolution.call source=probe(this.key) parameters=(K#2) arguments=(provided(this.key) as K#2) return=boolean kind=symbol target=probe instance=probe<K#2>
        /// @generic.instance source=probe(this.key) id=probe<K#2>
        /// @resolution.member source=this.key receiver=Box<K#2> kind=symbol target=Box.key
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Box<K#2>

    }
}

/// @generic.instance id=Box<K#1> template=Box arguments=(K#1)
/// @generic.instance id=Box<K#2> template=Box arguments=(K#2)
/// @generic.instance id=probe<K#2> template=probe arguments=(K#2)
"#, "");
}

#[test]
fn test_constrained_parameter_takes_where_clause_bounds() {
    let session = TestSession::single(
        r#"
interface Hash {
    hash(): float64;
}

interface Equal<T> {
    equals(other: T): boolean;
}

declare function probe<T: Equal<T>>(value: T): boolean;

class Box<K> {
    key: K;

    constructor(key: K) {
        this.key = key;
    }
}

extension<K: Hash> of Box<K> where K: Equal<K> {
    check(): boolean {
        return probe(this.key);
    }
}
"#,
    );

    session.assert_dir_checked_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
interface Hash {
    hash(): float64;
}

interface Equal<T> {
    equals(other: T): boolean;
}

declare function probe<T: Equal<T>>(value: T): boolean;

class Box<K> {
    key: K;

    constructor(key: K): Box<K> {
        this.key = key;
    }
}

extension<K: Hash> of Box<K> where K: Equal<K> {
    check(): boolean {
        return probe<K>(this.key);
    }
}

=== checked ===
interface Hash {
/// @generic.template symbol=Hash parameters=()
/// @type.symbol symbol=Hash type=Hash
/// @definition.interface symbol=Hash template=()
/// @definition.method symbol=Hash.hash source="hash(): float64" slot=hash type=(this: Hash) => float64

    hash(): float64;
    /// @type.symbol symbol=Hash.hash source="hash(): float64" type=(this: Hash) => float64

}

interface Equal<T> {
/// @generic.template symbol=Equal parameters=(T#1)
/// @type.symbol symbol=Equal type=Equal
/// @definition.interface symbol=Equal template=(T#1)
/// @definition.where symbol=Equal relation=satisfies left=this right=Equal<T#1>
/// @definition.method symbol=Equal.equals source="equals(other: T): boolean" slot=equals type=(this: Equal<T#1>, T#1) => boolean
/// @type.symbol symbol=Equal.T source=T type=T#1

    equals(other: T): boolean;
    /// @type.symbol symbol=Equal.equals source="equals(other: T): boolean" type=(this: Equal<T#1>, T#1) => boolean
    /// @type.symbol symbol=Equal.equals.other source="other: T" type=T#1
    /// @resolution.name source=T target=Equal.T

}

declare function probe<T: Equal<T>>(value: T): boolean;
/// @generic.template symbol=probe parameters=(T#2: Equal<T#2>)
/// @type.symbol symbol=probe source="declare function probe<T: Equal<T>>(value: T): boolean" type=<T#2: Equal<T#2>>(T#2) => boolean
/// @type.symbol symbol=probe.T source="T: Equal<T>" type=T#2
/// @resolution.name source=Equal target=Equal
/// @resolution.name source=T target=probe.T
/// @type.symbol symbol=probe.value source="value: T" type=T#2
/// @resolution.name source=T target=probe.T

class Box<K> {
/// @generic.template symbol=Box parameters=(K#1)
/// @type.symbol symbol=Box type=Box
/// @definition.class symbol=Box template=(K#1)
/// @definition.field symbol=Box.key source="key: K" key=key type=K#1
/// @definition.method symbol=Box.constructor slot=constructor role=constructor type=(K#1) => Box<K#1>
/// @type.symbol symbol=Box.K source=K type=K#1

    key: K;
    /// @type.symbol symbol=Box.key source="key: K" type=K#1
    /// @resolution.name source=K target=Box.K

    constructor(key: K) {
    /// @type.symbol symbol=Box.constructor type=(K#1) => Box<K#1>
    /// @type.symbol symbol=Box.constructor.key source="key: K" type=K#1
    /// @resolution.name source=K target=Box.K

        this.key = key;
        /// @resolution.receiver source=this kind=this declaration=Box type=Box<K#1>
        /// @resolution.pattern.assign source=this.key kind=place place=field(Box.key) type=K#1
        /// @resolution.name source=key target=Box.constructor.key

    }
}

extension<K: Hash> of Box<K> where K: Equal<K> {
/// @generic.template symbol=<module>#2 parameters=(K#2: Hash)
/// @definition.extension symbol=<module>#2 form=local target=Box<K#2>
/// @definition.where symbol=<module>#2 source="K: Equal<K>" relation=satisfies left=K#2 right=Equal<K#2>
/// @definition.method symbol=check slot=check type=(this: Box<K#2>) => boolean
/// @type.symbol symbol=K source="K: Hash" type=K#2
/// @resolution.name source=Hash target=Hash
/// @resolution.name source=Box target=Box
/// @resolution.name source=K target=K
/// @resolution.name source=K target=K
/// @resolution.name source=Equal target=Equal
/// @resolution.name source=K target=K

    check(): boolean {
    /// @type.symbol symbol=check type=(this: Box<K#2>) => boolean

        return probe(this.key);
        /// @resolution.name source=probe target=probe
        /// @resolution.call source=probe(this.key) parameters=(K#2) arguments=(provided(this.key) as K#2) return=boolean kind=symbol target=probe instance=probe<K#2>
        /// @generic.instance source=probe(this.key) id=probe<K#2>
        /// @resolution.member source=this.key receiver=Box<K#2> kind=symbol target=Box.key
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Box<K#2>

    }
}

/// @generic.instance id=Box<K#1> template=Box arguments=(K#1)
/// @generic.instance id=Box<K#2> template=Box arguments=(K#2)
/// @generic.instance id=Equal<T#1> template=Equal arguments=(T#1)
/// @generic.instance id=probe<K#2> template=probe arguments=(K#2)
"#, "");
}

#[test]
fn test_member_selects_through_where_bound() {
    let session = TestSession::single(
        r#"
interface Doubling {
    double(): int32;
}

function twice<T>(value: T): int32 where T: Doubling {
    return value.double();
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Doubling {
    double(): int32;
}

function twice<T>(value: T): int32 where T: Doubling {
    return value.double();
}

=== checked ===
interface Doubling {
/// @generic.template symbol=Doubling parameters=()
/// @type.symbol symbol=Doubling type=Doubling
/// @definition.interface symbol=Doubling template=()
/// @definition.method symbol=Doubling.double source="double(): int32" slot=double type=(this: Doubling) => int32

    double(): int32;
    /// @type.symbol symbol=Doubling.double source="double(): int32" type=(this: Doubling) => int32

}

function twice<T>(value: T): int32 where T: Doubling {
/// @generic.template symbol=twice parameters=(T)
/// @type.symbol symbol=twice type=<T>(T) => int32
/// @type.symbol symbol=twice.T source=T type=T
/// @type.symbol symbol=twice.value source="value: T" type=T
/// @resolution.name source=T target=twice.T
/// @resolution.name source=T target=twice.T
/// @resolution.name source=Doubling target=Doubling

    return value.double();
    /// @resolution.name source=value target=twice.value
    /// @resolution.member source=value.double receiver=T kind=symbol target=Doubling.double
    /// @resolution.call source=value.double() parameters=() return=int32 kind=symbol target=Doubling.double receiver=T

}
"#,
    );
}

#[test]
fn test_exported_static_member_infers_extension_parameters() {
    let session = TestSession::single(
        r#"
declare function todo(message: string): never;

newtype Inner<T> = intrinsic;

extension<T> of Inner<T> {
    static new(value: T): Inner<T> {
        todo("Inner.new")
    }
}

export struct Cell<T> {
    storage: Inner<T>;
}

export extension<T> of Cell<T> {
    static new(value: T): Cell<T> {
        Cell { storage: Inner.new(value) }
    }
}
"#,
    );

    session.assert_dir_checked_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
declare function todo(message: string): never;

newtype Inner<T> = intrinsic;

extension<T> of Inner<T> {
    static new(value: T): Inner<T> {
        todo("Inner.new")
    }
}

export struct Cell<T> {
    storage: Inner<T>;
}

export extension<T> of Cell<T> {
    static new(value: T): Cell<T> {
        Cell { storage: Inner.new<T>(value) }
    }
}

=== checked ===
declare function todo(message: string): never;
/// @type.symbol symbol=todo source="declare function todo(message: string): never" type=(string) => never
/// @type.symbol symbol=todo.message source="message: string" type=string

newtype Inner<T> = intrinsic;
/// @generic.template symbol=Inner parameters=(T#1)
/// @type.symbol symbol=Inner source="newtype Inner<T> = intrinsic" type=Inner
/// @definition.newtype symbol=Inner source="newtype Inner<T> = intrinsic" template=(T#1) value=intrinsic
/// @type.symbol symbol=Inner.T source=T type=T#1

extension<T> of Inner<T> {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @definition.extension symbol=<module>#2 form=local target=Inner<T#2>
/// @definition.method symbol=new#1 slot=new static=true type=(T#2) => Inner<T#2>
/// @type.symbol symbol=T#1 source=T type=T#2
/// @resolution.name source=Inner target=Inner
/// @resolution.name source=T target=T#1

    static new(value: T): Inner<T> {
    /// @type.symbol symbol=new#1 type=(T#2) => Inner<T#2>
    /// @type.symbol symbol=new.value#1 source="value: T" type=T#2
    /// @resolution.name source=T target=T#1
    /// @resolution.name source=Inner target=Inner
    /// @resolution.name source=T target=T#1

        todo("Inner.new")
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"Inner.new\")" parameters=(string) arguments=(provided("Inner.new") as string) return=never kind=symbol target=todo

    }
}

export struct Cell<T> {
/// @generic.template symbol=Cell parameters=(T#3)
/// @type.symbol symbol=Cell type=Cell
/// @definition.struct symbol=Cell template=(T#3)
/// @definition.field symbol=Cell.storage source="storage: Inner<T>" key=storage type=Inner<T#3>
/// @type.symbol symbol=Cell.T source=T type=T#3

    storage: Inner<T>;
    /// @type.symbol symbol=Cell.storage source="storage: Inner<T>" type=Inner<T#3>
    /// @resolution.name source=Inner target=Inner
    /// @resolution.name source=T target=Cell.T

}

export extension<T> of Cell<T> {
/// @generic.template symbol=<module>#3 parameters=(T#4)
/// @definition.extension symbol=<module>#3 form=exported target=Cell<T#4>
/// @definition.method symbol=new#2 slot=new static=true type=(T#4) => Cell<T#4>
/// @type.symbol symbol=T#2 source=T type=T#4
/// @resolution.name source=Cell target=Cell
/// @resolution.name source=T target=T#2

    static new(value: T): Cell<T> {
    /// @type.symbol symbol=new#2 type=(T#4) => Cell<T#4>
    /// @type.symbol symbol=new.value#2 source="value: T" type=T#4
    /// @resolution.name source=T target=T#2
    /// @resolution.name source=Cell target=Cell
    /// @resolution.name source=T target=T#2

        Cell { storage: Inner.new(value) }
        /// @resolution.name source=Cell target=Cell
        /// @resolution.name source=Inner target=Inner
        /// @resolution.member source=Inner.new receiver=Inner kind=symbol target=new#1
        /// @resolution.call source=Inner.new(value) parameters=(T#4) arguments=(provided(value) as T#4) return=Inner<T#4> kind=symbol target=new#1 receiver=Inner
        /// @resolution.name source=value target=new.value#2

    }
}

/// @generic.instance id=Cell<T#4> template=Cell arguments=(T#4)
/// @generic.instance id=Inner<T#2> template=Inner arguments=(T#2)
/// @generic.instance id=Inner<T#3> template=Inner arguments=(T#3)
"#, r#""#);
}

#[test]
fn test_static_member_selects_through_a_parameter_bound() {
    let session = TestSession::single(
        r#"
interface Makeable {
    static make(): this;
}

function build<T: Makeable>(): T {
    T.make()
}
"#,
    );

    session.assert_dir_checked_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
interface Makeable {
    static make(): this;
}

function build<T: Makeable>(): T {
    T.make()
}

=== checked ===
interface Makeable {
/// @generic.template symbol=Makeable parameters=()
/// @type.symbol symbol=Makeable type=Makeable
/// @definition.interface symbol=Makeable template=()
/// @definition.method symbol=Makeable.make source="static make(): this" slot=make static=true type=() => this

    static make(): this;
    /// @type.symbol symbol=Makeable.make source="static make(): this" type=() => this

}

function build<T: Makeable>(): T {
/// @generic.template symbol=build parameters=(T: Makeable)
/// @type.symbol symbol=build type=<T: Makeable>() => T
/// @type.symbol symbol=build.T source="T: Makeable" type=T
/// @resolution.name source=Makeable target=Makeable
/// @resolution.name source=T target=build.T

    T.make()
    /// @resolution.name source=T target=build.T
    /// @resolution.member source=T.make receiver=T kind=symbol target=Makeable.make
    /// @resolution.call source=T.make() parameters=() return=T kind=symbol target=Makeable.make receiver=T

}
"#, r#""#);
}

#[test]
fn test_static_member_infers_extension_parameters_at_calls() {
    let session = TestSession::single(
        r#"
class Box<T> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }
}

extension<T> of Box<T> {
    static make(value: T): Box<T> {
        new Box<T>(value)
    }
}

extension<T> of Box<T> {
    static wrap(value: T): Box<T> {
        Box.make(value)
    }
}
"#,
    );

    session.assert_dir_checked_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
class Box<T> {
    value: T;

    constructor(value: T): Box<T> {
        this.value = value;
    }
}

extension<T> of Box<T> {
    static make(value: T): Box<T> {
        new Box<T>(value)
    }
}

extension<T> of Box<T> {
    static wrap(value: T): Box<T> {
        Box.make<T>(value)
    }
}

=== checked ===
class Box<T> {
/// @generic.template symbol=Box parameters=(T#1)
/// @type.symbol symbol=Box type=Box
/// @definition.class symbol=Box template=(T#1)
/// @definition.field symbol=Box.value source="value: T" key=value type=T#1
/// @definition.method symbol=Box.constructor slot=constructor role=constructor type=(T#1) => Box<T#1>
/// @type.symbol symbol=Box.T source=T type=T#1

    value: T;
    /// @type.symbol symbol=Box.value source="value: T" type=T#1
    /// @resolution.name source=T target=Box.T

    constructor(value: T) {
    /// @type.symbol symbol=Box.constructor type=(T#1) => Box<T#1>
    /// @type.symbol symbol=Box.constructor.value source="value: T" type=T#1
    /// @resolution.name source=T target=Box.T

        this.value = value;
        /// @resolution.receiver source=this kind=this declaration=Box type=Box<T#1>
        /// @resolution.pattern.assign source=this.value kind=place place=field(Box.value) type=T#1
        /// @resolution.name source=value target=Box.constructor.value

    }
}

extension<T> of Box<T> {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @definition.extension symbol=<module>#2 form=local target=Box<T#2>
/// @definition.method symbol=make slot=make static=true type=(T#2) => Box<T#2>
/// @type.symbol symbol=T#1 source=T type=T#2
/// @resolution.name source=Box target=Box
/// @resolution.name source=T target=T#1

    static make(value: T): Box<T> {
    /// @type.symbol symbol=make type=(T#2) => Box<T#2>
    /// @type.symbol symbol=make.value source="value: T" type=T#2
    /// @resolution.name source=T target=T#1
    /// @resolution.name source=Box target=Box
    /// @resolution.name source=T target=T#1

        new Box<T>(value)
        /// @resolution.construct source="new Box<T>(value)" parameters=(T#2) arguments=(provided(value) as T#2) return=Box<T#2> kind=class target=Box constructor=Box.constructor instance=Box<T#2>
        /// @generic.instance source="new Box<T>(value)" id=Box<T#2>
        /// @resolution.name source=Box target=Box
        /// @resolution.name source=T target=T#1
        /// @resolution.name source=value target=make.value

    }
}

extension<T> of Box<T> {
/// @generic.template symbol=<module>#3 parameters=(T#3)
/// @definition.extension symbol=<module>#3 form=local target=Box<T#3>
/// @definition.method symbol=wrap slot=wrap static=true type=(T#3) => Box<T#3>
/// @type.symbol symbol=T#2 source=T type=T#3
/// @resolution.name source=Box target=Box
/// @resolution.name source=T target=T#2

    static wrap(value: T): Box<T> {
    /// @type.symbol symbol=wrap type=(T#3) => Box<T#3>
    /// @type.symbol symbol=wrap.value source="value: T" type=T#3
    /// @resolution.name source=T target=T#2
    /// @resolution.name source=Box target=Box
    /// @resolution.name source=T target=T#2

        Box.make(value)
        /// @resolution.name source=Box target=Box
        /// @resolution.member source=Box.make receiver=Box kind=symbol target=make
        /// @resolution.call source=Box.make(value) parameters=(T#3) arguments=(provided(value) as T#3) return=Box<T#3> kind=symbol target=make receiver=Box
        /// @resolution.name source=value target=wrap.value

    }
}

/// @generic.instance id=Box<T#1> template=Box arguments=(T#1)
/// @generic.instance id=Box<T#2> template=Box arguments=(T#2)
/// @generic.instance id=Box<T#3> template=Box arguments=(T#3)
"#, r#""#);
}

#[test]
fn test_expected_field_type_drives_static_member_inference() {
    let session = TestSession::single(
        r#"
struct Inner<T> {
    value: T;
}

extension<T> of Inner<T> {
    static new(value: T): Inner<T> {
        Inner { value }
    }
}

struct Outer<T> {
    inner: Inner<T>;
}

extension<T> of Outer<T> {
    static new(value: T): Outer<T> {
        Outer { inner: Inner.new(value) }
    }
}
"#,
    );

    session.assert_dir_checked_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
struct Inner<T> {
    value: T;
}

extension<T> of Inner<T> {
    static new(value: T): Inner<T> {
        Inner { value }
    }
}

struct Outer<T> {
    inner: Inner<T>;
}

extension<T> of Outer<T> {
    static new(value: T): Outer<T> {
        Outer { inner: Inner.new<T>(value) }
    }
}

=== checked ===
struct Inner<T> {
/// @generic.template symbol=Inner parameters=(T#1)
/// @type.symbol symbol=Inner type=Inner
/// @definition.struct symbol=Inner template=(T#1)
/// @definition.field symbol=Inner.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Inner.T source=T type=T#1

    value: T;
    /// @type.symbol symbol=Inner.value source="value: T" type=T#1
    /// @resolution.name source=T target=Inner.T

}

extension<T> of Inner<T> {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @definition.extension symbol=<module>#2 form=local target=Inner<T#2>
/// @definition.method symbol=new#1 slot=new static=true type=(T#2) => Inner<T#2>
/// @type.symbol symbol=T#1 source=T type=T#2
/// @resolution.name source=Inner target=Inner
/// @resolution.name source=T target=T#1

    static new(value: T): Inner<T> {
    /// @type.symbol symbol=new#1 type=(T#2) => Inner<T#2>
    /// @type.symbol symbol=new.value#1 source="value: T" type=T#2
    /// @resolution.name source=T target=T#1
    /// @resolution.name source=Inner target=Inner
    /// @resolution.name source=T target=T#1

        Inner { value }
        /// @resolution.name source=Inner target=Inner
        /// @resolution.name source=value target=new.value#1

    }
}

struct Outer<T> {
/// @generic.template symbol=Outer parameters=(T#3)
/// @type.symbol symbol=Outer type=Outer
/// @definition.struct symbol=Outer template=(T#3)
/// @definition.field symbol=Outer.inner source="inner: Inner<T>" key=inner type=Inner<T#3>
/// @type.symbol symbol=Outer.T source=T type=T#3

    inner: Inner<T>;
    /// @type.symbol symbol=Outer.inner source="inner: Inner<T>" type=Inner<T#3>
    /// @resolution.name source=Inner target=Inner
    /// @resolution.name source=T target=Outer.T

}

extension<T> of Outer<T> {
/// @generic.template symbol=<module>#3 parameters=(T#4)
/// @definition.extension symbol=<module>#3 form=local target=Outer<T#4>
/// @definition.method symbol=new#2 slot=new static=true type=(T#4) => Outer<T#4>
/// @type.symbol symbol=T#2 source=T type=T#4
/// @resolution.name source=Outer target=Outer
/// @resolution.name source=T target=T#2

    static new(value: T): Outer<T> {
    /// @type.symbol symbol=new#2 type=(T#4) => Outer<T#4>
    /// @type.symbol symbol=new.value#2 source="value: T" type=T#4
    /// @resolution.name source=T target=T#2
    /// @resolution.name source=Outer target=Outer
    /// @resolution.name source=T target=T#2

        Outer { inner: Inner.new(value) }
        /// @resolution.name source=Outer target=Outer
        /// @resolution.name source=Inner target=Inner
        /// @resolution.member source=Inner.new receiver=Inner kind=symbol target=new#1
        /// @resolution.call source=Inner.new(value) parameters=(T#4) arguments=(provided(value) as T#4) return=Inner<T#4> kind=symbol target=new#1 receiver=Inner
        /// @resolution.name source=value target=new.value#2

    }
}

/// @generic.instance id=Inner<T#2> template=Inner arguments=(T#2)
/// @generic.instance id=Inner<T#3> template=Inner arguments=(T#3)
/// @generic.instance id=Outer<T#4> template=Outer arguments=(T#4)
"#, r#""#);
}

#[test]
fn test_call_result_assigns_into_a_union_result() {
    let session = TestSession::single(
        r#"
function pair<T>(a: T): (T, boolean) {
    (a, true)
}

function check<T>(a: T): T | undefined {
    let (result, overflow) = pair(a);

    if (overflow) {
        return undefined;
    }

    result
}
"#,
    );

    session.assert_dir_checked_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
function pair<T>(a: T): (T, boolean) {
    (a, true)
}

function check<T>(a: T): T | undefined {
    let (result, overflow) = pair<T>(a);

    if (overflow) {
        return undefined as T | undefined;
    }

    result
} as T | undefined

=== checked ===
function pair<T>(a: T): (T, boolean) {
/// @generic.template symbol=pair parameters=(T#1)
/// @type.symbol symbol=pair type=<T#1>(T#1) => (T#1, boolean)
/// @type.symbol symbol=pair.T source=T type=T#1
/// @type.symbol symbol=pair.a source="a: T" type=T#1
/// @resolution.name source=T target=pair.T
/// @resolution.name source=T target=pair.T

    (a, true)
    /// @resolution.name source=a target=pair.a

}

function check<T>(a: T): T | undefined {
/// @generic.template symbol=check parameters=(T#2)
/// @type.symbol symbol=check type=<T#2>(T#2) => T#2 | undefined
/// @type.symbol symbol=check.T source=T type=T#2
/// @type.symbol symbol=check.a source="a: T" type=T#2
/// @resolution.name source=T target=check.T
/// @resolution.name source=T target=check.T

    let (result, overflow) = pair(a);
    /// @resolution.pattern source=(result, overflow) kind=tuple fields=(check.result, check.overflow)
    /// @type.symbol symbol=check.result source=result type=T#2
    /// @resolution.pattern source=result kind=binding target=check.result
    /// @type.symbol symbol=check.overflow source=overflow type=boolean
    /// @resolution.pattern source=overflow kind=binding target=check.overflow
    /// @resolution.name source=pair target=pair
    /// @resolution.call source=pair(a) parameters=(T#2) arguments=(provided(a) as T#2) return=(T#2, boolean) kind=symbol target=pair instance=pair<T#2>
    /// @generic.instance source=pair(a) id=pair<T#2>
    /// @resolution.name source=a target=check.a

    if (overflow) {
    /// @resolution.name source=overflow target=check.overflow

        return undefined;
    }

    result
    /// @resolution.name source=result target=check.result

}

/// @generic.instance id=pair<T#2> template=pair arguments=(T#2)
"#, r#""#);
}
