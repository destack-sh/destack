# @destack/access

Decide who may do what to each object, and share, delegate and revoke that access.

## Overview

A package declares a policy per object type, and an authorizer decides each call against the relationships stored in the space's database.

- **Policy**: the relations and permissions of one object type; `permission` references one of them.
- **Caller**: who calls, with the actors acting for it; an `AccessContext` adds the time and what the request presented.
- **Authorizer**: decides permissions over the database; `resolve` reads a caller's `Access` in a scope, `check` answers one permission and `require` throws an `AccessError` for the first one missing.
- **Authorization**: one caller's changes in one transaction; every change requires the caller's permission.
- **Relationship**: a subject related to an object; `grant`, `link` and `revoke` change it.
- **Role**: a named set of permissions in a scope; `create`, `upsert` and `assign`.
- **Grant**: permissions a delegator lets a delegate use for it; `create`, `act` and `revoke`.

```ts
import {
    AccessContext,
    Authorization,
    type AuthorizationEnvironment,
    Authorizer,
    Caller,
    Policy,
    principal,
    Relationship,
    relation,
    union,
} from "@destack/access";
import { type DatabaseConnection, Snapshot } from "@destack/db";
import type { ObjectReference, Subject } from "@destack/sync";

const note = new Policy(import.meta.destack.package, {
    name: "note",
    relations: {
        owner: { subjects: [principal.user], grantedBy: null },
        editor: { subjects: [principal.user] },
    },
    permissions: { read: union(relation("owner"), relation("editor")), share: relation("owner") },
    grantedBy: "share",
});

async (
    authorizer: Authorizer,
    database: DatabaseConnection,
    environment: AuthorizationEnvironment,
    alice: Subject,
    bob: Subject,
    target: ObjectReference,
) => {
    const context = AccessContext.of(Caller.of(alice), Date.now());
    await Authorization.run(authorizer, database, () => context, environment, (authorization) =>
        Relationship.grant({ object: target, relation: "editor", subject: bob }, authorization),
    );
    const snapshot = Snapshot.live(database);
    const access = await authorizer.resolve(snapshot, target.scope, context);
    await authorizer.require(snapshot, [note.permission("read")], target, access);
};
```

## Examples

### Declare a policy

Permissions combine relations with `union`, `intersection`, `exclusion` and `through`, and `elevated` asks for a recent stronger sign-in.

```ts
import { Policy, principal, relation, through, union } from "@destack/access";

export const folder = new Policy(import.meta.destack.package, {
    name: "folder",
    relations: { owner: { subjects: [principal.user], grantedBy: null } },
    permissions: { read: relation("owner"), delete: relation("owner") },
});
export const page = new Policy(import.meta.destack.package, {
    name: "page",
    relations: {
        owner: { subjects: [principal.user], grantedBy: null },
        parent: { subjects: [folder], grantedBy: null, onDelete: "cascade" },
    },
    permissions: {
        read: union(relation("owner"), through("parent", "read")),
        delete: relation("owner"),
    },
    elevated: { delete: { assurance: 2, maxAge: 15 * 60 * 1000 } },
});
```

### Check a permission

`check` answers one permission on one object as a `Decision`, and `explain` lists the grants that decided it.

```ts
import type { Authorizer, Access, PermissionReference } from "@destack/access";
import type { Snapshot } from "@destack/db";
import type { ObjectReference } from "@destack/sync";

async (
    authorizer: Authorizer,
    snapshot: Snapshot,
    permission: PermissionReference,
    target: ObjectReference,
    access: Access,
) => {
    const decision = await authorizer.check(snapshot, permission, target, access);
    const explanation = await authorizer.explain(snapshot, permission, target, access);
};
```

### Define and assign a role

A role grants only permissions its creator has, and `assign` binds it to exactly the given subjects.

```ts
import { type Authorization, type PermissionReference, Role } from "@destack/access";
import type { ObjectReference, Subject } from "@destack/sync";

async (
    authorization: Authorization,
    space: ObjectReference,
    read: PermissionReference,
    carol: Subject,
) => {
    const request = { name: "reviewer", description: "Read every note", permissions: [read] };
    const role = await Role.create(space, request, authorization);
    await Role.assign(space, role.id, [carol], authorization);
};
```

### Delegate to an agent

A principal grants a delegate some of its permissions, and the delegate then acts for it with what both allow.

```ts
import { type Authorization, Grant, type PermissionReference } from "@destack/access";
import type { Snapshot } from "@destack/db";
import type { ObjectReference, Subject } from "@destack/sync";

async (
    authorization: Authorization,
    snapshot: Snapshot,
    alice: ObjectReference,
    agent: Subject,
    read: PermissionReference,
) => {
    const request = { delegate: agent, permissions: [read] };
    const grant = await Grant.create(alice, request, authorization);
    const caller = await Grant.act(snapshot, { delegator: alice, delegate: agent }, Date.now());
    await Grant.revoke(alice, grant.id, authorization);
};
```

## Comparison

### SpiceDB

Relations, permissions and their operators follow SpiceDB's schema language; Destack differs in three ways.

- Relationships live in the space's database, and `Authorizer.candidates` turns a permission into SQL over the object's table.
- A `Grant` lets a delegate act for its delegator as an RFC 8693 actor chain, limited to RFC 9396 permissions.
- `elevated` asks for a recent stronger sign-in per permission, after RFC 9470 step-up authentication.
