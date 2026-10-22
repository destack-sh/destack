import { Scope } from "@destack/sync";
import { user } from "../src/object/user.ts";
import { Account } from "../src/object/account.ts";
import type { Subject } from "@destack/sync";
import { Snapshot, eq } from "@destack/db";
import { AccessFixture } from "@destack/access/test";
import { expect, test } from "@destack/test";
import {
    AccessContext,
    accessRelationship,
    Authorization,
    Caller,
    Credential,
    type Permission,
    principal,
    Relationship,
    Role,
} from "@destack/access";
import { schema, Identifier } from "@destack/schema";
import { relationship, role } from "@destack/object";
import { ObjectStore } from "@destack/object/server";
import * as object from "../src/object/index.ts";
import accountPackage from "../src/package.ts";
import { Package } from "@destack/package";
import { testEvents, serversOf } from "@destack/object/test";

import { testCallKey } from "@destack/object/test";
import { SIGN_UP } from "../src/test/index.ts";
import { AccountFixture } from "../src/test/index.ts";
import { Clock } from "@destack/environment";

/** Make an account's root its owner, and let members, groups and installations read it. */
test("decide account access for its root, members, group members and installations", async () => {
    const opened = await AccountFixture.storage();
    const database = opened.database;
    const time = Date.now();
    const record = { createdAt: time, updatedAt: time };
    const owner = principal.user.reference("universe", Identifier.create("user"));
    const member = principal.user.reference("universe", Identifier.create("user"));
    const reader = principal.user.reference("universe", Identifier.create("user"));
    const accountId = Identifier.create("account");
    const otherAccountId = Identifier.create("account");
    const software = principal.installation.reference(
        Identifier.create("space"),
        Identifier.create("installation"),
    );
    const read = object.account.permission("read");
    const update = object.account.permission("update");

    try {
        // decide through the account package's policies
        const { authorizer } = new ObjectStore({
            chain: async () => undefined,
            clock: Clock.system,
            principal: Scope.universe,
            servers: serversOf([
                object.account,
                object.organisation,
                object.member,
                object.group,
                role,
                relationship,
            ]),
            database,
            claims: { object: object.claim },
            callKey: testCallKey,
            events: await testEvents(database),
            origin: { package: Package.of(accountPackage) },
        });

        // create two accounts under the owner's root that are the owners of their scopes
        await database.insert(user.table).values([
            {
                ...record,
                ...SIGN_UP,
                id: schema.identifier("user").parse(owner.id),
                homeId: Account.home(Account.personal(owner.id)),
                name: "Owner",
                email: "owner@example.com",
            },
            {
                ...record,
                ...SIGN_UP,
                id: schema.identifier("user").parse(member.id),
                homeId: Account.home(Account.personal(member.id)),
                name: "Member",
                email: "member@example.com",
            },
            {
                ...record,
                ...SIGN_UP,
                id: schema.identifier("user").parse(reader.id),
                homeId: Account.home(Account.personal(reader.id)),
                name: "Reader",
                email: "reader@example.com",
            },
        ]);
        await new AccessFixture(database).replicateScope(owner);
        for (const [accountKey, handle] of [
            [accountId, "personal"],
            [otherAccountId, "other"],
        ] as const) {
            await database.insert(object.account.table).values({
                ...record,
                id: accountKey,
                handle,
                name: handle,
                residencyId: "eu",
                scope: owner.id,
            });
            const reference = object.account.reference(owner.id, accountKey);
            await Authorization.run(
                authorizer,
                database,
                () => AccessContext.of(Caller.of(owner), time),
                (authorization) =>
                    authorization.create(reference, {
                        relationships: [{ relation: "owner", subject: owner }],
                        owner: { ...reference, relation: "owner" },
                    }),
            );
        }

        // add a member, a group of readers and an installation reading through a role as the owner
        const onAccount = object.account.reference(owner.id, accountId);
        const groupId = Identifier.create("group");
        await database
            .insert(object.group.table)
            .values({ ...record, id: groupId, scope: accountId, name: "readers" });
        const readers = object.group.reference(accountId, groupId);
        const asOwner = (): AccessContext => AccessContext.of(Caller.of(owner), time);
        const binding = await Authorization.run(authorizer, database, asOwner, async (granting) => {
            await Relationship.grant(
                { object: onAccount, relation: "editor", subject: member },
                granting,
            );
            await Relationship.grant(
                { object: readers, relation: "member", subject: reader },
                granting,
            );
            const readingRole = await Role.create(
                onAccount,
                {
                    name: "reader",
                    description: "Reads account metadata",
                    permissions: [read],
                },
                granting,
            );
            const bound = await Relationship.grant(
                {
                    object: onAccount,
                    role: readingRole.id,
                    subject: { ...readers, relation: "member" },
                },
                granting,
            );
            await Relationship.grant(
                { object: onAccount, role: readingRole.id, subject: software },
                granting,
            );

            return bound;
        });

        // decide reading and updating the account as each subject
        const decide = async (subject: Subject, permission: Permission, target: string) => {
            const credential = {
                kind: "test",
                id: "test-1",
                audience: accountPackage.id,
                verifiedAt: time,
                expiresAt: time + 60_000,
            };
            Credential.requireCurrent(credential, accountPackage.id, time, target);
            const access = await authorizer.resolve(
                Snapshot.live(database),
                target,
                AccessContext.of(Caller.of(subject), time, { credential }),
            );
            const reference = object.account.reference(owner.id, target);
            const decision = await authorizer.check(
                Snapshot.live(database),
                permission,
                reference,
                access,
            );

            return decision.isAllowed ? "allowed" : "denied";
        };
        const matrix = async () =>
            Promise.all(
                [owner, member, reader, software].map(async (subject) => [
                    await decide(subject, read, accountId),
                    await decide(subject, update, accountId),
                    await decide(subject, read, otherAccountId),
                ]),
            );
        expect(await matrix()).toEqual([
            ["allowed", "allowed", "allowed"],
            ["allowed", "denied", "denied"],
            ["allowed", "denied", "denied"],
            ["allowed", "denied", "denied"],
        ]);

        // let the member read once the group's role binding is gone
        await database
            .delete(accessRelationship)
            .where(eq(accessRelationship.id, schema.identifier("relationship").parse(binding.id)));
        expect((await matrix()).map(([reading]) => reading)).toEqual([
            "allowed",
            "allowed",
            "denied",
            "allowed",
        ]);
    } finally {
        await opened.close();
    }
});

test("let a person's home follow on their behalf and read their contact from the user row alone, and no other space", async () => {
    // store a person whose row has their home, without any relationship to it
    const opened = await AccountFixture.storage();
    const database = opened.database;
    const time = Date.now();
    const person = principal.user.reference("universe", Identifier.create("user"));
    const home = Account.home(Account.personal(person.id));
    try {
        const { authorizer } = new ObjectStore({
            chain: async () => undefined,
            clock: Clock.system,
            principal: Scope.universe,
            servers: serversOf([user]),
            database,
            callKey: testCallKey,
            events: await testEvents(database),
            origin: { package: Package.of(accountPackage) },
        });
        await database.insert(user.table).values({
            ...SIGN_UP,
            createdAt: time,
            updatedAt: time,
            id: schema.identifier("user").parse(person.id),
            homeId: home,
            name: "Person",
            email: "person@example.com",
        });

        // decide `follow` and `contact` on the person as their home and as another space
        const decide = async (space: string, permission: "follow" | "recipient") => {
            const subject = principal.space.reference(Scope.universe.id, space);
            const context = AccessContext.of(Caller.of(subject), time);
            const access = await authorizer.resolve(Snapshot.live(database), "universe", context);
            const decision = await authorizer.check(
                Snapshot.live(database),
                user.permission(permission),
                person,
                access,
            );

            return decision.isAllowed;
        };
        const other = Identifier.create("space");
        expect([
            await decide(home, "follow"),
            await decide(other, "follow"),
            await decide(home, "recipient"),
            await decide(other, "recipient"),
        ]).toEqual([true, false, true, false]);
    } finally {
        await opened.close();
    }
});
