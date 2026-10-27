# @destack/access

`Policy` is a SpiceDB schema definition (`relation`, `permission`, `union` as `+`, `intersection` as `&`, `exclusion` as `-`, `through` as the `->` arrow, `members` as a subject set, `resource` and `context` as caveats), `Authorizer` is SpiceDB's `CheckPermission`, `LookupSubjects` and `LookupResources` compiled to SQL over the object's own database, and `elevated` is RFC 9470 step-up authentication.

```ts
// owner + editor + parent->read
read: union(relation("owner"), relation("editor"), through("parent", "read"));
// user | folder#read
const reader = { subjects: [principal.user, folder.members("read")] };
edit: intersection(relation("editor"), resource({ team: sql.placeholder("team") })); // a caveat
await authorizer.check(snapshot, page.permission("edit"), target, access); // CheckPermission
// LookupSubjects
await authorizer.subjects(snapshot, page.permission("read"), target, type, Date.now(), {
    limit: 50,
});
// LookupResources as SQL
await database
    .select()
    .from(pages)
    .where(authorizer.where(page.permission("read"), access));
await Relationship.grant({ object: page, relation: "editor", subject }, authorization); // WriteRelationships
```

## Policies

A `Policy` declares an object type's relations and permissions, `self()` allows the principal an object represents, `onDelete: "cascade"` removes rows with their subject, and `elevated` asks for a recent stronger sign-in.

```ts
export const note = new Policy(import.meta.destack.package, {
    name: "note",
    relations: {
        owner: { subjects: [principal.user], grantedBy: null },
        editor: { subjects: [principal.user, group.members("member")] },
        parent: { subjects: ["note"], grantedBy: null, onDelete: "cascade" },
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

## Callers

A `Caller` is who calls, a `Credential` is what its request presented, a verifier answers both as an `Authentication`, and an `AccessContext` is what the authorizer decides on.

```ts
const { caller, credential } = await verifier.verify(token, now);
const context = AccessContext.of(caller, now, { credential, linkSecrets });
const own = AccessContext.of(Caller.of(space), now);
const applying = AccessContext.of(caller, now, { approvers }); // lifts grants their delegator approved, roles by others' count
AccessContext.assurance(context);
await Grant.permissions(snapshot, applying); // what its chain's live grants give it, read from their rows
Caller.principal(caller);
```

## Decisions

An `Authorizer` resolves a caller's `Access` in a scope, `require` throws an `AccessError` with the first permission the caller lacks, and each related object's permission is decided with the chain of the scope it lives in, as a direct check of it would.

```ts
const authorizer = new Authorizer(
    [note],
    [{ policy: note, table: notes, id: "id", scope: "scope", attributes: {}, relations: {} }],
);
const snapshot = Snapshot.live(database);
const access = await authorizer.resolve(snapshot, spaceId, context);
await authorizer.require(snapshot, [note.permission("read")], target, access);
// the gate or failing grants
await authorizer.explain(snapshot, note.permission("delete"), target, access);
```

## Changes

An `Authorization` is one caller's session in a transaction: it records scopes, and `Relationship`, `Invitation`, `Role` and `Grant` change access through it, each requiring the caller's permission.

```ts
await Authorization.run(
    authorizer,
    database,
    () => context,
    async (authorization) => {
        // a scope created here, then born at its home below its account
        await authorization.create(space, { owner: alice });
        await authorization.receive(space, [accountId], { owner: alice });
        await authorization.remove(space);
        await Relationship.revoke(page, relationshipId, authorization);
        // a share link
        const { id, secret } = await Relationship.link(
            { object: page, relation: "viewer" },
            authorization,
        );
        const offer = await Invitation.create(
            { relationship: { object: page, relation: "editor", subject: contact } },
            authorization,
        );
        await Invitation.accept(page, offer.id, authorization);
    },
);
```

## Roles

`Role.create` defines a role granting permissions the caller has, `Role.upsert` creates or updates a declared role by name, `Role.assign` binds a role to one subject, and `Role.OWNER` is the one owner role of every scope.

```ts
const role = await Role.create(
    space,
    {
        name: "reviewer",
        description: "Read and share every note",
        permissions: [note.permission("read"), note.permission("share")],
    },
    authorization,
);
await Role.assign(space, role.id, carol, authorization);
const merging = { ...branch.permission("merge"), approvals: 2 }; // a use waits for two others' approvals
await authorizer.requiredApprovals(snapshot, merging, access, allows); // 2 for a denied caller its roles give it on approvals
await Relationship.grant({ object: space, role: Role.OWNER, subject: bob }, authorization);
```

## Grants

A `Grant` lets a delegate act for its delegator with what both allow, one grant per pair, its RFC 9396 permissions used at once or each once approved; a caller presents its RFC 8693 actor chain with each link's grant by reference, read where decided.

```ts
const grant = await Grant.create(
    aliceScope,
    {
        delegate: installation,
        permissions: [
            { ...note.permission("read"), scope: spaceId, method: "get" },
            { ...message.permission("send"), mode: "approval" }, // every scope alice reaches, each send approved
        ],
    },
    authorization,
); // replaces alice's earlier grant to the installation
const caller = await Grant.act(snapshot, { delegator: alice, delegate: installation }, now);
// { subject: alice, actors: [installation], grants: [{ id, scope }] }, a sub-agent adding its own link
await Grant.permissions(snapshot, context); // what every live grant of the chain gives
await Grant.revoke(aliceScope, grant.id, authorization);
```

## Requests

A `PermissionRequest` asks each delegator for a `PermissionSet`: its installation's space at install and each user on use, for a package or one of its capabilities (GitHub App installation and user permissions).

```ts
definePackage({
    permissions: {
        installation: {
            space: [note.permission("write")],
            universe: [user.permission("recipient")],
        },
        user: { home: [notification.permission("read")] },
    },
});
PermissionSet.grant(set, { space, home: [userId, homeId], account }); // the grant permissions in a context
```

## Tokens

`TokenIssuer` issues access tokens carrying an RFC 8693 actor chain and an RFC 9396 grant, `TokenVerifier` verifies them, and `Exchange` asks a call's callee to act for its caller.

```ts
await issuer.issue({ caller, credential }, now); // a caller's verified authentication
client.notes.list({}, { context: { exchange: Exchange.of(served) } });
```
