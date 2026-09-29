---
title: Classes
description: Reference types, the way TS writes them.
---

# Classes

Classes work like they do in TS: fields and methods, construction with `new`, and every variable holding an instance points at the same object.

```tspp
class Counter {
    value = 0;

    increment(): void {
        this.value += 1;
    }
}

const counter = new Counter();
const alias = counter;
alias.increment();
counter.value // => 1
```

A class value is a handle to one object, shared by everyone who holds it.
When you want a value that copies instead, that's a [struct](/docs/language/typescriptpp/structs/).

## Constructors

Fields get their values from initializers or the constructor, and every field must have one on every path through the constructor.

```tspp
class User {
    name: string;
    visits: int32;
//  ^^^^^^ error[field-not-definitely-initialized]: field 'visits' is not initialized on every constructor path

    constructor(name: string) {
        this.name = name;
    }
}
```

## Inheritance

Classes extend one base class.
Methods are final unless the base marks them `virtual`, and a subclass says `override` when it replaces one, so every override is intentional.

```tspp
class Shape {
    virtual area(): float64 {
        0.0
    }
}

class Square extends Shape {
    side: float64 = 2.0;

    override area(): float64 {
        this.side * this.side
    }
}

class Circle extends Shape {
    override perimeter(): float64 {
//           ^^^^^^^^^ error[invalid-override]: 'perimeter' does not override an inherited member
        0.0
    }
}
```

## Methods Are Not Values

Reading a method off an object doesn't produce a bound function, so `this` never goes missing.
Wrap the call in a closure instead.

```tspp
class Greeter {
    greet(): string {
        "hello"
    }
}

const greeter = new Greeter();
const bound = () => greeter.greet();
const detached = greeter.greet;
//                       ^^^^^ error[cannot-extract-bound-method]: method 'greet' cannot be read as a value
```

## Visibility

Members are public by default, as in TS.
`private` and `protected` apply per module: code in the declaring module sees everything, other modules only see what's public.

```tspp src/account.tspp
export class Account {
    private balance: int64 = 0;

    deposit(amount: int64): void {
        this.balance += amount;
    }
}

export function audit(account: Account): int64 {
    account.balance
}
```

```tspp src/main.tspp
import { Account } from "./account.tspp";

const account = new Account();
account.deposit(10);
account.balance;
//      ^^^^^^^ error[inaccessible-member]: member 'balance' is private
```
