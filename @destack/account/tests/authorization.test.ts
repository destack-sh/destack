import { user } from "../src/object/user.ts";
import { Snapshot } from "@destack/db/log";
import { serviceAccount } from "../src/object/service.ts";
import { copyScope } from "@destack/access/test";
import { expect, test } from "@destack/test";
import { eq } from "@destack/db";
import {
    accessRelationship,
    principal,
    type AccessContext,
    type Permission,
    type Subject,
    Authorization,
} from "@destack/access";
import { identifier } from "@destack/schema";
import { Journal } from "@destack/service/database";
import { Caller } from "@destack/service/authentication";
import { AuditRecorder } from "@destack/audit";
import { AuditOutbox } from "@destack/audit/outbox";
import { ObjectServer } from "@destack/object/server";
import { accountJournal } from "../src/stack/index.ts";
import * as object from "../src/object/index.ts";
import { accountPackage } from "../src/audit/index.ts";
import { openAccountDatabase } from "./database.ts";
import { id } from "./fixture.ts";

/** Let an account's root own it, its members read it, and groups and service accounts read it through a role. */
test("decide account access for its root, members, group members and service accounts", async () => {
    const opened = await openAccountDatabase();
    const database = opened.database;
    const time = Date.now();
    const record = { createdAt: time, updatedAt: time };
    const owner = principal.user.reference("universe", id("user"));
    const member = principal.user.reference("universe", id("user"));
    const reader = principal.user.reference("universe", id("user"));
    const accountId = id("account");
    const otherAccountId = id("account");
    const softwareId = id("service-account");
    const software = serviceAccount.reference(accountId, softwareId);
    const read = object.account.permission("read");
    const update = object.account.permission("update");

    try {
        // decide through the account service's own policies
        const { authorizer } = new ObjectServer({
            objects: {
                account: object.account,
                organisation: object.organisation,
                group: object.group,
                role: object.role,
                relationship: object.relationship,
                serviceAccount: object.serviceAccount,
                serviceToken: object.serviceToken,
            },
            database,
            context: () => ({ subjects: [], now: time, attributes: {} }),
            audit: () =>
                new AuditRecorder(
                    {
                        actor: { type: "system", name: "test" },
                        delegation: [],
                        package: accountPackage,
                        service: "account",
                        scope: "universe",
                    },
                    new AuditOutbox(database),
                ),
            journal: new Journal(accountJournal),
        });

        // create two accounts under the owner's root that own their scopes
        await database.insert(user.table).values([
            {
                ...record,
                id: identifier("user").parse(owner.id),
                name: "Owner",
                email: "owner@example.com",
            },
            {
                ...record,
                id: identifier("user").parse(member.id),
                name: "Member",
                email: "member@example.com",
            },
            {
                ...record,
                id: identifier("user").parse(reader.id),
                name: "Reader",
                email: "reader@example.com",
            },
        ]);
        await copyScope(database, owner);
        for (const [accountKey, handle] of [
            [accountId, "personal"],
            [otherAccountId, "other"],
        ] as const) {
            await database.insert(object.account.table).values({
                ...record,
                id: accountKey,
                handle,
                name: handle,
                defaultResidency: "eu",
                scope: owner.id,
            });
            const reference = object.account.reference(owner.id, accountKey);
            await new Authorization(authorizer, database, () => ({
                subjects: [owner],
                now: time,
                attributes: {},
            })).create(reference, {
                relationships: [{ relation: "root", subject: owner }],
                owner: { ...reference, relation: "root" },
            });
        }

        // as the owner, add a member, a group of readers, and a service account reading through a role
        const asOwner = new Authorization(authorizer, database, (): AccessContext => ({
            subjects: [owner],
            now: time,
            attributes: {},
        }));
        const onAccount = object.account.reference(owner.id, accountId);
        await asOwner.grant({ object: onAccount, relation: "member", subject: member });
        const groupId = id("group");
        await database
            .insert(object.group.table)
            .values({ ...record, id: groupId, scope: accountId, name: "readers" });
        const readers = object.group.reference(accountId, groupId);
        await asOwner.grant({ object: readers, relation: "member", subject: reader });
        const role = await asOwner.createRole(onAccount, {
            name: "reader",
            description: "Reads account metadata",
            permissions: [read],
        });
        const binding = await asOwner.grant({
            object: onAccount,
            role: role.id,
            subject: { ...readers, relation: "member" },
        });
        await database
            .insert(serviceAccount.table)
            .values({ ...record, id: softwareId, scope: accountId, name: "automation" });
        await asOwner.grant({ object: onAccount, role: role.id, subject: software });

        // decide reading and updating the account, and reading the other account, as each subject
        const decide = async (subject: Subject, permission: Permission, target: string) => {
            const context = new Caller({
                credential: { kind: "test", id: "test-1" },
                audience: accountPackage.id,
                subject,
                subjects: [subject],
                verifiedAt: time,
                expiresAt: time + 60_000,
            }).context(accountPackage.id, time, target);
            const access = await authorizer.resolve(Snapshot.live(database), target, context);
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

        // keep the member reading once the group's role binding is gone, and stop the group's reader
        await database
            .delete(accessRelationship)
            .where(eq(accessRelationship.id, identifier("relationship").parse(binding.id)));
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
