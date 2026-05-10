# Declaration Heritage

Declaration heritage fixtures cover class extends, implements, generic boundaries, and related comments.

## Class Heritage and Generics

### class extends and implements with boundary comments

Comments before `implements` stay with the heritage clause.
Trailing line comments after the final heritage item become the first body comment.

```ts:main.ts
class Derived extends Base // base-tail
implements
// impl-head
A,
B // impl-tail
{}
```

```ts expected
class Derived
    extends Base // base-tail
    // impl-head
    implements A, B {
    // impl-tail
}
```

### declare class with implements and generic comment

Comments on declare class generic and implements boundaries stay attached to declarations.

```ts:main.ts
declare class Box // box-head
<T> implements Item<T>, Other {
  value: T
}
```

```ts expected
declare class Box<T> // box-head
    implements Item<T>, Other
{
    value: T;
}
```

## Class Heritage Comments

### class superclass boundary comment

Comments around superclass boundaries stay attached to class heritage heads.

```ts:main.ts
class Child extends Base // extends-tail
{
  value = 1
}
```

```ts expected
class Child extends Base {
    // extends-tail
    value = 1;
}
```

### class implement list comment placement

Comments in implement lists stay with intermediate items.
Trailing line comments after the final heritage item become the first body comment.

```ts:main.ts
class Child implements First, // impl-first
Second // impl-second
{
  value = 1
}
```

```ts expected
class Child
    implements
        First, // impl-first
        Second
{
    // impl-second
    value = 1;
}
```
