
## Functions

### Return the same function item from its declaration and calls

A function name and every direct call identify the same callable declaration.

```tspp main.tspp
function callee(): void {}
^^^^^^^^^^^^^^^^^^^^^^^^^^ declaration
         ^^^^^^ name

callee();
^^^^^^ call
```

```query call_item main.tspp#name
@call_item.item name=callee kind=function signature="callee(): void" location=main.tspp#declaration selection=main.tspp#name symbol=main.tspp#callee@1
```

```query call_item main.tspp#call
@call_item.item name=callee kind=function signature="callee(): void" location=main.tspp#declaration selection=main.tspp#name symbol=main.tspp#callee@1
```

### Resolve the current call target

A call identifies the function selected after each edit.

```tspp main.tspp
function first(): void {}
^^^^^^^^^^^^^^^^^^^^^^^^^ declaration:first
         ^^^^^ name:first

function second(): void {}
^^^^^^^^^^^^^^^^^^^^^^^^^^ declaration:second
         ^^^^^^ name:second

first();
^^^^^ call
```

```query call_item main.tspp#call
@call_item.item name=first kind=function signature="first(): void" location=main.tspp#declaration:first selection=main.tspp#name:first symbol=main.tspp#first@1
```

```tspp main.tspp change
function first(): void {}
^^^^^^^^^^^^^^^^^^^^^^^^^ declaration:first
         ^^^^^ name:first

function second(): void {}
^^^^^^^^^^^^^^^^^^^^^^^^^^ declaration:second
         ^^^^^^ name:second

second();
^^^^^^ call
```

```query call_item main.tspp#call
@call_item.item name=second kind=function signature="second(): void" location=main.tspp#declaration:second selection=main.tspp#name:second symbol=main.tspp#second@2
```

## Methods

### Return the same method item from its declaration and calls

A method name and its calls identify the same qualified declaration.

```tspp main.tspp
class Service {
    run(): void {}
    ^^^^^^^^^^^^^^ declaration
    ^^^ name
}

const service = new Service();
service.run();
        ^^^ call
```

```query call_item main.tspp#name
@call_item.item name=run kind=method signature="Service.run(): void" location=main.tspp#declaration selection=main.tspp#name symbol=main.tspp#run@2
```

```query call_item main.tspp#call
@call_item.item name=run kind=method signature="Service.run(): void" location=main.tspp#declaration selection=main.tspp#name symbol=main.tspp#run@2
```

## Imports

### Resolve imported and aliased names to the defining function

Imported names, local aliases, and calls through those aliases identify the function in its defining module.

```tspp library.tspp
export function target(): void {}
^ declaration:start
                                 ^ declaration:end
                ^^^^^^ name
```

```tspp main.tspp
import { target as localTarget } from "./library.tspp";
         ^^^^^^ imported_name
                   ^^^^^^^^^^^ local_name

localTarget();
^^^^^^^^^^^ call
```

```query call_item main.tspp#imported_name
@call_item.item name=target kind=function signature="target(): void" location=library.tspp#declaration selection=library.tspp#name symbol=library.tspp#target@1
```

```query call_item main.tspp#local_name
@call_item.item name=target kind=function signature="target(): void" location=library.tspp#declaration selection=library.tspp#name symbol=library.tspp#target@1
```

```query call_item main.tspp#call
@call_item.item name=target kind=function signature="target(): void" location=library.tspp#declaration selection=library.tspp#name symbol=library.tspp#target@1
```

## Overloads

### Return the matching overload

Each call identifies the overload that accepts its argument.

```tspp main.tspp
function parse(value: int32): int32 {
^ integer_declaration:start
         ^^^^^ integer_name
    return value;
}
^ integer_declaration:end

function parse(value: string): string {
^ string_declaration:start
         ^^^^^ string_name
    return value;
}
^ string_declaration:end

const integerValue = parse(1);
                     ^^^^^ integer_call
const stringValue = parse("ok");
                    ^^^^^ string_call
```

```query call_item main.tspp#integer_call
@call_item.item name=parse kind=function signature="parse(value: int32): int32" location=main.tspp#integer_declaration selection=main.tspp#integer_name symbol=main.tspp#parse@1
```

```query call_item main.tspp#string_call
@call_item.item name=parse kind=function signature="parse(value: string): string" location=main.tspp#string_declaration selection=main.tspp#string_name symbol=main.tspp#parse@3
```

## Generic Functions

### Return the declared generic function

A generic declaration and an applied call identify the same generic callable item.

```tspp main.tspp
function identity<T>(value: T): T {
^ declaration:start
         ^^^^^^^^ name
    return value;
}
^ declaration:end

const result = identity<string>("value");
               ^^^^^^^^ call
```

```query call_item main.tspp#name
@call_item.item name=identity kind=function signature="identity<T>(value: T): T" location=main.tspp#declaration selection=main.tspp#name symbol=main.tspp#identity@1
```

```query call_item main.tspp#call
@call_item.item name=identity kind=function signature="identity<T>(value: T): T" location=main.tspp#declaration selection=main.tspp#name symbol=main.tspp#identity@1
```

## Extensions

### Return an extension method

An extension call identifies its method declaration.

```tspp main.tspp
struct Calculator {}

extension of Calculator {
    add(left: int32, right: int32): int32 {
    ^ declaration:start
    ^^^ name
        return left + right;
    }
    ^ declaration:end
}

const calculator = Calculator {};
calculator.add(1, 2);
           ^^^ call
```

```query call_item main.tspp#name
@call_item.item name=add kind=method signature="Calculator.add(left: int32, right: int32): int32" location=main.tspp#declaration selection=main.tspp#name symbol=main.tspp#add@3
```

```query call_item main.tspp#call
@call_item.item name=add kind=method signature="Calculator.add(left: int32, right: int32): int32" location=main.tspp#declaration selection=main.tspp#name symbol=main.tspp#add@3
```

## Class Constructors

### Return a declared constructor

A constructor declaration and its constructions identify the same constructor member.

```tspp main.tspp
class User {
    constructor(name: string) {}
    ^ declaration:start
                                ^ declaration:end
    ^^^^^^^^^^^ name
}

const user = new User("Ada");
                 ^^^^ call
```

```query call_item main.tspp#name
@call_item.item name=constructor kind=constructor signature="User.constructor(name: string)" location=main.tspp#declaration selection=main.tspp#name symbol=main.tspp#symbol@2
```

```query call_item main.tspp#call
@call_item.item name=constructor kind=constructor signature="User.constructor(name: string)" location=main.tspp#declaration selection=main.tspp#name symbol=main.tspp#symbol@2
```

### Return the class item for a default constructor

A class without a constructor declaration owns its default construction item.

```tspp main.tspp
class User {}
^^^^^^^^^^^^^ declaration
      ^^^^ name

const user = new User();
                 ^^^^ call
```

```query call_item main.tspp#name
@call_item.item name=User kind=constructor signature="User(): User" location=main.tspp#declaration selection=main.tspp#name symbol=main.tspp#User@1
```

```query call_item main.tspp#call
@call_item.item name=User kind=constructor signature="User(): User" location=main.tspp#declaration selection=main.tspp#name symbol=main.tspp#User@1
```

## Newtype Constructors

### Return a newtype constructor from its declaration

A newtype declaration identifies its constructor.

```tspp main.tspp
newtype UserId = string;
^^^^^^^^^^^^^^^^^^^^^^^ declaration
        ^^^^^^ name
```

```query call_item main.tspp#name
@call_item.item name=UserId kind=constructor signature="UserId(string): UserId" location=main.tspp#declaration selection=main.tspp#name symbol=main.tspp#UserId@1
```

### Return a newtype constructor from its construction

A newtype construction identifies its constructor.

```tspp main.tspp
newtype UserId = string;
^^^^^^^^^^^^^^^^^^^^^^^ declaration
        ^^^^^^ name

const userId = UserId("user-1");
               ^^^^^^ call
```

```query call_item main.tspp#call
@call_item.item name=UserId kind=constructor signature="UserId(string): UserId" location=main.tspp#declaration selection=main.tspp#name symbol=main.tspp#UserId@1
```

## Indirect Calls

### Return no item for a function-valued binding

Calling through a variable does not identify a declaration-backed item.

```tspp main.tspp
function callee(): void {}

const callback = callee;
callback();
^^^^^^^^ call
```

```query call_item main.tspp#call
@call_item.none
```

## Union Dispatch

### Return no item when a call has multiple targets

A union receiver can select a finite set of methods, but it does not identify one hierarchy item.

```tspp main.tspp
class Alpha {
    run(): void {}
}

class Beta {
    run(): void {}
}

function start(service: Alpha | Beta): void {
    service.run();
            ^^^ call
}
```

```query call_item main.tspp#call
@call_item.none
```

## Non-callable Symbols

### Return no item for a value binding

A non-callable symbol has no call hierarchy item.

```tspp main.tspp
const value = 1;
      ^^^^^ value
```

```query call_item main.tspp#value
@call_item.none
```

### Return no item for an enum member

An enum member is a value rather than a callable constructor.

```tspp main.tspp
enum Status {
    Ready,
    ^^^^^ value
}
```

```query call_item main.tspp#value
@call_item.none
```
