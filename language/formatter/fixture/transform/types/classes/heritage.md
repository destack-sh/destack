# Declaration Heritage

Declaration heritage fixtures cover class extends, implements, generic boundaries, and related comments.

## Class Heritage and Generics

### class extends and implements with boundary comments

Comments before `implements` stay with the heritage clause.
Trailing line comments after the final heritage item become the first body comment.

```tspp:main.tspp
class Derived extends Base // base-tail
implements
// impl-head
A,
B // impl-tail
{}
```

```tspp expected
class Derived
    extends Base // base-tail
    // impl-head
    implements A, B
{
    // impl-tail
}
```

### declare class with implements and generic comment

Comments on declare class generic and implements boundaries stay attached to declarations.

```tspp:main.tspp
declare class Box // box-head
<T> implements Item<T>, Other {
  value: T
}
```

```tspp expected
declare class Box<T> // box-head
    implements Item<T>, Other
{
    value: T;
}
```

### class with multiline implements

Long implemented interface lists break before the keyword and indent each implemented type.

```tspp:main.tspp line-width=60
class Worker implements VeryLongInterfaceNameOne, VeryLongInterfaceNameTwo, VeryLongInterfaceNameThree {
  value: string
}
```

```tspp expected
class Worker
    implements
        VeryLongInterfaceNameOne,
        VeryLongInterfaceNameTwo,
        VeryLongInterfaceNameThree
{
    value: string;
}
```

## Class Heritage Comments

### class superclass boundary comment

Comments around superclass boundaries stay attached to class heritage heads.

```tspp:main.tspp
class Child extends Base // extends-tail
{
  value = 1
}
```

```tspp expected
class Child extends Base {
    // extends-tail
    value = 1;
}
```

### class implement list comment placement

Comments in implement lists stay with intermediate items.
Trailing line comments after the final heritage item become the first body comment.

```tspp:main.tspp
class Child implements First, // impl-first
Second // impl-second
{
  value = 1
}
```

```tspp expected
class Child
    implements
        First, // impl-first
        Second
{
    // impl-second
    value = 1;
}
```
