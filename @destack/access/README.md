# @destack/access

Decide who may do what to each object, and share, delegate and revoke that access.

## Overview

`@destack/access` is SpiceDB's model in the space's own database: policies, relationships as rows, and authorization in the same transaction.

Packages declare access per object type:

- `Policy`: the relations and permissions of one object type, built from `relation`, `union`, `through` and the other expressions.

Servers decide each call:

- `Authorizer`: checks, requires and explains a caller's permissions.
- `Authorization`: one caller's changes in one transaction.

People share and delegate:

- `Relationship`: a subject related to an object, such as `alice` as a note's `editor`.
- `Role`: a named set of permissions in a scope, bound to subjects.
- `Grant`: permissions a delegate, such as an agent, uses on someone's behalf.
- `Invitation`: a relationship that applies once accepted.

Declare a policy, share a note with Bob, then require read access:

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

// declare who may read and share a note
export const note = new Policy(import.meta.destack.package, {
    name: "note",
    relations: {
        owner: { subjects: [principal.user], grantedBy: null },
        editor: { subjects: [principal.user] },
    },
    permissions: { read: union(relation("owner"), relation("editor")), share: relation("owner") },
    grantedBy: "share",
});

/** Share a note with Bob as Alice, then require that Alice may still read it. */
export async function shareNote(
    authorizer: Authorizer,
    database: DatabaseConnection,
    environment: AuthorizationEnvironment,
    alice: Subject,
    bob: Subject,
    target: ObjectReference,
) {
    // make Bob an editor as Alice
    const context = AccessContext.of(Caller.of(alice), Date.now());
    await Authorization.run(
        authorizer,
        database,
        () => context,
        environment,
        (authorization) =>
            Relationship.grant({ object: target, relation: "editor", subject: bob }, authorization),
    );

    // require that Alice may read
    const snapshot = Snapshot.live(database);
    const access = await authorizer.resolve(snapshot, target.scope, context);
    await authorizer.require(snapshot, [note.permission("read")], target, access);
}
```

## Reference

### Policy

`new Policy(owner, { name, relations, permissions, … })` declares an object type's access.

| Field                   | What it is                                                                 |
| ----------------------- | -------------------------------------------------------------------------- |
| `relations`             | relations by name, each `{ subjects, grantedBy, isRequired, onDelete, … }` |
| `permissions`           | permissions by name, each an expression below                              |
| `grantedBy`             | the permission that grants relations and binds roles by default            |
| `attributes`, `context` | the object and request attributes conditions read                          |
| `elevated`              | permissions that need a recent stronger sign-in, `{ assurance, maxAge }`   |
| `reserved`              | permissions owner roles leave out                                          |
| `suspended`, `moving`   | permissions left while the scope is suspended or moving                    |
| `scope`, `partition`    | whether its objects are scopes, and whether each is a partition            |

Permissions are expressions over relations, as in SpiceDB's schema language:

| Expression                                 | Permits                                                             |
| ------------------------------------------ | ------------------------------------------------------------------- |
| `relation(name)`                           | the subjects of a relation                                          |
| `permission(name)`                         | whoever has another permission on the same object                   |
| `union(…)`, `intersection(…)`              | whoever any, or every, expression permits                           |
| `exclusion(include, exclude)`              | whoever one permits and the other doesn't                           |
| `through(relation, permission)`            | whoever has a permission on the related object, as SpiceDB's `->`   |
| `contained(…)`                             | the principals inside a scope and its enclosed scopes               |
| `subjectSet(…)`                            | the members of an object's subject set, such as a group             |
| `self()`                                   | the principal the object is                                         |
| `resource(where)`, `context(where)`        | objects, or requests, whose attributes meet a condition             |
| `grantersOf(field)`, `readersOf(field, …)` | who may grant on, or sees the relationships of, a referenced object |
| `none()`                                   | nobody, so only roles grant it                                      |

### Authorizer

An authorizer decides permissions over a database's policies and relationships.

| Verb                                             | What it does                                                                 |
| ------------------------------------------------ | ---------------------------------------------------------------------------- |
| `resolve(snapshot, scope, context)`              | resolves a caller's `Access` in a scope: subject sets, roles and scope chain |
| `check(snapshot, permission, target, access)`    | decides one permission on one object, and until when                         |
| `require(snapshot, permissions, target, access)` | throws for the first permission the caller lacks                             |
| `explain(snapshot, permission, target, access)`  | lists the gate and each grant that decided it                                |
| `candidates(…)`, `visibility(…)`                 | turns a permission into SQL over a table, for list queries                   |
| `subjects(…)`                                    | lists the principals with a permission on an object                          |

### Authorization

`Authorization.run(authorizer, database, context, environment, call)` runs a call as one caller in a transaction.
Each change requires the caller's permission.

### Relationship, Role, Grant and Invitation

These run as the caller of an `Authorization`.

| Verb                                                     | What it does                                                                                                         |
| -------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `Relationship.grant(request, authorization)`             | relates a subject to an object by a relation or role                                                                 |
| `Relationship.revoke(…)`, `Relationship.replace(…)`      | revokes one relationship, or replaces a set with exactly the wanted ones                                             |
| `Role.create(scope, request, authorization)`             | defines a role with permissions the caller has there                                                                 |
| `Role.update(…)`, `Role.delete(…)`                       | changes or deletes a role                                                                                            |
| `Role.declare(scope, declaration, authorization)`        | declares a role: exactly its permissions and subjects, removed once it grants none, as Kubernetes' server-side apply |
| `Role.assign(scope, role, subjects, authorization)`      | binds a role to exactly some subjects                                                                                |
| `Grant.create(delegator, request, authorization)`        | lets a delegate use some of the delegator's permissions                                                              |
| `Grant.act(snapshot, { delegator, delegate }, now)`      | gives the caller acting for a delegator within its grant                                                             |
| `Grant.revoke(delegator, grant, authorization)`          | revokes a grant                                                                                                      |
| `Invitation.create(request, authorization)`              | invites a relationship that applies once accepted                                                                    |
| `Invitation.accept(id, …)`, `Invitation.withdraw(id, …)` | accepts or withdraws an invitation                                                                                   |

## Examples

### Declare nested permissions

`through` follows a relation, so a folder's readers can read its pages.

```ts
import { Policy, principal, relation, through, union } from "@destack/access";

// declare folders
export const folder = new Policy(import.meta.destack.package, {
    name: "folder",
    relations: { owner: { subjects: [principal.user], grantedBy: null } },
    permissions: { read: relation("owner"), delete: relation("owner") },
});

// declare pages in folders, deleted after a recent strong sign-in
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

### Invite someone

An invitation offers a relationship that applies once its addressee accepts it.

```ts
import { type Authorization, Invitation } from "@destack/access";
import type { ObjectReference, Subject } from "@destack/sync";

/** Invite Dana to edit a note, then accept as Dana. */
export async function inviteDana(
    inviter: Authorization,
    dana: Authorization,
    note: ObjectReference,
    subject: Subject,
) {
    // offer the relationship for a week
    const invitation = await Invitation.create(
        {
            relationship: { object: note, relation: "editor", subject },
            purpose: "Review the draft",
        },
        inviter,
    );

    // accept it as the addressee
    return await Invitation.accept(invitation.id, dana);
}
```

### Share until a date

A relationship with `expiresAt` stops applying at that time.

```ts
import { type Authorization, Relationship } from "@destack/access";
import type { ObjectReference, Subject } from "@destack/sync";

/** Let a contractor read a project until the end of the month. */
export async function shareUntil(
    authorization: Authorization,
    project: ObjectReference,
    contractor: Subject,
    endOfMonth: number,
) {
    await Relationship.grant(
        { object: project, relation: "reader", subject: contractor, expiresAt: endOfMonth },
        authorization,
    );
}
```

### List who may read an object

`subjects` lists the principals of a type with a permission, through groups and roles.

```ts
import { type Authorizer, type PermissionReference, principal } from "@destack/access";
import type { Snapshot } from "@destack/db";
import type { ObjectReference } from "@destack/sync";

/** List the first hundred users who may read a note. */
export async function readers(
    authorizer: Authorizer,
    snapshot: Snapshot,
    read: PermissionReference,
    note: ObjectReference,
) {
    // read users, by their type's reference
    const user = { packageId: principal.user.definition.packageId, type: principal.user.name };

    return await authorizer.subjects(snapshot, read, note, user, Date.now(), { limit: 100 });
}
```

### Explain a denial

`check` answers a `Decision`, and `explain` lists why each path failed.

```ts
import type { Access, Authorizer, PermissionReference } from "@destack/access";
import type { Snapshot } from "@destack/db";
import type { ObjectReference } from "@destack/sync";

/** Read why a caller may not use a permission on an object, absent when it may. */
export async function explainDenial(
    authorizer: Authorizer,
    snapshot: Snapshot,
    permission: PermissionReference,
    target: ObjectReference,
    access: Access,
) {
    // explain only a denial
    const decision = await authorizer.check(snapshot, permission, target, access);
    if (decision.isAllowed) {
        return undefined;
    }

    return await authorizer.explain(snapshot, permission, target, access);
}
```

### Add a reviewer role

A role grants only permissions its creator has.

```ts
import { type Authorization, type PermissionReference, Role } from "@destack/access";
import type { ObjectReference, Subject } from "@destack/sync";

/** Let Carol read every note in a space as its reviewer. */
export async function addReviewer(
    authorization: Authorization,
    space: ObjectReference,
    read: PermissionReference,
    carol: Subject,
) {
    // define the role, then bind it to Carol
    const request = { name: "reviewer", description: "Read every note", permissions: [read] };
    const role = await Role.create(space, request, authorization);
    await Role.assign(space, role.id, [carol], authorization);
}
```

### Delegate to an agent

A delegate acts with what both it and the delegator may do.

```ts
import { type Authorization, Grant, type PermissionReference } from "@destack/access";
import type { Snapshot } from "@destack/db";
import type { ObjectReference, Subject } from "@destack/sync";

/** Let an agent read for Alice, act once as her, then revoke it. */
export async function delegateReading(
    authorization: Authorization,
    snapshot: Snapshot,
    alice: ObjectReference,
    agent: Subject,
    read: PermissionReference,
) {
    // grant the agent Alice's read permission
    const grant = await Grant.create(
        alice,
        { delegate: agent, permissions: [read] },
        authorization,
    );

    // act as Alice, then revoke the grant
    const caller = await Grant.act(snapshot, { delegator: alice, delegate: agent }, Date.now());
    await Grant.revoke(alice, grant.id, authorization);

    return caller;
}
```

## Comparison

### SpiceDB

Relations, permissions and their operators basically follow SpiceDB's schema language.
Relationships are rows next to the objects, so checks and list filters are SQL in the same transaction.

| SpiceDB              | `@destack/access`                               | Difference                                                      |
| -------------------- | ----------------------------------------------- | --------------------------------------------------------------- |
| `definition`         | `Policy`                                        | Declared in TypeScript by the package that owns the type.       |
| `+`, `&`, `-`, `->`  | `union`, `intersection`, `exclusion`, `through` | The same.                                                       |
| `CheckPermission`    | `Authorizer.check`                              | Also answers until when the decision holds.                     |
| `LookupResources`    | `Authorizer.candidates`                         | Gives SQL over the object's table instead of a list of ids.     |
| `WriteRelationships` | `Relationship.grant`, `revoke`                  | Requires the caller's grant permission in the same transaction. |
| caveats              | `resource(where)`, `context(where)`             | Conditions on object and request attributes.                    |

### OAuth delegation and step-up

Grants and elevation follow OAuth's delegation and step-up standards.

| Standard                        | `@destack/access`          | Difference                                           |
| ------------------------------- | -------------------------- | ---------------------------------------------------- |
| RFC 8693 actor chains           | `Grant.act`, `Grant.chain` | A delegate acts with what every link allows.         |
| RFC 9396 authorization details  | a grant's `permissions`    | Permissions on objects instead of free-form details. |
| RFC 9470 step-up authentication | `elevated`                 | Declared per permission in the policy.               |
