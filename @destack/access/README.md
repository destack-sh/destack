# @destack/access

Relate subjects to objects and decide their permissions.

## Policies

A `Policy` declares an object type's relations and permissions.

```ts
export const note = new Policy(import.meta.destack.package, {
    name: "note",
    relations: {
        owner: { subjects: [principal.user], grantedBy: null },
        editor: { subjects: [principal.user, group.members("member")] },
        parent: { subjects: ["note"], grantedBy: null },
    },
    permissions: {
        read: union(relation("owner"), relation("editor"), through("parent", "read")),
        share: relation("owner"),
        delete: relation("owner"),
    },
    grantedBy: "share",
    elevated: { delete: { assurance: 2, maxAge: 15 * 60 * 1000 } },
});
```

## Expressions

`relation`, `permission`, `through` and the other expressions define who holds a permission.

```ts
relation("owner"); // the subjects related to the object
permission("read"); // the holders of another permission on the object
through("parent", "read"); // the holders of a permission on the related object
grantersOf("object"); // whoever may grant on the object a row references
readersOf("object", "relation"); // whoever sees the relationships of the object a row references
contained(principal.installation); // the principals inside the object's scope, or inside the scope it is
resource({ team: 1 }); // everyone, where a condition over the object's attributes holds
context({ team: 1 }); // everyone, where a condition over the request's attributes holds
union(relation("owner"), relation("editor")); // also intersection and exclusion
none(); // nobody
```

## Request attributes

`attributes` types the object attributes `condition` reads, and `context` types the request attributes `context` expressions and placeholders read.

```ts
export const sheet = new Policy(import.meta.destack.package, {
    name: "sheet",
    attributes: { team: "number" },
    context: { team: "number" },
    relations: { editor: { subjects: [principal.user] } },
    permissions: {
        edit: intersection(relation("editor"), resource({ team: sql.placeholder("team") })),
    },
});
```

## Relationship visibility

`relationships.read` names the permission whose holders see an object's relationships, `read` by default, and `concealed` shows a relation's relationships only to holders of the permission granting it.

```ts
relations: { reviewer: { subjects: [principal.user], concealed: true, grantedBy: "share" } },
relationships: { read: "share" },
```

## Inheritance

`isScope` marks the relation naming the scope containing each object, which `through` reads the scope's permissions over.

```ts
export const document = new Policy(import.meta.destack.package, {
    name: "document",
    relations: { folder: { subjects: [folder], grantedBy: null, isScope: true } },
    permissions: { edit: through("folder", "edit") },
});
```

## Subject sets

`members` names a relation or permission of another object as a subject set.

```ts
relations: { reader: { subjects: [principal.user, folder.members("read")] } },
```

## Authorizer

An `Authorizer` resolves a caller's `Access` in a scope and decides permissions on the objects in one database.

```ts
const authorizer = new Authorizer(
    [note],
    [{ policy: note, table: notes, id: "id", scope: "scope", attributes: {}, relations: {} }],
);
const snapshot = Snapshot.live(database);
const access = await authorizer.resolve(snapshot, spaceId, context);
```

## Lists

`where` restricts a query to the rows a caller has a permission on, in SQL.

```ts
await database
    .select()
    .from(notes)
    .where(authorizer.where(note.permission("read"), access));
```

## Objects

`check` decides one object with the time its decision next changes, and `require` throws an `AccessError` naming the first permission the caller lacks.

```ts
const target = note.reference(spaceId, id);
const { isAllowed, until } = await authorizer.check(
    snapshot,
    note.permission("share"),
    target,
    access,
);
await authorizer.require(
    snapshot,
    [note.permission("read"), note.permission("share")],
    target,
    access,
);
```

## Explanations

`explain` names the gate or the failing grants per authority, and `challenge` finds the `StepUp` that admits a caller refused for weak authentication.

```ts
const explanation = await authorizer.explain(snapshot, note.permission("delete"), target, access);
const stepUp = await authorizer.challenge(
    snapshot,
    note.permission("delete"),
    access,
    async (stepped) =>
        (await authorizer.check(snapshot, note.permission("delete"), target, stepped)).isAllowed,
);
```

## Subjects

`subjects` lists a page of the principals of one type that have a permission on an object.

```ts
const { packageId, name } = principal.user.definition;
const readers = await authorizer.subjects(
    snapshot,
    note.permission("read"),
    target,
    { packageId, type: name },
    Date.now(),
    { limit: 50 },
);
```

## Authorization

`Authorization` decides and changes access as one caller, requiring the caller's permission for each change.

```ts
const authorization = new Authorization(authorizer, database, (scope) => caller.context(scope));
await authorization.create(page, {
    relationships: [{ relation: "owner", subject: caller.subject }],
});
await authorization.grant({ object: page, relation: "editor", subject });
await authorization.revoke(page, relationshipId);
```

## Links

`link` relates whoever holds a new secret to an object and returns the secret once.

```ts
const { id, secret } = await authorization.link({ object: page, relation: "viewer" });
```

## Invitations

`invite` keeps a pending grant to a principal or a contact, `accept` applies it, and `withdraw` revokes it, each settled invitation kept with its status.

```ts
const offer = await authorization.invite({
    relationship: { object: page, relation: "editor", subject: contact },
});
await authorization.accept(page, offer.id); // as the invited principal, or whoever verified the contact
const pending = await authorization.invitations({ object: page }, { limit: 20 });
```

## Roles

`createRole` creates a role granting named permissions the caller has, and system code keeps roles through the system's authorization like any caller.

```ts
const role = await authorization.createRole(space, {
    name: "reviewer",
    description: "Read and share every note",
    permissions: [note.permission("read"), note.permission("share")],
});

// keep a role by name with exactly these permissions, writing nothing when it matches
const operator = await system.keepRole(Scope.universe, OPERATOR_ROLE);

// bind it to one subject alone, revoking its other bindings
await system.assign(Scope.universe, operator.id, owners);

// read roles
await Role.read(database, scope, role.id);
await Role.permissions(database, role.id);
```

## Declared access

A stack declaration's roles and relationships carry its `Manager`, which only the system changes, and `keepRole` and `keepRelationships` keep exactly what it declares, unchanged ones under their identifiers.

```ts
const manager = Manager.schema.parse({ installationId, packageId, name: "roles/reader" });
await system.keepRole(space, { name: "reader", description: "Reads notes", permissions }, manager);
await system.keepRelationships({ scope: space.id, manager }, [
    { object: space, role: reader.id, subject: carol },
]);
```

## Grants

`keepGrant` keeps one managed role granting exactly some permissions bound to one subject, and drops both when it grants none.

```ts
await system.keepGrant(
    home,
    {
        name: `installation/${installationId}`,
        description: "What the inbox may do for Ada",
        permissions, // [] drops the role and its binding
        subject: principal.installation.reference(spaceId, installationId),
        conditions: { onBehalfOf: ada }, // applies only while the installation acts for Ada
    },
    manager,
);
```

## Storage

`accessTables` lists the tables every database with protected objects includes.

```ts
export const main = defineDatabase({ name: "main", tables: [...accessTables, notes] });
```

## Copies

`chain` lists the `chain` shape subscriptions a database below a scope follows, one copy per scope above it, each named by `Authorizer.chainCopy(below)`.

```ts
for (const request of await authorizer.chain(database, spaceId, { isHome: true })) {
    await authorizer
        .replicaOf(request)
        .follow(
            database,
            (from, stream) => source.stream({ ...request, ...from }, stream),
            signal,
            { subscription: request },
        );
}

// request one copy at the universe of the chains a placed workload replicates
const request = await authorizer.chainVia(database, placementId, accountIds);
```

## Represented

`Caller.represent` lets a caller act as the principal an object stands for, with the caller as its actor, and `AccessContext.withCells` lets hosts act for the cells they are.

```ts
const represented = Caller.represent(caller, {
    permission,
    object: space,
    subject,
    within: [account],
});
const served = AccessContext.withCells(context);
```

## Enclosed scopes

`Access.descend` resolves a caller in the scopes below one, and `checkRows` decides each row by the caller's access in the row's own scope.

```ts
const above = await authorizer.resolve(snapshot, accountId, context);
const below = await above.descend(snapshot, spaceIds);
const { permitted } = await authorizer.checkRows(
    snapshot,
    permission,
    above,
    rows,
    undefined,
    below,
);
```

## Errors

A refused or invalid access decision throws an `AccessError`, and `toServiceError` names the service error its caller receives, with the step-up a challenge asks for.

```ts
import { AccessError } from "@destack/access/error";

new AccessError("INSUFFICIENT_AUTHENTICATION", "authenticate again", {
    stepUp: { assurance: 2 },
}).toServiceError(); // { code: "INSUFFICIENT_AUTHENTICATION", message: "authenticate again", data: { assurance: 2 } }
```

## Tests

`AccessFixture` records the scopes, owners and roles a copy of another service's access keeps in a database, for tests of services that follow one.

```ts
import { AccessFixture } from "@destack/access/test";

const copies = new AccessFixture(database);
await copies.copyScope(space.reference(Scope.universe.id, spaceId));
const editor = {
    name: "editor",
    description: "Edit notes",
    permissions: [note.permission("edit")],
};
await copies.copyRole(scope, editor, bob);
```
