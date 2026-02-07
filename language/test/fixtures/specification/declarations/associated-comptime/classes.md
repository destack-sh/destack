# Associated Comptime Constants: Classes

Class associated comptime constant tests live here.

## classes

### class can declare associated comptime constants

> Classes can declare associated compile time constants.
> Declares an owner-scoped `comptime const` on a class and uses it in a sibling field type.
> The projection `MessagePage.Rows` must resolve to a folded compile-time literal.

```ds
class MessagePage {
    comptime const Rows: number = 128;
    data: uint8[Rows];
}

declare const rows: MessagePage.Rows;
rows satisfies 128;
```

### class associated comptime constants can use outer type substitutions

> Associated comptime constants can depend on class static type parameters.
> Specializes one owner with two different outer type arguments and projects the same associated constant from both.
> The two projections must fold to different literals after owner substitution.

```ds
class SegmentPlan<Row> {
    comptime const SegmentBytes: number = Row extends string ? 4096 : 1024;
}

declare const logSegment: SegmentPlan<string>.SegmentBytes;
declare const metricSegment: SegmentPlan<int32>.SegmentBytes;
logSegment satisfies 4096;
metricSegment satisfies 1024;
```

### class associated comptime constants require static initializers

> Class associated comptime constant initializers must be static expressions.
> Uses a runtime-only initializer in an associated comptime declaration.
> The declaration must fail in Analyze static evaluation.

```ds
class BadConfig {
    comptime const SegmentBytes: number = Date.now();
}
```

- contains: static expression

### class associated comptime constants require initializer

> Class associated comptime constants cannot be declaration only.
> Declares a class associated comptime member without an initializer.
> The compiler must reject the declaration because projection requires a concrete compile-time value.

```ds
class BadConfig {
    comptime const SegmentBytes: number;
}
```

- contains: initializer

### class associated comptime constants enforce annotation compatibility

> Initializer type must satisfy the declared associated comptime constant type.
> Assigns an incompatible initializer type to a typed associated comptime member.
> The declaration should fail with an assignment-compatibility diagnostic.

```ds
class BadConfig {
    comptime const SegmentBytes: number = "x";
}
```

- contains: not assignable

### class static const remains runtime member not associated comptime member

> Runtime static constants remain runtime members and cannot use type only relations.
> Uses `static const` with a type-dependent expression that requires type-space evaluation.
> The fixture locks the runtime/type-space boundary between `static const` and `comptime const`.

```ds
class SegmentPlan<Row> {
    static const SegmentBytes: number = Row extends string ? 4096 : 1024;
}
```

- contains: type

### class static comptime const is rejected as redundant

> `comptime const` is already static by owner scope and cannot be combined with `static`.
> Declares a redundant modifier pair on an associated comptime member.
> The parser/analyzer surface should reject `static comptime const` as invalid form.

```ds
class BadConfig {
    static comptime const SegmentBytes: number = 1024;
}
```

- contains: static comptime const

### class associated comptime constants can coexist with runtime static constants

> Class declarations can define both associated compile time constants and runtime static constants.
> Defines both member kinds on one class and uses each through the proper access path.
> This confirms associated compile-time projection and runtime static access can coexist without ambiguity.

```ds
class StorageProfile<Row> {
    comptime const SegmentBytes: number = Row extends string ? 4096 : 1024;
    static const SchemaVersion: number = 3;

    static currentVersion(): number {
        StorageProfile.SchemaVersion
    }
}

const version = StorageProfile<string>.currentVersion();
version satisfies number;
```

### abstract classes can declare abstract associated comptime constants

> Abstract classes can defer associated comptime constants to concrete subclasses.
> Declares an abstract associated comptime requirement in a base class and fulfills it in a concrete subclass.
> The subclass projection must type-check through inherited aliases after override selection.

```ds
abstract class BatchPlan<Row> {
    abstract comptime const SegmentRows: number;
    type Segment = Row[SegmentRows];
}

class LogBatch extends BatchPlan<string> {
    comptime const SegmentRows: number = 256;
}

// inherited owner contracts should be selected before projection
declare const segment: LogBatch.Segment;
segment satisfies string[256];
```

### concrete subclasses must implement abstract associated comptime constants

> Concrete subclasses must define inherited abstract associated comptime constants.
> Leaves an inherited abstract associated comptime requirement unimplemented in a concrete subclass.
> The class declaration must fail with a missing-associated-member diagnostic.

```ds
abstract class BatchPlan<Row> {
    abstract comptime const SegmentRows: number;
}

class LogBatch extends BatchPlan<string> {}
```

- contains: missing associated
