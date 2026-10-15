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
    type Permission,
    principal,
} from "@destack/access";
import { Clock, schema, Identifier } from "@destack/schema";
import { ObjectStore } from "@destack/object/server";
import * as object from "../src/object/index.ts";
import accountPackage from "../src/package.ts";
import { Package } from "@destack/package";
import { testEvents } from "@destack/object/test";

import { testCallKey } from "@destack/runtime/test";
import { SIGN_UP } from "../src/test/index.ts";
import { AccountFixture } from "../src/test/index.ts";

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
            objects: [
                object.account,
                object.organisation,
                object.member,
                object.group,
                object.role,
                object.relationship,
            ],
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
                () => ({ subjects: [owner], now: time, attributes: {} }),
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
        const asOwner = (): AccessContext => ({ subjects: [owner], now: time, attributes: {} });
        const binding = await Authorization.run(authorizer, database, asOwner, async (granting) => {
            await granting.grant({ object: onAccount, relation: "editor", subject: member });
            await granting.grant({ object: readers, relation: "member", subject: reader });
            const role = await granting.createRole(onAccount, {
                name: "reader",
                description: "Reads account metadata",
                permissions: [read],
            });
            const bound = await granting.grant({
                object: onAccount,
                role: role.id,
                subject: { ...readers, relation: "member" },
            });
            await granting.grant({ object: onAccount, role: role.id, subject: software });

            return bound;
        });

        // decide reading and updating the account as each subject
        const decide = async (subject: Subject, permission: Permission, target: string) => {
            const context: Caller = {
                subject,
                subjects: [subject],
                credential: {
                    kind: "test",
                    id: "test-1",
                    audience: accountPackage.id,
                    verifiedAt: time,
                    expiresAt: time + 60_000,
                },
            };
            Caller.requireCurrent(context, accountPackage.id, time, target);
            const access = await authorizer.resolve(
                Snapshot.live(database),
                target,
                AccessContext.of(context, time),
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

        // keep the member reading once the group's role binding is gone
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
    // keep a person whose row names their home, without any relationship to it
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
            objects: [user],
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
        const decide = async (space: string, permission: "follow" | "contact") => {
            const subject = principal.space.reference(Scope.universe.id, space);
            const context = { subjects: [subject], now: time, attributes: {} };
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
            await decide(home, "contact"),
            await decide(other, "contact"),
        ]).toEqual([true, false, true, false]);
    } finally {
        await opened.close();
    }
});
