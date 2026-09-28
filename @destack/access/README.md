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

Expressions define who holds a permission.

| Expression | Grants |
|---|---|
| `relation(name)` | The subjects related to the object |
| `permission(name)` | The holders of another permission on the object |
| `through(relation, permission)` | The holders of a permission on the related object |
| `grants(reference)` | Whoever may grant on the object a row references |
| `condition(condition)` | Everyone, where a db `Condition` over the object's attributes holds |
| `union`, `intersection`, `exclusion` | Compositions of the above |
| `none()` | Nobody |

## Authorizer

An `Authorizer` decides permissions on the objects in one database.

```ts
const authorizer = new Authorizer([note], [
    { policy: note, table: notes, id: "id", scope: "scope", attributes: {}, relations: {} },
]);
const snapshot = Snapshot.live(database);
const access = await authorizer.resolve(snapshot, spaceId, context);
await database.select().from(notes).where(authorizer.where(note.permission("read"), access));
const decision = await authorizer.check(snapshot, note.permission("share"), note.reference(spaceId, id), access);
```

## Decisions

`where` decides lists in SQL, and `check` decides single objects.

| Method | Decides |
|---|---|
| `resolve` | A caller's `Access` in a scope: its authorities, the scope chain and the roles along it |
| `resolveAssured` | A principal's `Access` as if it just authenticated at the highest assurance |
| `where`, `holds` | The rows a caller holds a permission on, as SQL |
| `check` | One object, as a `Decision` with the time it next changes |
| `require` | Every permission on one object, or an `AccessError` |
| `checkRows` | Many rows through one `GrantReader` |
| `subjects` | A page of the principals of a type holding a permission on one object |
| `explain` | Why a caller holds a permission or not, per authority |
| `challenge` | The `StepUp` that would admit a caller refused for weak authentication |

## Authorization

An `Authorization` decides and changes access as one caller.

```ts
const authorization = new Authorization(authorizer, database, (scope) => caller.context(scope));
await authorization.create(page, { relationships: [{ relation: "owner", subject: caller.subject }] });
await authorization.grant({ object: page, relation: "editor", subject });
const link = await authorization.link({ object: page, relation: "viewer" });
const offer = await authorization.propose({ relationship: { object: page, relation: "editor" }, recipient });
```

## Changes

Each change requires the caller to hold its permission.

| Method | Effect |
|---|---|
| `create` | A new object's first relationships, and a new scope's ancestry and owner role |
| `suspend`, `resume` | A scope withholding every permission but administration |
| `grant`, `revoke`, `link` | Relationships and capability links |
| `propose`, `accept`, `decline`, `proposals`, `addressed` | Relationships that apply once the recipient accepts |
| `createRole`, `updateRole`, `deleteRole` | Roles granting only permissions the caller holds |

## Scopes

`Scope` reads a scope's ancestors and fences it while its database moves.

| Function | Effect |
|---|---|
| `Scope.chain` | The scope and the scopes enclosing it, nearest first |
| `Scope.object` | The scope's own object |
| `Scope.fence`, `Scope.unfence` | Send the scope's writes to another holder, and stop |
| `Scope.guard` | Keep a write's scopes unfenced until it commits |

## Storage

Every database holding protected objects includes `ACCESS_TABLES`.

```ts
export const main = defineDatabase({ name: "main", tables: [...ACCESS_TABLES, notes] });

const replica = authorizer.replica(spaceId);
```
