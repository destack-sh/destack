
## Functions

### Find direct callers

Incoming calls identify the caller and each call site.

```ds main.ds
function callee(): void {}
         ^^^^^^ callee

function caller(): void {
         ^^^^^^ caller
    callee();
    ^^^^^^^^ call
}
```

```query incoming_calls main.ds#callee
@incoming_calls.call index=0 name=caller kind=function signature="caller(): void" location=main.ds:3:1-5:2 selection=main.ds#caller symbol=main.ds#caller@2
@incoming_calls.site call=0 range=main.ds#call
```

## Call Sites

### Return every call site in one caller

Repeated calls share one caller item and follow source order.

```ds main.ds
function callee(): void {}
         ^^^^^^ callee

function caller(): void {
         ^^^^^^ caller
    callee();
    ^^^^^^^^ first_call
    callee();
    ^^^^^^^^ second_call
}
```

```query incoming_calls main.ds#callee
@incoming_calls.call index=0 name=caller kind=function signature="caller(): void" location=main.ds:3:1-6:2 selection=main.ds#caller symbol=main.ds#caller@2
@incoming_calls.site call=0 range=main.ds#first_call
@incoming_calls.site call=0 range=main.ds#second_call
```

## Callers

### Find distinct callers

Distinct callers follow source order and keep their own call sites.

```ds main.ds
function callee(): void {}
         ^^^^^^ callee

function first(): void {
         ^^^^^ first
    callee();
    ^^^^^^^^ first_call
}

function second(): void {
         ^^^^^^ second
    callee();
    ^^^^^^^^ second_call
}
```

```query incoming_calls main.ds#callee
@incoming_calls.call index=0 name=first kind=function signature="first(): void" location=main.ds:3:1-5:2 selection=main.ds#first symbol=main.ds#first@2
@incoming_calls.site call=0 range=main.ds#first_call
@incoming_calls.call index=1 name=second kind=function signature="second(): void" location=main.ds:7:1-9:2 selection=main.ds#second symbol=main.ds#second@3
@incoming_calls.site call=1 range=main.ds#second_call
```

### Return current incoming calls

Incoming calls include call sites added in later revisions.

```ds main.ds
function callee(): void {}
         ^^^^^^ callee

function caller(): void {
         ^^^^^^ caller
}
```

```query incoming_calls main.ds#callee
@incoming_calls.none
```

```ds main.ds change
function callee(): void {}
         ^^^^^^ callee

function caller(): void {
^ declaration:caller:start
         ^^^^^^ caller
    callee();
    ^^^^^^^^ call
}
^ declaration:caller:end
```

```query incoming_calls main.ds#callee
@incoming_calls.call index=0 name=caller kind=function signature="caller(): void" location=main.ds#declaration:caller selection=main.ds#caller symbol=main.ds#caller@2
@incoming_calls.site call=0 range=main.ds#call
```

## Modules

### Find a caller in another module

Incoming call lookup follows the imported function declaration.

```ds library.ds
export function callee(): void {}
                ^^^^^^ callee
```

```ds main.ds
import { callee } from "./library.ds";

function caller(): void {
         ^^^^^^ caller
    callee();
    ^^^^^^^^ call
}
```

```query incoming_calls library.ds#callee
@incoming_calls.call index=0 name=caller kind=function signature="caller(): void" location=main.ds:3:1-5:2 selection=main.ds#caller symbol=main.ds#caller@2
@incoming_calls.site call=0 range=main.ds#call
```

### Find a caller through a re-exported callee

Incoming call lookup follows the function declaration through re-exports.

```ds library.ds
export function callee(): void {}
                ^^^^^^ callee
```

```ds public.ds
export { callee } from "./library.ds";
```

```ds main.ds
import { callee } from "./public.ds";

function caller(): void {
         ^^^^^^ caller
    callee();
    ^^^^^^^^ call
}
```

```query incoming_calls library.ds#callee
@incoming_calls.call index=0 name=caller kind=function signature="caller(): void" location=main.ds:3:1-5:2 selection=main.ds#caller symbol=main.ds#caller@2
@incoming_calls.site call=0 range=main.ds#call
```

## Recursion

### Find a recursive caller

A recursive function is its own incoming caller.

```ds main.ds
function recurse(): void {
         ^^^^^^^ recurse
    recurse();
    ^^^^^^^^^ call
}
```

```query incoming_calls main.ds#recurse
@incoming_calls.call index=0 name=recurse kind=function signature="recurse(): void" location=main.ds:1:1-3:2 selection=main.ds#recurse symbol=main.ds#recurse@1
@incoming_calls.site call=0 range=main.ds#call
```

## Methods

### Find callers of a method

Incoming call lookup preserves the method identity.

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

```query incoming_calls main.ds#target
@incoming_calls.call index=0 name=start kind=function signature="start(service: Service): void" location=main.ds:5:1-7:2 selection=main.ds#source symbol=main.ds#start@4
@incoming_calls.site call=0 range=main.ds#call
```

## Overloads

### Keep calls separated by overload

Each overload receives only the calls that match its declaration.

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

```query incoming_calls main.ds#integer_call
@incoming_calls.call index=0 name=caller kind=function signature="caller(): void" location=main.ds:9:1-12:2 selection=main.ds#caller symbol=main.ds#caller@5
@incoming_calls.site call=0 range=main.ds#integer_call
```

```query incoming_calls main.ds#string_call
@incoming_calls.call index=0 name=caller kind=function signature="caller(): void" location=main.ds:9:1-12:2 selection=main.ds#caller symbol=main.ds#caller@5
@incoming_calls.site call=0 range=main.ds#string_call
```

## Extensions

### Find callers of an extension method

An extension call is attributed to its extension method.

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

```query incoming_calls main.ds#name
@incoming_calls.call index=0 name=caller kind=function signature="caller(calculator: Calculator): int32" location=main.ds:9:1-11:2 selection=main.ds#caller symbol=main.ds#caller@7
@incoming_calls.site call=0 range=main.ds#call
```

## Constructors

### Find callers of an explicit class constructor

Construction is attributed to its constructor declaration.

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

```query incoming_calls main.ds#name
@incoming_calls.call index=0 name=create kind=function signature="create(): User" location=main.ds:5:1-7:2 selection=main.ds#caller symbol=main.ds#create@5
@incoming_calls.site call=0 range=main.ds#call
```

### Find callers of a default class constructor

A class without a constructor declaration receives construction calls through its class item.

```ds main.ds
class User {}
      ^^^^ name

function create(): User {
         ^^^^^^ caller
    return new User();
           ^^^^^^^^^^ call
}
```

```query incoming_calls main.ds#name
@incoming_calls.call index=0 name=create kind=function signature="create(): User" location=main.ds:3:1-5:2 selection=main.ds#caller symbol=main.ds#create@2
@incoming_calls.site call=0 range=main.ds#call
```

### Find callers of a newtype constructor

Newtype construction is attributed to the nominal newtype declaration.

```ds main.ds
newtype UserId = string;
        ^^^^^^ name

function create(): UserId {
         ^^^^^^ caller
    return UserId("user-1");
           ^^^^^^^^^^^^^^^^ call
}
```

```query incoming_calls main.ds#name
@incoming_calls.call index=0 name=create kind=function signature="create(): UserId" location=main.ds:3:1-5:2 selection=main.ds#caller symbol=main.ds#create@2
@incoming_calls.site call=0 range=main.ds#call
```

## Caller Items

### Return a method as the caller

A call inside a method is attributed to that method rather than its class.

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

```query incoming_calls main.ds#target
@incoming_calls.call index=0 name=run kind=method signature="Service.run(): void" location=main.ds:4:5-6:6 selection=main.ds#caller symbol=main.ds#run@3
@incoming_calls.site call=0 range=main.ds#call
```

### Return a constructor as the caller

A call inside a constructor is attributed to that constructor.

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

```query incoming_calls main.ds#target
@incoming_calls.call index=0 name=constructor kind=constructor signature="Service.constructor()" location=main.ds:4:5-6:6 selection=main.ds#caller symbol=main.ds#symbol@3
@incoming_calls.site call=0 range=main.ds#call
```

## Indirect Calls

### Do not attribute calls through function-valued bindings

The declaration assigned to a function-valued binding is not the call target.

```ds main.ds
function callee(): void {}
         ^^^^^^ callee

function caller(): void {
    const callback = callee;
    callback();
}
```

```query incoming_calls main.ds#callee
@incoming_calls.none
```

## Anonymous Callers

### Do not attribute calls inside a lambda to its enclosing function

A lambda is the nearest callable boundary but has no named hierarchy item.

```ds main.ds
function target(): void {}
         ^^^^^^ target

function outer(): void {
    const callback = (): void => {
        target();
    };
}
```

```query incoming_calls main.ds#target
@incoming_calls.none
```

### Do not return a module-level call as a named caller

A call outside a callable item has no incoming hierarchy item.

```ds main.ds
function target(): void {}
         ^^^^^^ target

target();
```

```query incoming_calls main.ds#target
@incoming_calls.none
```

## Union Dispatch

### Attribute a shared call site to every target

Every method reached through a union receiver receives the shared caller and source range.

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

```query incoming_calls main.ds#alpha
@incoming_calls.call index=0 name=start kind=function signature="start(service: Alpha | Beta): void" location=main.ds:9:1-11:2 selection=main.ds#caller symbol=main.ds#start@7
@incoming_calls.site call=0 range=main.ds#call
```

```query incoming_calls main.ds#beta
@incoming_calls.call index=0 name=start kind=function signature="start(service: Alpha | Beta): void" location=main.ds:9:1-11:2 selection=main.ds#caller symbol=main.ds#start@7
@incoming_calls.site call=0 range=main.ds#call
```

## Empty Results

### Return no calls for an uncalled function

An uncalled function has no incoming calls.

```ds main.ds
function idle(): void {}
         ^^^^ idle
```

```query incoming_calls main.ds#idle
@incoming_calls.none
```
