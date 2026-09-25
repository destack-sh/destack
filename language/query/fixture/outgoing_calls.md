
## Functions

### Find direct callees

Outgoing calls identify each callee and call site in one function.

```tspp main.tspp
function target(): void {}
         ^^^^^^ target

function source(): void {
         ^^^^^^ source
    target();
    ^^^^^^^^ call
}
```

```query outgoing_calls main.tspp#source
@outgoing_calls.call index=0 name=target kind=function signature="target(): void" location=main.tspp:1:1-1:27 selection=main.tspp#target symbol=main.tspp#target@1
@outgoing_calls.site call=0 range=main.tspp#call
```

### Resolve the current outgoing call target

Outgoing calls identify the function selected after each edit.

```tspp main.tspp
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

```query outgoing_calls main.tspp#source
@outgoing_calls.call index=0 name=first kind=function signature="first(): void" location=main.tspp#declaration:first selection=main.tspp#name:first symbol=main.tspp#first@1
@outgoing_calls.site call=0 range=main.tspp#call
```

```tspp main.tspp change
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

```query outgoing_calls main.tspp#source
@outgoing_calls.call index=0 name=second kind=function signature="second(): void" location=main.tspp#declaration:second selection=main.tspp#name:second symbol=main.tspp#second@2
@outgoing_calls.site call=0 range=main.tspp#call
```

## Multiple Callees

### Find each direct callee

Distinct callees follow first-call order and include every call site.

```tspp main.tspp
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

```query outgoing_calls main.tspp#source
@outgoing_calls.call index=0 name=second kind=function signature="second(): void" location=main.tspp:2:1-2:27 selection=main.tspp#second symbol=main.tspp#second@2
@outgoing_calls.site call=0 range=main.tspp#second_call
@outgoing_calls.call index=1 name=first kind=function signature="first(): void" location=main.tspp:1:1-1:26 selection=main.tspp#first symbol=main.tspp#first@1
@outgoing_calls.site call=1 range=main.tspp#first_call
```

## Call Sites

### Return every call site for one callee

Repeated calls share one callee item and follow source order.

```tspp main.tspp
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

```query outgoing_calls main.tspp#source
@outgoing_calls.call index=0 name=target kind=function signature="target(): void" location=main.tspp#declaration selection=main.tspp#target symbol=main.tspp#target@1
@outgoing_calls.site call=0 range=main.tspp#first_call
@outgoing_calls.site call=0 range=main.tspp#second_call
```

## Modules

### Find an imported callee

Outgoing call lookup preserves the defining-module target.

```tspp library.tspp
export function target(): void {}
                ^^^^^^ target
```

```tspp main.tspp
import { target } from "./library.tspp";

function source(): void {
         ^^^^^^ source
    target();
    ^^^^^^^^ call
}
```

```query outgoing_calls main.tspp#source
@outgoing_calls.call index=0 name=target kind=function signature="target(): void" location=library.tspp:1:1-1:34 selection=library.tspp#target symbol=library.tspp#target@1
@outgoing_calls.site call=0 range=main.tspp#call
```

### Find a namespace-imported callee

Namespace member calls resolve to their exported callable.

```tspp library.tspp
export function target(): void {}
                ^^^^^^ target
```

```tspp main.tspp
import * as library from "./library.tspp";

function source(): void {
         ^^^^^^ source
    library.target();
    ^^^^^^^^^^^^^^^^ call
}
```

```query outgoing_calls main.tspp#source
@outgoing_calls.call index=0 name=target kind=function signature="target(): void" location=library.tspp:1:1-1:34 selection=library.tspp#target symbol=library.tspp#target@1
@outgoing_calls.site call=0 range=main.tspp#call
```

### Find a re-exported callee

Outgoing call lookup follows a re-export to the defining function.

```tspp library.tspp
export function target(): void {}
                ^^^^^^ target
```

```tspp public.tspp
export { target } from "./library.tspp";
```

```tspp main.tspp
import { target } from "./public.tspp";

function source(): void {
         ^^^^^^ source
    target();
    ^^^^^^^^ call
}
```

```query outgoing_calls main.tspp#source
@outgoing_calls.call index=0 name=target kind=function signature="target(): void" location=library.tspp:1:1-1:34 selection=library.tspp#target symbol=library.tspp#target@1
@outgoing_calls.site call=0 range=main.tspp#call
```

## Recursion

### Find a recursive callee

A recursive function is its own outgoing callee.

```tspp main.tspp
function recurse(): void {
         ^^^^^^^ recurse
    recurse();
    ^^^^^^^^^ call
}
```

```query outgoing_calls main.tspp#recurse
@outgoing_calls.call index=0 name=recurse kind=function signature="recurse(): void" location=main.tspp:1:1-3:2 selection=main.tspp#recurse symbol=main.tspp#recurse@1
@outgoing_calls.site call=0 range=main.tspp#call
```

## Methods

### Find a called method

Outgoing call lookup preserves the method identity.

```tspp main.tspp
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

```query outgoing_calls main.tspp#source
@outgoing_calls.call index=0 name=run kind=method signature="Service.run(): void" location=main.tspp:2:5-2:19 selection=main.tspp#target symbol=main.tspp#run@2
@outgoing_calls.site call=0 range=main.tspp#call
```

## Overloads

### Return each matching overload

Calls to overloads identify their matching declarations.

```tspp main.tspp
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

```query outgoing_calls main.tspp#caller
@outgoing_calls.call index=0 name=parse kind=function signature="parse(value: int32): int32" location=main.tspp:1:1-3:2 selection=main.tspp#integer_name symbol=main.tspp#parse@1
@outgoing_calls.site call=0 range=main.tspp#integer_call
@outgoing_calls.call index=1 name=parse kind=function signature="parse(value: string): string" location=main.tspp:5:1-7:2 selection=main.tspp#string_name symbol=main.tspp#parse@3
@outgoing_calls.site call=1 range=main.tspp#string_call
```

## Generic Functions

### Return the declared generic callee

An applied generic call identifies the generic callable declaration.

```tspp main.tspp
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

```query outgoing_calls main.tspp#caller
@outgoing_calls.call index=0 name=identity kind=function signature="identity<T>(value: T): T" location=main.tspp#declaration selection=main.tspp#name symbol=main.tspp#identity@1
@outgoing_calls.site call=0 range=main.tspp#call
```

## Extensions

### Return an extension method

An extension call identifies its extension method.

```tspp main.tspp
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

```query outgoing_calls main.tspp#caller
@outgoing_calls.call index=0 name=add kind=method signature="Calculator.add(left: int32, right: int32): int32" location=main.tspp:4:5-6:6 selection=main.tspp#name symbol=main.tspp#add@3
@outgoing_calls.site call=0 range=main.tspp#call
```

## Constructors

### Return an explicit class constructor

Construction identifies its constructor declaration.

```tspp main.tspp
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

```query outgoing_calls main.tspp#caller
@outgoing_calls.call index=0 name=constructor kind=constructor signature="User.constructor(name: string)" location=main.tspp:2:5-2:33 selection=main.tspp#name symbol=main.tspp#symbol@2
@outgoing_calls.site call=0 range=main.tspp#call
```

### Return a default class constructor

Construction of a class without a constructor declaration identifies the class item.

```tspp main.tspp
class User {}
^^^^^^^^^^^^^ declaration
      ^^^^ name

function create(): User {
         ^^^^^^ caller
    return new User();
           ^^^^^^^^^^ call
}
```

```query outgoing_calls main.tspp#caller
@outgoing_calls.call index=0 name=User kind=constructor signature="User(): User" location=main.tspp#declaration selection=main.tspp#name symbol=main.tspp#User@1
@outgoing_calls.site call=0 range=main.tspp#call
```

### Return a newtype constructor

Newtype construction identifies the nominal newtype declaration.

```tspp main.tspp
newtype UserId = string;
^^^^^^^^^^^^^^^^^^^^^^^ declaration
        ^^^^^^ name

function create(): UserId {
         ^^^^^^ caller
    return UserId("user-1");
           ^^^^^^^^^^^^^^^^ call
}
```

```query outgoing_calls main.tspp#caller
@outgoing_calls.call index=0 name=UserId kind=constructor signature="UserId(string): UserId" location=main.tspp#declaration selection=main.tspp#name symbol=main.tspp#UserId@1
@outgoing_calls.site call=0 range=main.tspp#call
```

## Caller Items

### Find calls made by a method

Outgoing lookup expands a method body through its method item.

```tspp main.tspp
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

```query outgoing_calls main.tspp#caller
@outgoing_calls.call index=0 name=target kind=function signature="target(): void" location=main.tspp:1:1-1:27 selection=main.tspp#target symbol=main.tspp#target@1
@outgoing_calls.site call=0 range=main.tspp#call
```

### Find calls made by a constructor

Outgoing lookup expands a constructor body through its constructor item.

```tspp main.tspp
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

```query outgoing_calls main.tspp#caller
@outgoing_calls.call index=0 name=target kind=function signature="target(): void" location=main.tspp:1:1-1:27 selection=main.tspp#target symbol=main.tspp#target@1
@outgoing_calls.site call=0 range=main.tspp#call
```

## Indirect Calls

### Return no declaration for an indirect call

A call through a function-valued binding has no declaration-backed callee.

```tspp main.tspp
function callee(): void {}

function caller(): void {
         ^^^^^^ caller
    const callback = callee;
    callback();
}
```

```query outgoing_calls main.tspp#caller
@outgoing_calls.none
```

## Callable Boundaries

### Do not attribute calls inside a lambda to its enclosing function

A lambda body is not part of the enclosing function's outgoing call hierarchy.

```tspp main.tspp
function target(): void {}

function outer(): void {
         ^^^^^ outer
    const callback = (): void => {
        target();
    };
}
```

```query outgoing_calls main.tspp#outer
@outgoing_calls.none
```

### Attribute calls to the nearest named function

A nested named function owns the calls in its body.

```tspp main.tspp
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

```query outgoing_calls main.tspp#outer
@outgoing_calls.none
```

```query outgoing_calls main.tspp#inner
@outgoing_calls.call index=0 name=target kind=function signature="target(): void" location=main.tspp#declaration selection=main.tspp#target symbol=main.tspp#target@1
@outgoing_calls.site call=0 range=main.tspp#call
```

## Union Dispatch

### Return every target of a union call

A union receiver contributes one outgoing edge per reachable method.

```tspp main.tspp
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

```query outgoing_calls main.tspp#caller
@outgoing_calls.call index=0 name=run kind=method signature="Alpha.run(): void" location=main.tspp:2:5-2:19 selection=main.tspp#alpha symbol=main.tspp#run@2
@outgoing_calls.site call=0 range=main.tspp#call
@outgoing_calls.call index=1 name=run kind=method signature="Beta.run(): void" location=main.tspp:6:5-6:19 selection=main.tspp#beta symbol=main.tspp#run@5
@outgoing_calls.site call=1 range=main.tspp#call
```

## Empty Results

### Return no calls for a leaf function

A leaf function has no outgoing calls.

```tspp main.tspp
function leaf(): void {}
         ^^^^ leaf
```

```query outgoing_calls main.tspp#leaf
@outgoing_calls.none
```
