
## Functions

### Find direct callees

Outgoing calls identify each callee and call site in one function.

```ds main.ds
function target(): void {}
         ^^^^^^ target

function source(): void {
         ^^^^^^ source
    target();
    ^^^^^^^^ call
}
```

```query outgoing_calls main.ds#source
@outgoing_calls.call index=0 name=target kind=function signature="target(): void" location=main.ds:1:1-1:27 selection=main.ds#target symbol=main.ds#target@1
@outgoing_calls.site call=0 range=main.ds#call
```

### Resolve the current outgoing call target

Outgoing calls identify the function selected after each edit.

```ds main.ds
function first(): void {}
^^^^^^^^^^^^^^^^^^^^^^^^^ declaration:first
         ^^^^^ name:first
function second(): void {}
^^^^^^^^^^^^^^^^^^^^^^^^^^ declaration:second
         ^^^^^^ name:second

function source(): void {
         ^^^^^^ source
    first();
    ^^^^^^^ call
}
```

```query outgoing_calls main.ds#source
@outgoing_calls.call index=0 name=first kind=function signature="first(): void" location=main.ds#declaration:first selection=main.ds#name:first symbol=main.ds#first@1
@outgoing_calls.site call=0 range=main.ds#call
```

```ds main.ds change
function first(): void {}
^^^^^^^^^^^^^^^^^^^^^^^^^ declaration:first
         ^^^^^ name:first
function second(): void {}
^^^^^^^^^^^^^^^^^^^^^^^^^^ declaration:second
         ^^^^^^ name:second

function source(): void {
         ^^^^^^ source
    second();
    ^^^^^^^^ call
}
```

```query outgoing_calls main.ds#source
@outgoing_calls.call index=0 name=second kind=function signature="second(): void" location=main.ds#declaration:second selection=main.ds#name:second symbol=main.ds#second@2
@outgoing_calls.site call=0 range=main.ds#call
```

## Multiple Callees

### Find each direct callee

Distinct callees follow first-call order and include every call site.

```ds main.ds
function first(): void {}
         ^^^^^ first
function second(): void {}
         ^^^^^^ second

function source(): void {
         ^^^^^^ source
    second();
    ^^^^^^^^ second_call
    first();
    ^^^^^^^ first_call
}
```

```query outgoing_calls main.ds#source
@outgoing_calls.call index=0 name=second kind=function signature="second(): void" location=main.ds:2:1-2:27 selection=main.ds#second symbol=main.ds#second@2
@outgoing_calls.site call=0 range=main.ds#second_call
@outgoing_calls.call index=1 name=first kind=function signature="first(): void" location=main.ds:1:1-1:26 selection=main.ds#first symbol=main.ds#first@1
@outgoing_calls.site call=1 range=main.ds#first_call
```

## Call Sites

### Return every call site for one callee

Repeated calls share one callee item and follow source order.

```ds main.ds
function target(): void {}
^^^^^^^^^^^^^^^^^^^^^^^^^^ declaration
         ^^^^^^ target

function source(): void {
         ^^^^^^ source
    target();
    ^^^^^^^^ first_call
    target();
    ^^^^^^^^ second_call
}
```

```query outgoing_calls main.ds#source
@outgoing_calls.call index=0 name=target kind=function signature="target(): void" location=main.ds#declaration selection=main.ds#target symbol=main.ds#target@1
@outgoing_calls.site call=0 range=main.ds#first_call
@outgoing_calls.site call=0 range=main.ds#second_call
```

## Modules

### Find an imported callee

Outgoing call lookup preserves the defining-module target.

```ds library.ds
export function target(): void {}
                ^^^^^^ target
```

```ds main.ds
import { target } from "./library.ds";

function source(): void {
         ^^^^^^ source
    target();
    ^^^^^^^^ call
}
```

```query outgoing_calls main.ds#source
@outgoing_calls.call index=0 name=target kind=function signature="target(): void" location=library.ds:1:1-1:34 selection=library.ds#target symbol=library.ds#target@1
@outgoing_calls.site call=0 range=main.ds#call
```

### Find a namespace-imported callee

Namespace member calls resolve to their exported callable.

```ds library.ds
export function target(): void {}
                ^^^^^^ target
```

```ds main.ds
import * as library from "./library.ds";

function source(): void {
         ^^^^^^ source
    library.target();
    ^^^^^^^^^^^^^^^^ call
}
```

```query outgoing_calls main.ds#source
@outgoing_calls.call index=0 name=target kind=function signature="target(): void" location=library.ds:1:1-1:34 selection=library.ds#target symbol=library.ds#target@1
@outgoing_calls.site call=0 range=main.ds#call
```

### Find a re-exported callee

Outgoing call lookup follows a re-export to the defining function.

```ds library.ds
export function target(): void {}
                ^^^^^^ target
```

```ds public.ds
export { target } from "./library.ds";
```

```ds main.ds
import { target } from "./public.ds";

function source(): void {
         ^^^^^^ source
    target();
    ^^^^^^^^ call
}
```

```query outgoing_calls main.ds#source
@outgoing_calls.call index=0 name=target kind=function signature="target(): void" location=library.ds:1:1-1:34 selection=library.ds#target symbol=library.ds#target@1
@outgoing_calls.site call=0 range=main.ds#call
```

## Recursion

### Find a recursive callee

A recursive function is its own outgoing callee.

```ds main.ds
function recurse(): void {
         ^^^^^^^ recurse
    recurse();
    ^^^^^^^^^ call
}
```

```query outgoing_calls main.ds#recurse
@outgoing_calls.call index=0 name=recurse kind=function signature="recurse(): void" location=main.ds:1:1-3:2 selection=main.ds#recurse symbol=main.ds#recurse@1
@outgoing_calls.site call=0 range=main.ds#call
```

## Methods

### Find a called method

Outgoing call lookup preserves the method identity.

```ds main.ds
class Service {
    run(): void {}
    ^^^ target
}

function start(service: Service): void {
         ^^^^^ source
    service.run();
    ^^^^^^^^^^^^^ call
}
```

```query outgoing_calls main.ds#source
@outgoing_calls.call index=0 name=run kind=method signature="Service.run(): void" location=main.ds:2:5-2:19 selection=main.ds#target symbol=main.ds#run@2
@outgoing_calls.site call=0 range=main.ds#call
```

## Overloads

### Return each matching overload

Calls to overloads identify their matching declarations.

```ds main.ds
function parse(value: int32): int32 {
         ^^^^^ integer_name
    return value;
}

function parse(value: string): string {
         ^^^^^ string_name
    return value;
}

function caller(): void {
         ^^^^^^ caller
    parse(1);
    ^^^^^^^^ integer_call
    parse("one");
    ^^^^^^^^^^^^ string_call
}
```

```query outgoing_calls main.ds#caller
@outgoing_calls.call index=0 name=parse kind=function signature="parse(value: int32): int32" location=main.ds:1:1-3:2 selection=main.ds#integer_name symbol=main.ds#parse@1
@outgoing_calls.site call=0 range=main.ds#integer_call
@outgoing_calls.call index=1 name=parse kind=function signature="parse(value: string): string" location=main.ds:5:1-7:2 selection=main.ds#string_name symbol=main.ds#parse@3
@outgoing_calls.site call=1 range=main.ds#string_call
```

## Generic Functions

### Return the declared generic callee

An applied generic call identifies the generic callable declaration.

```ds main.ds
function identity<T>(value: T): T {
^ declaration:start
         ^^^^^^^^ name
    return value;
}
^ declaration:end

function caller(): string {
         ^^^^^^ caller
    return identity<string>("value");
           ^^^^^^^^^^^^^^^^^^^^^^^^^ call
}
```

```query outgoing_calls main.ds#caller
@outgoing_calls.call index=0 name=identity kind=function signature="identity<T>(value: T): T" location=main.ds#declaration selection=main.ds#name symbol=main.ds#identity@1
@outgoing_calls.site call=0 range=main.ds#call
```

## Extensions

### Return an extension method

An extension call identifies its extension method.

```ds main.ds
struct Calculator {}

extension of Calculator {
    add(left: int32, right: int32): int32 {
    ^^^ name
        return left + right;
    }
}

function caller(calculator: Calculator): int32 {
         ^^^^^^ caller
    return calculator.add(1, 2);
           ^^^^^^^^^^^^^^^^^^^^ call
}
```

```query outgoing_calls main.ds#caller
@outgoing_calls.call index=0 name=add kind=method signature="Calculator.add(left: int32, right: int32): int32" location=main.ds:4:5-6:6 selection=main.ds#name symbol=main.ds#add@3
@outgoing_calls.site call=0 range=main.ds#call
```

## Constructors

### Return an explicit class constructor

Construction identifies its constructor declaration.

```ds main.ds
class User {
    constructor(name: string) {}
    ^^^^^^^^^^^ name
}

function create(): User {
         ^^^^^^ caller
    return new User("Ada");
           ^^^^^^^^^^^^^^^ call
}
```

```query outgoing_calls main.ds#caller
@outgoing_calls.call index=0 name=constructor kind=constructor signature="User.constructor(name: string)" location=main.ds:2:5-2:33 selection=main.ds#name symbol=main.ds#symbol@2
@outgoing_calls.site call=0 range=main.ds#call
```

### Return a default class constructor

Construction of a class without a constructor declaration identifies the class item.

```ds main.ds
class User {}
^^^^^^^^^^^^^ declaration
      ^^^^ name

function create(): User {
         ^^^^^^ caller
    return new User();
           ^^^^^^^^^^ call
}
```

```query outgoing_calls main.ds#caller
@outgoing_calls.call index=0 name=User kind=constructor signature="User(): User" location=main.ds#declaration selection=main.ds#name symbol=main.ds#User@1
@outgoing_calls.site call=0 range=main.ds#call
```

### Return a newtype constructor

Newtype construction identifies the nominal newtype declaration.

```ds main.ds
newtype UserId = string;
^^^^^^^^^^^^^^^^^^^^^^^ declaration
        ^^^^^^ name

function create(): UserId {
         ^^^^^^ caller
    return UserId("user-1");
           ^^^^^^^^^^^^^^^^ call
}
```

```query outgoing_calls main.ds#caller
@outgoing_calls.call index=0 name=UserId kind=constructor signature="UserId(string): UserId" location=main.ds#declaration selection=main.ds#name symbol=main.ds#UserId@1
@outgoing_calls.site call=0 range=main.ds#call
```

## Caller Items

### Find calls made by a method

Outgoing lookup expands a method body through its method item.

```ds main.ds
function target(): void {}
         ^^^^^^ target

class Service {
    run(): void {
    ^^^ caller
        target();
        ^^^^^^^^ call
    }
}
```

```query outgoing_calls main.ds#caller
@outgoing_calls.call index=0 name=target kind=function signature="target(): void" location=main.ds:1:1-1:27 selection=main.ds#target symbol=main.ds#target@1
@outgoing_calls.site call=0 range=main.ds#call
```

### Find calls made by a constructor

Outgoing lookup expands a constructor body through its constructor item.

```ds main.ds
function target(): void {}
         ^^^^^^ target

class Service {
    constructor() {
    ^^^^^^^^^^^ caller
        target();
        ^^^^^^^^ call
    }
}
```

```query outgoing_calls main.ds#caller
@outgoing_calls.call index=0 name=target kind=function signature="target(): void" location=main.ds:1:1-1:27 selection=main.ds#target symbol=main.ds#target@1
@outgoing_calls.site call=0 range=main.ds#call
```

## Indirect Calls

### Return no declaration for an indirect call

A call through a function-valued binding has no declaration-backed callee.

```ds main.ds
function callee(): void {}

function caller(): void {
         ^^^^^^ caller
    const callback = callee;
    callback();
}
```

```query outgoing_calls main.ds#caller
@outgoing_calls.none
```

## Callable Boundaries

### Do not attribute calls inside a lambda to its enclosing function

A lambda body is not part of the enclosing function's outgoing call hierarchy.

```ds main.ds
function target(): void {}

function outer(): void {
         ^^^^^ outer
    const callback = (): void => {
        target();
    };
}
```

```query outgoing_calls main.ds#outer
@outgoing_calls.none
```

### Attribute calls to the nearest named function

A nested named function owns the calls in its body.

```ds main.ds
function target(): void {}
^^^^^^^^^^^^^^^^^^^^^^^^^^ declaration
         ^^^^^^ target

function outer(): void {
         ^^^^^ outer
    function inner(): void {
             ^^^^^ inner
        target();
        ^^^^^^^^ call
    }
}
```

```query outgoing_calls main.ds#outer
@outgoing_calls.none
```

```query outgoing_calls main.ds#inner
@outgoing_calls.call index=0 name=target kind=function signature="target(): void" location=main.ds#declaration selection=main.ds#target symbol=main.ds#target@1
@outgoing_calls.site call=0 range=main.ds#call
```

## Union Dispatch

### Return every target of a union call

A union receiver contributes one outgoing edge per reachable method.

```ds main.ds
class Alpha {
    run(): void {}
    ^^^ alpha
}

class Beta {
    run(): void {}
    ^^^ beta
}

function start(service: Alpha | Beta): void {
         ^^^^^ caller
    service.run();
    ^^^^^^^^^^^^^ call
}
```

```query outgoing_calls main.ds#caller
@outgoing_calls.call index=0 name=run kind=method signature="Alpha.run(): void" location=main.ds:2:5-2:19 selection=main.ds#alpha symbol=main.ds#run@2
@outgoing_calls.site call=0 range=main.ds#call
@outgoing_calls.call index=1 name=run kind=method signature="Beta.run(): void" location=main.ds:6:5-6:19 selection=main.ds#beta symbol=main.ds#run@5
@outgoing_calls.site call=1 range=main.ds#call
```

## Empty Results

### Return no calls for a leaf function

A leaf function has no outgoing calls.

```ds main.ds
function leaf(): void {}
         ^^^^ leaf
```

```query outgoing_calls main.ds#leaf
@outgoing_calls.none
```
