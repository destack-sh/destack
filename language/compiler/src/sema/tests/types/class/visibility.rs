use crate::tests::{DirRows, TestSession};

/// A private member admits reads from its declaring module, including extensions.
#[test]
fn test_private_member_admits_its_declaring_module() {
    let session = TestSession::single(
        r#"
class Account {
    private balance: int32 = 0;

    total(&readonly this): int32 {
        return this.balance;
    }
}

extension of Account {
    audit(&readonly this): int32 {
        return this.balance;
    }
}

declare const account: Account;
const read = account.balance;
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked().with_definitions(), r#"
=== annotated ===
class Account {
    private balance: int32 = 0;

    total(&readonly this): int32 {
        return this.balance;
    }
}

extension of Account {
    audit(&readonly this): int32 {
        return this.balance;
    }
}

declare const account: Account;
const read: int32 = account.balance;

=== dir ===
class Account {
/// @type.symbol symbol=Account type=Account
/// @definition.class symbol=Account
/// @definition.field symbol=Account.balance source="private balance: int32 = 0" key=balance visibility=private type=int32
/// @definition.method symbol=Account.total slot=total type=<Account.total.'a>(this: &Account.total.'a readonly this) => int32

    private balance: int32 = 0;
    /// @type.symbol symbol=Account.balance source="private balance: int32 = 0" type=int32

    total(&readonly this): int32 {
    /// @generic.template symbol=Account.total parameters=('a)
    /// @type.symbol symbol=Account.total type=<Account.total.'a>(this: &Account.total.'a readonly this) => int32
    /// @type.symbol symbol=Account.total.this source="&readonly this" type=&Account.total.'a readonly this

        return this.balance;
        /// @resolution.member source=this.balance receiver=&Account.total.'a readonly Account type=int32 kind=field target_receiver=&Account.total.'a readonly Account key=balance target=Account.balance target_type=int32
        /// @resolution.receiver source=this kind=this declaration=Account type=&Account.total.'a readonly Account
        /// @resolution.place source=this placement=Account.total.'a lifetime=Account.total.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.balance placement=Account.total.'a lifetime=Account.total.'a access="readonly"
        /// @resolution.access source=this.balance root=this keys=[balance]

    }
}

extension of Account {
/// @definition.extension symbol=<module>#2 form=local target=Account
/// @definition.method symbol=audit slot=audit type=<audit.'a>(this: &audit.'a readonly this) => int32
/// @resolution.name source=Account target=Account

    audit(&readonly this): int32 {
    /// @generic.template symbol=audit parameters=('a)
    /// @type.symbol symbol=audit type=<audit.'a>(this: &audit.'a readonly this) => int32
    /// @type.symbol symbol=audit.this source="&readonly this" type=&audit.'a readonly this

        return this.balance;
        /// @resolution.member source=this.balance receiver=&audit.'a readonly Account type=int32 kind=field target_receiver=&audit.'a readonly Account key=balance target=Account.balance target_type=int32
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&audit.'a readonly Account
        /// @resolution.place source=this placement=audit.'a lifetime=audit.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.balance placement=audit.'a lifetime=audit.'a access="readonly"
        /// @resolution.access source=this.balance root=this keys=[balance]

    }
}

declare const account: Account;
/// @type.symbol symbol=account source=account type=Account
/// @resolution.pattern source=account kind=binding target=account
/// @resolution.name source=Account target=Account

const read = account.balance;
/// @type.symbol symbol=read source=read type=int32
/// @resolution.pattern source=read kind=binding target=read
/// @resolution.name source=account target=account
/// @resolution.member source=account.balance receiver=Account type=int32 kind=field target_receiver=Account key=balance target=Account.balance target_type=int32
/// @resolution.place source=account placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=account root=account
/// @resolution.access source=account.balance root=account keys=[balance]
"#, r#""#);
}

/// A private member rejects reads from another module.
#[test]
fn test_private_member_rejects_a_foreign_module() {
    let session = TestSession::builder()
        .module(
            "account.ds",
            r#"
export class Account {
    private balance: int32 = 0;
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Account } from "./account.ds";

declare const account: Account;
const read = account.balance;
"#,
        )
        .build();

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
import { Account } from "./account.ds";

declare const account: Account;
const read: int32 = account.balance;

=== dir ===
import { Account } from "./account.ds";

declare const account: Account;
/// @type.symbol symbol=account source=account type=account.Account
/// @resolution.pattern source=account kind=binding target=account
/// @resolution.name source=Account target=account.Account

const read = account.balance;
/// @type.symbol symbol=read source=read type=int32
/// @resolution.pattern source=read kind=binding target=read
/// @resolution.name source=account target=account
/// @resolution.member source=account.balance receiver=account.Account type=int32 kind=field target_receiver=account.Account key=balance target=account.Account.balance target_type=int32
/// @resolution.place source=account placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=account root=account
/// @resolution.access source=account.balance root=account keys=[balance]
"#, r#"
/// @diagnostic.error id=inaccessible-member message="member 'balance' is private"
/// @diagnostic.label line=5 column=22 span="balance" line_source="const read = account.balance;"
"#);
}

/// A protected member admits derived classes and rejects unrelated sites.
#[test]
fn test_protected_member_admits_derived_classes_only() {
    let session = TestSession::builder()
        .module(
            "base.ds",
            r#"
export class Shape {
    protected area: int32 = 0;
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Shape } from "./base.ds";

class Circle extends Shape {
    measure(&readonly this): int32 {
        return this.area;
    }
}

declare const shape: Shape;
const read = shape.area;
"#,
        )
        .build();

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
import { Shape } from "./base.ds";

class Circle extends Shape {
    measure(&readonly this): int32 {
        return this.area;
    }
}

declare const shape: Shape;
const read: int32 = shape.area;

=== dir ===
import { Shape } from "./base.ds";

class Circle extends Shape {
/// @type.symbol symbol=Circle type=Circle
/// @definition.class symbol=Circle
/// @definition.extends symbol=Circle source=Shape target=base.Shape
/// @definition.method symbol=Circle.measure slot=measure type=<Circle.measure.'a>(this: &Circle.measure.'a readonly this) => int32
/// @resolution.name source=Shape target=base.Shape

    measure(&readonly this): int32 {
    /// @generic.template symbol=Circle.measure parameters=('a)
    /// @type.symbol symbol=Circle.measure type=<Circle.measure.'a>(this: &Circle.measure.'a readonly this) => int32
    /// @type.symbol symbol=Circle.measure.this source="&readonly this" type=&Circle.measure.'a readonly this

        return this.area;
        /// @resolution.member source=this.area receiver=&Circle.measure.'a readonly Circle type=int32 kind=field target_receiver=&Circle.measure.'a readonly Circle key=area target=base.Shape.area target_type=int32
        /// @resolution.receiver source=this kind=this declaration=Circle type=&Circle.measure.'a readonly Circle
        /// @resolution.place source=this placement=Circle.measure.'a lifetime=Circle.measure.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.area placement=Circle.measure.'a lifetime=Circle.measure.'a access="readonly"
        /// @resolution.access source=this.area root=this keys=[area]

    }
}

declare const shape: Shape;
/// @type.symbol symbol=shape source=shape type=base.Shape
/// @resolution.pattern source=shape kind=binding target=shape
/// @resolution.name source=Shape target=base.Shape

const read = shape.area;
/// @type.symbol symbol=read source=read type=int32
/// @resolution.pattern source=read kind=binding target=read
/// @resolution.name source=shape target=shape
/// @resolution.member source=shape.area receiver=base.Shape type=int32 kind=field target_receiver=base.Shape key=area target=base.Shape.area target_type=int32
/// @resolution.place source=shape placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=shape root=shape
/// @resolution.access source=shape.area root=shape keys=[area]
"#, r#"
/// @diagnostic.error id=inaccessible-member message="member 'area' is protected"
/// @diagnostic.label line=11 column=20 span="area" line_source="const read = shape.area;"
"#);
}

/// A private field admits writes from its declaring module.
#[test]
fn test_private_field_admits_writes_from_its_declaring_module() {
    let session = TestSession::single(
        r#"
class Account {
    private balance: int32 = 0;

    deposit(&this, amount: int32): void {
        this.balance = amount;
    }
}
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
class Account {
    private balance: int32 = 0;

    deposit(&this, amount: int32): void {
        this.balance = amount;
    }
}

=== dir ===
class Account {
/// @type.symbol symbol=Account type=Account
/// @definition.class symbol=Account
/// @definition.field symbol=Account.balance source="private balance: int32 = 0" key=balance visibility=private type=int32
/// @definition.method symbol=Account.deposit slot=deposit type=<Account.deposit.'a>(this: &Account.deposit.'a this, int32) => void

    private balance: int32 = 0;
    /// @type.symbol symbol=Account.balance source="private balance: int32 = 0" type=int32

    deposit(&this, amount: int32): void {
    /// @generic.template symbol=Account.deposit parameters=('a)
    /// @type.symbol symbol=Account.deposit type=<Account.deposit.'a>(this: &Account.deposit.'a this, int32) => void
    /// @type.symbol symbol=Account.deposit.this source=&this type=&Account.deposit.'a this
    /// @type.symbol symbol=Account.deposit.amount source="amount: int32" type=int32

        this.balance = amount;
        /// @resolution.receiver source=this kind=this declaration=Account type=&Account.deposit.'a Account
        /// @resolution.place source=this placement=Account.deposit.'a lifetime=Account.deposit.'a access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.balance kind=place
        /// @resolution.access source=this.balance root=this keys=[balance]
        /// @resolution.assignment source=this.balance write="receiver=&Account.deposit.'a Account, target=field(receiver=&Account.deposit.'a Account, target=Account.balance, type=int32), type=int32" type=int32
        /// @resolution.name source=amount target=Account.deposit.amount
        /// @resolution.place source=amount placement="local" lifetime="frame" access="mutable"
        /// @resolution.access source=amount root=Account.deposit.amount

    }
}
"#, r#""#);
}

/// A private field rejects writes from another module.
#[test]
fn test_private_field_rejects_writes_from_a_foreign_module() {
    let session = TestSession::builder()
        .module(
            "account.ds",
            r#"
export class Account {
    private balance: int32 = 0;
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Account } from "./account.ds";

declare let account: Account;
account.balance = 1;
"#,
        )
        .build();

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
import { Account } from "./account.ds";

declare let account: Account;
account.balance = 1;

=== dir ===
import { Account } from "./account.ds";

declare let account: Account;
/// @type.symbol symbol=account source=account type=account.Account
/// @resolution.pattern source=account kind=binding target=account
/// @resolution.name source=Account target=account.Account

account.balance = 1;
/// @resolution.name source=account target=account
/// @resolution.place source=account placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=account root=account
/// @resolution.pattern.assign source=account.balance kind=place
/// @resolution.access source=account.balance root=account keys=[balance]
/// @resolution.assignment source=account.balance write="receiver=account.Account, target=field(receiver=account.Account, target=account.Account.balance, type=int32), type=int32" type=int32
"#, r#"
/// @diagnostic.error id=inaccessible-member message="member 'balance' is private"
/// @diagnostic.label line=5 column=9 span="balance" line_source="account.balance = 1;"
"#);
}

/// A private setter rejects writes from another module.
#[test]
fn test_private_setter_rejects_writes_from_a_foreign_module() {
    let session = TestSession::builder()
        .module(
            "account.ds",
            r#"
export class Account {
    balance: int32 = 0;

    private set total(&this, value: int32) {
        this.balance = value;
    }
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Account } from "./account.ds";

declare let account: Account;
account.total = 1;
"#,
        )
        .build();

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
import { Account } from "./account.ds";

declare let account: Account;
account.total = 1;

=== dir ===
import { Account } from "./account.ds";

declare let account: Account;
/// @type.symbol symbol=account source=account type=account.Account
/// @resolution.pattern source=account kind=binding target=account
/// @resolution.name source=Account target=account.Account

account.total = 1;
/// @resolution.name source=account target=account
/// @resolution.place source=account placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=account root=account
/// @resolution.pattern.assign source=account.total kind=place
/// @resolution.assignment source=account.total write="receiver=account.Account, target=account.Account.total(parameters=(int32), arguments=(supplied as int32), return=void, regions=(\"managed\" & \"local\")), type=int32" type=int32
"#, r#"
/// @diagnostic.error id=inaccessible-member message="member 'total' is private"
/// @diagnostic.label line=5 column=9 span="total" line_source="account.total = 1;"
"#);
}

/// A private constructor admits constructions from its declaring module.
#[test]
fn test_private_constructor_admits_its_declaring_module() {
    let session = TestSession::single(
        r#"
class Session {
    private constructor() {}

    static open(): Session {
        return new Session();
    }
}
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
class Session {
    private constructor() {}

    static open(): Session {
        return new Session();
    }
}

=== dir ===
class Session {
/// @type.symbol symbol=Session type=Session
/// @definition.class symbol=Session
/// @definition.method symbol=Session.constructor source="private constructor() {}" slot=constructor visibility=private role=constructor type=<Session.constructor.P0: Place>() => Managed<this, Session.constructor.P0>
/// @definition.method symbol=Session.open slot=open static=true type=() => Session

    private constructor() {}
    /// @generic.template symbol=Session.constructor parameters=(P0: Place)
    /// @type.symbol symbol=Session.constructor source="private constructor() {}" type=<Session.constructor.P0: Place>() => Managed<this, Session.constructor.P0>
    /// @type.symbol symbol=Session.constructor.this type=Session

    static open(): Session {
    /// @type.symbol symbol=Session.open type=() => Session
    /// @resolution.name source=Session target=Session

        return new Session();
        /// @resolution.construct source="new Session()" parameters=() return=local Session kind=class target=Session constructor=Session.constructor
        /// @generic.instantiation id="Session.constructor<\"local\">" template=Session.constructor arguments=("local")
        /// @generic.instantiation id="Session<\"local\">" template=Session arguments=("local")
        /// @resolution.name source=Session target=Session

    }
}
"#, r#""#);
}

/// A private constructor rejects constructions from another module.
#[test]
fn test_private_constructor_rejects_a_foreign_module() {
    let session = TestSession::builder()
        .module(
            "session.ds",
            r#"
export class Session {
    private constructor() {}
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Session } from "./session.ds";

const session = new Session();
"#,
        )
        .build();

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
import { Session } from "./session.ds";

const session: local Session = new Session();

=== dir ===
import { Session } from "./session.ds";

const session = new Session();
/// @type.symbol symbol=session source=session type=local session.Session
/// @resolution.pattern source=session kind=binding target=session
/// @resolution.construct source="new Session()" parameters=() return=local session.Session kind=class target=session.Session constructor=session.Session.symbol2
/// @generic.instantiation id="session.Session.symbol2<\"local\">" template=session.Session.symbol2 arguments=("local")
/// @generic.instantiation id="session.Session<\"local\">" template=session.Session arguments=("local")
/// @resolution.name source=Session target=session.Session
"#, r#"
/// @diagnostic.error id=inaccessible-member message="member 'constructor' is private"
/// @diagnostic.label line=4 column=17 span="new Session()" line_source="const session = new Session();"
"#);
}

/// A protected constructor admits derived classes and rejects unrelated sites.
#[test]
fn test_protected_constructor_admits_derived_classes_only() {
    let session = TestSession::builder()
        .module(
            "base.ds",
            r#"
export class Shape {
    protected constructor() {}
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Shape } from "./base.ds";

const shape = new Shape();
"#,
        )
        .build();

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
import { Shape } from "./base.ds";

const shape: local Shape = new Shape();

=== dir ===
import { Shape } from "./base.ds";

const shape = new Shape();
/// @type.symbol symbol=shape source=shape type=local base.Shape
/// @resolution.pattern source=shape kind=binding target=shape
/// @resolution.construct source="new Shape()" parameters=() return=local base.Shape kind=class target=base.Shape constructor=base.Shape.symbol2
/// @generic.instantiation id="base.Shape.symbol2<\"local\">" template=base.Shape.symbol2 arguments=("local")
/// @generic.instantiation id="base.Shape<\"local\">" template=base.Shape arguments=("local")
/// @resolution.name source=Shape target=base.Shape
"#, r#"
/// @diagnostic.error id=inaccessible-member message="member 'constructor' is protected"
/// @diagnostic.label line=4 column=15 span="new Shape()" line_source="const shape = new Shape();"
"#);
}

/// An accessor pair splits visibility between reads and writes.
#[test]
fn test_accessor_visibility_splits_reads_and_writes() {
    let session = TestSession::builder()
        .module(
            "gauge.ds",
            r#"
export class Gauge {
    stored: int32 = 0;

    get level(&readonly this): int32 {
        return this.stored;
    }

    private set level(&this, next: int32) {
        this.stored = next;
    }
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Gauge } from "./gauge.ds";

declare const gauge: Gauge;
const read = gauge.level;

function drain(gauge: &Gauge): void {
    gauge.level = 0;
}
"#,
        )
        .build();

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
import { Gauge } from "./gauge.ds";

declare const gauge: Gauge;
const read: int32 = gauge.level;

function drain<'a>(gauge: &'a Gauge): void {
    gauge.level = 0;
}

=== dir ===
import { Gauge } from "./gauge.ds";

declare const gauge: Gauge;
/// @type.symbol symbol=gauge source=gauge type=gauge.Gauge
/// @resolution.pattern source=gauge kind=binding target=gauge
/// @resolution.name source=Gauge target=gauge.Gauge

const read = gauge.level;
/// @type.symbol symbol=read source=read type=int32
/// @resolution.pattern source=read kind=binding target=read
/// @resolution.name source=gauge target=gauge
/// @resolution.member source=gauge.level receiver=gauge.Gauge type=int32 kind=call target="gauge.Gauge.level#1(parameters=(), arguments=(), return=int32, regions=(\"managed\" & \"local\"))"
/// @resolution.place source=gauge placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=gauge root=gauge

function drain(gauge: &Gauge): void {
/// @generic.template symbol=drain parameters=('a)
/// @type.symbol symbol=drain type=<drain.'a>(&drain.'a gauge.Gauge) => void
/// @type.symbol symbol=drain.gauge source="gauge: &Gauge" type=&drain.'a gauge.Gauge
/// @resolution.name source=Gauge target=gauge.Gauge

    gauge.level = 0;
    /// @resolution.name source=gauge target=drain.gauge
    /// @resolution.place source=gauge placement=drain.'a lifetime=drain.'a access="mutable"
    /// @resolution.access source=gauge root=drain.gauge
    /// @resolution.pattern.assign source=gauge.level kind=place
    /// @resolution.assignment source=gauge.level write="receiver=&drain.'a gauge.Gauge, target=gauge.Gauge.level#2(parameters=(int32), arguments=(supplied as int32), return=void, regions=(drain.'a)), type=int32" type=int32

}
"#, r#"
/// @diagnostic.error id=inaccessible-member message="member 'level' is private"
/// @diagnostic.label line=8 column=11 span="level" line_source="gauge.level = 0;"
"#);
}
