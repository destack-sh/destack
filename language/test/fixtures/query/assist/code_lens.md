# Code Lens

## Function Lenses

### Reference and test lenses

Code lenses should include reference counts and test actions.

```ds
$0function greet(): void {}

greet();

@test
function test_example(): void {}
```

```query code_lens $0
main.ds:1:10-1:15 kind=references title=1 reference
main.ds:6:10-6:22 kind=run_test title=▶ Run test_example
```

## Resolve

### Resolved lenses preserve titles

Resolving a code lens should keep the title when it is already resolved.

```ds
function say_$0hi(): void {}
say_hi();
```

```query resolve_code_lens $0
<same>
```

## Implementations

### Implementation lenses for interfaces and classes

Interfaces should show implementation counts and classes should show subclass counts.

```ds
interface Animal {
    speak(): void;
}

class Dog implements Animal {
    speak(): void {}
}

class Puppy extends Dog {}
```

```query code_lens $0
main.ds:1:11-1:17 kind=implementations title=1 implementation
main.ds:5:7-5:10 kind=implementations title=1 implementation
```

## Resolve Index

### Resolve can select by index

Resolve should support selecting a lens by index when multiple lenses are present.

```ds
$0function first(): void {}
first();

function second(): void {}
second();
```

```query resolve_code_lens $0 1
<same>
```

## Empty Results

### Unused functions produce no lenses

Functions without references, test decorators, or implementation relationships should not produce code lenses.

```ds
function idle(): void {}
```

```query code_lens $0
<none>
```
