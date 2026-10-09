# @destack/access

`Policy` is a SpiceDB schema definition (`relation`, `permission`, `union` as `+`, `intersection` as `&`, `exclusion` as `-`, `through` as the `->` arrow, `members` as a subject set, `resource` and `context` as caveats), `Authorizer` is SpiceDB's `CheckPermission`, `LookupSubjects` and `LookupResources` compiled to SQL over the object's own database, and `elevated` is RFC 9470 step-up authentication.

```ts
read: union(relation("owner"), relation("editor"), through("parent", "read")); // owner + editor + parent->read
reader: { subjects: [principal.user, folder.members("read")] }; // user | folder#read
edit: intersection(relation("editor"), resource({ team: sql.placeholder("team") })); // a caveat
await authorizer.check(snapshot, page.permission("edit"), target, access); // CheckPermission
await authorizer.subjects(snapshot, page.permission("read"), target, type, Date.now(), { limit: 50 }); // LookupSubjects
await database.select().from(pages).where(authorizer.where(page.permission("read"), access)); // LookupResources as SQL
await authorization.grant({ object: page, relation: "editor", subject }); // WriteRelationships
```

## Policies

A `Policy` declares an object type's relations and permissions, and `elevated` asks for a recent stronger sign-in.

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

## Decisions

An `Authorizer` resolves a caller's `Access` in a scope, and `require` throws an `AccessError` naming the first permission the caller lacks.

```ts
const authorizer = new Authorizer([note], [{ policy: note, table: notes, id: "id", scope: "scope", attributes: {}, relations: {} }]);
const snapshot = Snapshot.live(database);
const access = await authorizer.resolve(snapshot, spaceId, context);
await authorizer.require(snapshot, [note.permission("read")], target, access);
await authorizer.explain(snapshot, note.permission("delete"), target, access); // the gate or failing grants
```

## Changes

`Authorization` changes access as one caller, requiring the caller's permission for each change.

```ts
const authorization = new Authorization(authorizer, database, (scope) => caller.context(scope));
await authorization.revoke(page, relationshipId);
const { id, secret } = await authorization.link({ object: page, relation: "viewer" }); // a share link
const offer = await authorization.invite({ relationship: { object: page, relation: "editor", subject: contact } });
await authorization.accept(page, offer.id);
```

## Roles

`createRole` creates a role granting permissions the caller has, `keepRole` keeps a declared role by name, and `assign` binds a role to one subject.

```ts
const role = await authorization.createRole(space, {
    name: "reviewer",
    description: "Read and share every note",
    permissions: [note.permission("read"), note.permission("share")],
});
await authorization.assign(space, role.id, carol);
```

## Storage

`accessTables` lists the tables every database with protected objects includes.

```ts
export const main = defineDatabase({ name: "main", tables: [...accessTables, notes] });
```
