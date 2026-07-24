# Call Item

## Functions

### Return the same function item from its declaration and calls

A function name and every direct call identify the same callable declaration.

```ds main.ds
function callee(): void {}
^^^^^^^^^^^^^^^^^^^^^^^^^^ declaration
         ^^^^^^ name

callee();
^^^^^^ call
```

```query call_item main.ds#name
@call_item.item name=callee kind=function detail="callee(): void" location=main.ds#declaration selection=main.ds#name symbol=main.ds#callee@1
```

```query call_item main.ds#call
@call_item.item name=callee kind=function detail="callee(): void" location=main.ds#declaration selection=main.ds#name symbol=main.ds#callee@1
```

## Methods

### Return the same method item from its declaration and calls

A method name and every statically selected call identify the same qualified callable declaration.

```ds main.ds
class Service {
    run(): void {}
    ^^^^^^^^^^^^^^ declaration
    ^^^ name
}

const service = new Service();
service.run();
        ^^^ call
```

```query call_item main.ds#name
@call_item.item name=run kind=method detail="Service.run(): void" location=main.ds#declaration selection=main.ds#name symbol=main.ds#run@2
```

```query call_item main.ds#call
@call_item.item name=run kind=method detail="Service.run(): void" location=main.ds#declaration selection=main.ds#name symbol=main.ds#run@2
```

## Imports

### Resolve imported and aliased names to the defining function

Imported names, local aliases, and calls through those aliases identify the function in its defining module.

```ds library.ds
export function target(): void {}
                ^^^^^^ name
```

```ds main.ds
import { target as localTarget } from "./library.ds";
         ^^^^^^ imported_name
                   ^^^^^^^^^^^ local_name

localTarget();
^^^^^^^^^^^ call
```

```query call_item main.ds#imported_name
@call_item.item name=target kind=function detail="target(): void" location=library.ds:1:1-1:34 selection=library.ds#name symbol=library.ds#target@1
```

```query call_item main.ds#local_name
@call_item.item name=target kind=function detail="target(): void" location=library.ds:1:1-1:34 selection=library.ds#name symbol=library.ds#target@1
```

```query call_item main.ds#call
@call_item.item name=target kind=function detail="target(): void" location=library.ds:1:1-1:34 selection=library.ds#name symbol=library.ds#target@1
```

## Overloads

### Return the selected overload

Each call identifies the exact overload selected by checking.

```ds main.ds
function parse(value: int32): int32 {
         ^^^^^ integer_name
    return value;
}

function parse(value: string): string {
         ^^^^^ string_name
    return value;
}

const integerValue = parse(1);
                     ^^^^^ integer_call
const stringValue = parse("ok");
                    ^^^^^ string_call
```

```query call_item main.ds#integer_call
@call_item.item name=parse kind=function detail="parse(value: int32): int32" location=main.ds:1:1-3:2 selection=main.ds#integer_name symbol=main.ds#parse@1
```

```query call_item main.ds#string_call
@call_item.item name=parse kind=function detail="parse(value: string): string" location=main.ds:5:1-7:2 selection=main.ds#string_name symbol=main.ds#parse@3
```

## Extensions

### Return the selected extension method

An extension call identifies the concrete extension member selected by checking.

```ds main.ds
struct Calculator {}

extension of Calculator {
    add(left: int32, right: int32): int32 {
    ^^^ name
        return left + right;
    }
}

const calculator = Calculator {};
calculator.add(1, 2);
           ^^^ call
```

```query call_item main.ds#name
@call_item.item name=add kind=method detail="Calculator.add(left: int32, right: int32): int32" location=main.ds:4:5-6:6 selection=main.ds#name symbol=main.ds#add@3
```

```query call_item main.ds#call
@call_item.item name=add kind=method detail="Calculator.add(left: int32, right: int32): int32" location=main.ds:4:5-6:6 selection=main.ds#name symbol=main.ds#add@3
```

## Class Constructors

### Return the declared constructor selected by construction

An explicit constructor declaration and every construction that selects it identify the same constructor member.

```ds main.ds
class User {
    constructor(name: string) {}
    ^^^^^^^^^^^ name
}

const user = new User("Ada");
                 ^^^^ call
```

```query call_item main.ds#name
@call_item.item name=constructor kind=constructor detail="User.constructor(name: string)" location=main.ds:2:5-2:33 selection=main.ds#name symbol=main.ds#symbol@2
```

```query call_item main.ds#call
@call_item.item name=constructor kind=constructor detail="User.constructor(name: string)" location=main.ds:2:5-2:33 selection=main.ds#name symbol=main.ds#symbol@2
```

### Return the class item for a default constructor

A class without an authored constructor owns its implicit default construction item.

```ds main.ds
class User {}
^^^^^^^^^^^^^ declaration
      ^^^^ name

const user = new User();
                 ^^^^ call
```

```query call_item main.ds#name
@call_item.item name=User kind=constructor detail="User()" location=main.ds#declaration selection=main.ds#name symbol=main.ds#User@1
```

```query call_item main.ds#call
@call_item.item name=User kind=constructor detail="User()" location=main.ds#declaration selection=main.ds#name symbol=main.ds#User@1
```

## Newtype Constructors

### Return the newtype construction item

A newtype declaration and its construction identify the same nominal constructor.

```ds main.ds
newtype UserId = string;
^^^^^^^^^^^^^^^^^^^^^^^ declaration
        ^^^^^^ name

const userId = UserId("user-1");
               ^^^^^^ call
```

```query call_item main.ds#name
@call_item.item name=UserId kind=constructor detail="UserId(string): UserId" location=main.ds#declaration selection=main.ds#name symbol=main.ds#UserId@1
```

```query call_item main.ds#call
@call_item.item name=UserId kind=constructor detail="UserId(string): UserId" location=main.ds#declaration selection=main.ds#name symbol=main.ds#UserId@1
```

## Tagged Variant Constructors

### Return the generated tagged variant construction item

A tagged variant construction identifies its generated nominal constructor and its authored newtype.

```ds main.ds
@derive(Tagged)
newtype Status = Ok<string>;
^^^^^^^^^^^^^^^^^^^^^^^^^^^ declaration
        ^^^^^^ owner
                 ^^ name

const status = Status.Ok({ value: "ready" });
                      ^^ call
```

```query call_item main.ds#name
@call_item.item name=Ok kind=constructor detail="Status.Ok({ value: string }): Status" location=main.ds#declaration selection=main.ds#name symbol=main.ds#Ok@3
```

```query call_item main.ds#call
@call_item.item name=Ok kind=constructor detail="Status.Ok({ value: string }): Status" location=main.ds#declaration selection=main.ds#name symbol=main.ds#Ok@3
```

## Indirect Calls

### Return no item for a function-valued binding

A function-valued variable is not substituted for the callable identity that may flow through it.

```ds main.ds
function callee(): void {}

const callback = callee;
callback();
^^^^^^^^ call
```

```query call_item main.ds#call
@call_item.none
```

## Union Dispatch

### Return no item when a call has multiple exact targets

A union receiver can select a finite set of methods, but it does not identify one hierarchy item.

```ds main.ds
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

```query call_item main.ds#call
@call_item.none
```

## Non-callable Symbols

### Return no item for a value binding

A non-callable symbol has no call hierarchy item.

```ds main.ds
const value = 1;
      ^^^^^ value
```

```query call_item main.ds#value
@call_item.none
```

### Return no item for an enum member

An enum member is a value rather than a callable constructor.

```ds main.ds
enum Status {
    Ready,
    ^^^^^ value
}
```

```query call_item main.ds#value
@call_item.none
```
