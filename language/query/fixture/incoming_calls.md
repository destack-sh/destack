
## Functions

### Find direct callers

Incoming calls identify the caller and each call site.

```tspp main.tspp
function callee(): void {}
         ^^^^^^ callee

function caller(): void {
         ^^^^^^ caller
    callee();
    ^^^^^^^^ call
}
```

```query incoming_calls main.tspp#callee
@incoming_calls.call index=0 name=caller kind=function signature="caller(): void" location=main.tspp:3:1-5:2 selection=main.tspp#caller symbol=main.tspp#caller@2
@incoming_calls.site call=0 range=main.tspp#call
```

## Call Sites

### Return every call site in one caller

Repeated calls share one caller item and follow source order.

```tspp main.tspp
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

```query incoming_calls main.tspp#callee
@incoming_calls.call index=0 name=caller kind=function signature="caller(): void" location=main.tspp:3:1-6:2 selection=main.tspp#caller symbol=main.tspp#caller@2
@incoming_calls.site call=0 range=main.tspp#first_call
@incoming_calls.site call=0 range=main.tspp#second_call
```

## Callers

### Find distinct callers

Distinct callers follow source order and keep their own call sites.

```tspp main.tspp
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

```query incoming_calls main.tspp#callee
@incoming_calls.call index=0 name=first kind=function signature="first(): void" location=main.tspp:3:1-5:2 selection=main.tspp#first symbol=main.tspp#first@2
@incoming_calls.site call=0 range=main.tspp#first_call
@incoming_calls.call index=1 name=second kind=function signature="second(): void" location=main.tspp:7:1-9:2 selection=main.tspp#second symbol=main.tspp#second@3
@incoming_calls.site call=1 range=main.tspp#second_call
```

### Return current incoming calls

Incoming calls include call sites added by later edits.

```tspp main.tspp
function callee(): void {}
         ^^^^^^ callee

function caller(): void {
         ^^^^^^ caller
}
```

```query incoming_calls main.tspp#callee
@incoming_calls.none
```

```tspp main.tspp change
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

```query incoming_calls main.tspp#callee
@incoming_calls.call index=0 name=caller kind=function signature="caller(): void" location=main.tspp#declaration:caller selection=main.tspp#caller symbol=main.tspp#caller@2
@incoming_calls.site call=0 range=main.tspp#call
```

## Modules

### Find a caller in another module

Incoming call lookup follows the imported function declaration.

```tspp library.tspp
export function callee(): void {}
                ^^^^^^ callee
```

```tspp main.tspp
import { callee } from "./library.tspp";

function caller(): void {
         ^^^^^^ caller
    callee();
    ^^^^^^^^ call
}
```

```query incoming_calls library.tspp#callee
@incoming_calls.call index=0 name=caller kind=function signature="caller(): void" location=main.tspp:3:1-5:2 selection=main.tspp#caller symbol=main.tspp#caller@2
@incoming_calls.site call=0 range=main.tspp#call
```

### Find a caller through a re-exported callee

Incoming call lookup follows the function declaration through re-exports.

```tspp library.tspp
export function callee(): void {}
                ^^^^^^ callee
```

```tspp public.tspp
export { callee } from "./library.tspp";
```

```tspp main.tspp
import { callee } from "./public.tspp";

function caller(): void {
         ^^^^^^ caller
    callee();
    ^^^^^^^^ call
}
```

```query incoming_calls library.tspp#callee
@incoming_calls.call index=0 name=caller kind=function signature="caller(): void" location=main.tspp:3:1-5:2 selection=main.tspp#caller symbol=main.tspp#caller@2
@incoming_calls.site call=0 range=main.tspp#call
```

## Recursion

### Find a recursive caller

A recursive function is its own incoming caller.

```tspp main.tspp
function recurse(): void {
         ^^^^^^^ recurse
    recurse();
    ^^^^^^^^^ call
}
```

```query incoming_calls main.tspp#recurse
@incoming_calls.call index=0 name=recurse kind=function signature="recurse(): void" location=main.tspp:1:1-3:2 selection=main.tspp#recurse symbol=main.tspp#recurse@1
@incoming_calls.site call=0 range=main.tspp#call
```

## Methods

### Find callers of a method

Incoming call lookup preserves the method identity.

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

```query incoming_calls main.tspp#target
@incoming_calls.call index=0 name=start kind=function signature="start(service: Service): void" location=main.tspp:5:1-7:2 selection=main.tspp#source symbol=main.tspp#start@4
@incoming_calls.site call=0 range=main.tspp#call
```

## Overloads

### Keep calls separated by overload

Each overload receives only the calls that match its declaration.

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

```query incoming_calls main.tspp#integer_call
@incoming_calls.call index=0 name=caller kind=function signature="caller(): void" location=main.tspp:9:1-12:2 selection=main.tspp#caller symbol=main.tspp#caller@5
@incoming_calls.site call=0 range=main.tspp#integer_call
```

```query incoming_calls main.tspp#string_call
@incoming_calls.call index=0 name=caller kind=function signature="caller(): void" location=main.tspp:9:1-12:2 selection=main.tspp#caller symbol=main.tspp#caller@5
@incoming_calls.site call=0 range=main.tspp#string_call
```

## Extensions

### Find callers of an extension method

An extension call is attributed to its extension method.

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

```query incoming_calls main.tspp#name
@incoming_calls.call index=0 name=caller kind=function signature="caller(calculator: Calculator): int32" location=main.tspp:9:1-11:2 selection=main.tspp#caller symbol=main.tspp#caller@7
@incoming_calls.site call=0 range=main.tspp#call
```

## Constructors

### Find callers of an explicit class constructor

Construction is attributed to its constructor declaration.

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

```query incoming_calls main.tspp#name
@incoming_calls.call index=0 name=create kind=function signature="create(): User" location=main.tspp:5:1-7:2 selection=main.tspp#caller symbol=main.tspp#create@5
@incoming_calls.site call=0 range=main.tspp#call
```

### Find callers of a default class constructor

A class without a constructor declaration receives construction calls through its class item.

```tspp main.tspp
class User {}
      ^^^^ name

function create(): User {
         ^^^^^^ caller
    return new User();
           ^^^^^^^^^^ call
}
```

```query incoming_calls main.tspp#name
@incoming_calls.call index=0 name=create kind=function signature="create(): User" location=main.tspp:3:1-5:2 selection=main.tspp#caller symbol=main.tspp#create@2
@incoming_calls.site call=0 range=main.tspp#call
```

### Find callers of a newtype constructor

Newtype construction is attributed to the nominal newtype declaration.

```tspp main.tspp
newtype UserId = string;
        ^^^^^^ name

function create(): UserId {
         ^^^^^^ caller
    return UserId("user-1");
           ^^^^^^^^^^^^^^^^ call
}
```

```query incoming_calls main.tspp#name
@incoming_calls.call index=0 name=create kind=function signature="create(): UserId" location=main.tspp:3:1-5:2 selection=main.tspp#caller symbol=main.tspp#create@2
@incoming_calls.site call=0 range=main.tspp#call
```

## Caller Items

### Return a method as the caller

A call inside a method is attributed to that method rather than its class.

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

```query incoming_calls main.tspp#target
@incoming_calls.call index=0 name=run kind=method signature="Service.run(): void" location=main.tspp:4:5-6:6 selection=main.tspp#caller symbol=main.tspp#run@3
@incoming_calls.site call=0 range=main.tspp#call
```

### Return a constructor as the caller

A call inside a constructor is attributed to that constructor.

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

```query incoming_calls main.tspp#target
@incoming_calls.call index=0 name=constructor kind=constructor signature="Service.constructor()" location=main.tspp:4:5-6:6 selection=main.tspp#caller symbol=main.tspp#symbol@3
@incoming_calls.site call=0 range=main.tspp#call
```

## Indirect Calls

### Do not attribute calls through function-valued bindings

The declaration assigned to a function-valued binding is not the call target.

```tspp main.tspp
function callee(): void {}
         ^^^^^^ callee

function caller(): void {
    const callback = callee;
    callback();
}
```

```query incoming_calls main.tspp#callee
@incoming_calls.none
```

## Anonymous Callers

### Do not attribute calls inside a lambda to its enclosing function

A lambda is the nearest callable boundary but has no named hierarchy item.

```tspp main.tspp
function target(): void {}
         ^^^^^^ target

function outer(): void {
    const callback = (): void => {
        target();
    };
}
```

```query incoming_calls main.tspp#target
@incoming_calls.none
```

### Do not return a module-level call as a named caller

A call outside a callable item has no incoming hierarchy item.

```tspp main.tspp
function target(): void {}
         ^^^^^^ target

target();
```

```query incoming_calls main.tspp#target
@incoming_calls.none
```

## Union Dispatch

### Attribute a shared call site to every target

Every method reached through a union receiver receives the shared caller and source range.

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

```query incoming_calls main.tspp#alpha
@incoming_calls.call index=0 name=start kind=function signature="start(service: Alpha | Beta): void" location=main.tspp:9:1-11:2 selection=main.tspp#caller symbol=main.tspp#start@7
@incoming_calls.site call=0 range=main.tspp#call
```

```query incoming_calls main.tspp#beta
@incoming_calls.call index=0 name=start kind=function signature="start(service: Alpha | Beta): void" location=main.tspp:9:1-11:2 selection=main.tspp#caller symbol=main.tspp#start@7
@incoming_calls.site call=0 range=main.tspp#call
```

## Empty Results

### Return no calls for an uncalled function

An uncalled function has no incoming calls.

```tspp main.tspp
function idle(): void {}
         ^^^^ idle
```

```query incoming_calls main.tspp#idle
@incoming_calls.none
```
