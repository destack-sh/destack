Relate subjects to objects, decide permissions in SQL and in memory, and propose, delegate and copy access.

## Policies

A policy is the relations and permissions of one type of object.

```ts
const note = new Policy(import.meta.destack.package, {
    name: "note",
    relations: {
        owner: { subjects: [principal.user], grantedBy: null },
        editor: { subjects: [principal.user, group.members("member")] },
        parent: { subjects: ["note"], grantedBy: null },
    },
    permissions: {
        read: union(relation("owner"), relation("editor"), through("parent", "read")),
        share: relation("owner"),
    },
    grantedBy: "share",
});
```

| Expression | Grants |
|---|---|
| `relation(name)` | The subjects related to the object |
| `through(relation, permission)` | The holders of a permission on the related object |
| `grants(reference)` | Whoever may grant on the object a row references |
| `condition(Condition)` | Everyone, where a db `Condition` over the object's attributes and the request's parameters holds |
| `union`, `intersection`, `exclusion` | Compositions of the above |

| Field | Meaning |
|---|---|
| `grantedBy` | The permission whose holders bind roles and grant relations; a relation overrides it, `null` leaves it to the system |
| `reserved` | Permissions only expressions grant, never roles |
| `elevated` | The `Elevation` each sensitive permission asks for: an assurance level reached within a maximum age |
| `administration` | Permissions that stay available in a suspended scope |
| `scope` | The type's objects are scopes, containing other objects |

An open relation takes the subject types other policies contribute, as an attachment's parent takes each host listing it.

```ts
const comment = new Policy(pkg, { name: "comment", relations: { parent: { subjects: [], open: true, grantedBy: null } }, permissions: { read: through("parent", "read") } });
const note = new Policy(pkg, { name: "note", ..., contributes: [{ policy: comment, relation: "parent" }] });
```

## Authorizer

An authorizer decides policies over a snapshot of the database their objects live in.

```ts
const authorizer = new Authorizer([note], [{ policy: note, table: notes, id: "id", scope: "scope", attributes: {}, relations: {} }]);
const snapshot = Snapshot.live(database);
const access = await authorizer.resolve(snapshot, spaceId, context);
await database.select().from(notes).where(authorizer.where(note.permission("read"), access));
const decision = await authorizer.check(snapshot, note.permission("share"), note.reference(spaceId, id), access);
```

| Verb | Decides |
|---|---|
| `resolve` | A caller in a scope: its authorities, the scope chain and the roles along it |
| `where` | The rows a caller holds a permission on, as SQL before sorting and paging |
| `check`, `require`, `owns` | One object; a `Decision` holds until the moment time alone may change it |
| `checkRows` | Rows through the grant trees a `GrantReader` shares |
| `explain` | Why a caller holds a permission or not: the gate, then each grant per authority |
| `challenge` | The fresh authentication that would admit a refused caller, which `require` raises as `INSUFFICIENT_AUTHENTICATION` |

A refusal that stronger or fresher authentication alone would lift is a step-up challenge, as OAuth's `insufficient_user_authentication` names one.

```ts
const note = new Policy(pkg, { name: "note", ..., elevated: { delete: { assurance: 2, maxAge: 15 * 60 * 1000 } } });
// AccessError { code: "INSUFFICIENT_AUTHENTICATION", stepUp: { assurance: 2, maxAge: 900000 } }
```

## Authorization

An `Authorization` is one caller bound to each scope it acts in, deciding and changing access.

```ts
const authorization = new Authorization(authorizer, database, (scope) => caller.context(audience, now, scope));
await authorization.create(page, { relationships: [{ relation: "owner", subject: caller.subject }] });
await authorization.grant({ object: page, relation: "editor", subject });
const link = await authorization.link({ object: page, relation: "viewer" });
const offer = await authorization.propose({ relationship: { object: page, relation: "editor" }, recipient: "email:bob@acme.com" });
```

| Verb | Effect |
|---|---|
| `create` | A new object's first relationships, and a new scope's ancestry and owner role |
| `suspend`, `resume` | A scope withholding every permission but administration |
| `grant`, `revoke`, `link` | Relationships, as the grant permission allows |
| `propose`, `accept`, `decline` | A relationship that applies once accepted |
| `createRole`, `updateRole`, `deleteRole` | Roles granting only permissions the caller holds |

## Copies

An access row lives in the database holding its object; every other database deciding in a scope copies the scope chain above it.

| Member | Meaning |
|---|---|
| `DECISION_TABLES` | The rows a decision reads: scopes, roles, role permissions, relationships |
| `Authorizer.replicaOf(scope, held)` | A copy of a scope's decision rows, leaving out those about the types the follower holds |
| `held`, `requireHeld` | The types whose access this database writes, and the refusal of every other write |
| `lag` | How long a copy may go unconfirmed before live decisions fail with `STALE` |

## Storage

Every database holding protected objects includes `ACCESS_TABLES`.
