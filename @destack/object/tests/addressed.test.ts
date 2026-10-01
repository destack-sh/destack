import { Subject } from "@destack/sync";
import { expect, onTestFinished, test } from "@destack/test";
import { accessRelationship, intersection, principal, relation, union } from "@destack/access";
import { journal } from "@destack/audit";
import { eq, isNotNull } from "@destack/db";
import { defineDatabase } from "@destack/db/declare";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { identifier, schema } from "@destack/schema";
import { Bookmark } from "@destack/service/bookmark";
import { ControlLoop } from "@destack/service/control";

import { Outbox } from "@destack/service/outbox";
import { RequestId } from "@destack/service/request";
import type { ServiceContext } from "@destack/service/server";
import { addressed, defineObject, field, HOME, method } from "../src/index.ts";
import { ObjectServer } from "../src/server/index.ts";
import { openSpace, space } from "./fixture/space.ts";
import { testCallKey } from "@destack/service/test";

/** How long copies may take to arrive, in milliseconds: well within the 5 s test timeout. */
const UNTIL_MILLISECONDS = 2000;

/** The space holding the memos. */
const spaceId = identifier("space").parse("space-01996ab0-0000-7000-8000-000000000012");

/** The recipient's home. */
const homeId = identifier("space").parse("space-01996ab0-0000-7000-8000-000000000013");

/** Memos their authors address to a recipient. */
const memo = defineObject({
    name: "memo",
    plural: "memos",
    scope: space,
    fields: {
        author: field.reference(principal.user).caller(),
        recipient: field.subject(principal.user),
        text: field.string(schema.string().min(1)),
        note: field.string().optional().guard({ read: "share" }),
    },
    relations: { reader: { subjects: [principal.user], grantedBy: "share" } },
    permissions: {
        read: union(relation("author"), intersection(relation("recipient"), relation("reader"))),
        write: relation("author"),
        share: relation("author"),
    },
    shareable: { by: "share" },
    addressed: { recipient: "recipient" },
    methods: {
        create: method.create("write", { fields: ["recipient", "text", "note"] }),
        update: method.update("write", { fields: ["text"] }),
        delete: method.delete("write"),
    },
});

/** The database holding both spaces' memos, their access, the journal and the outbox. */
const memoDatabase = defineDatabase({
    name: "main",
    tables: [journal, ...memo.tables],
});

test.each(TEST_DIALECTS)(
    "copy an addressed object into its recipient's home while the recipient may read it, on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, memoDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        await openSpace(storage.database, spaceId);
        await openSpace(storage.database, homeId, "private");
        const server = new ObjectServer({
            objects: { memo },
            database: storage.database,
            context: () => ({
                subjects: [principal.user.reference("universe", "alice")],
                now: Date.now(),
                attributes: {},
            }),
            callKey: testCallKey,
            origin: {
                package: memo.package,
                service: "test",
            },
        });
        const context = {
            scope: spaceId,
            requireAuthentication: () => ({ id: "alice" }),
            bookmark: new Bookmark(),
            observed: new Bookmark(),
        } as unknown as ServiceContext;
        const call = async (name: string, input: object) =>
            (await server.call(
                memo,
                name,
                { spaceId, requestId: RequestId.create(), ...input },
                context,
            )) as { id: string };

        // deliver the copies into the recipient's home, in place of the server's delivery
        const delivery = {
            ...HOME,
            batch: 100,
            accept: async (copies: Parameters<typeof addressed.accept>[1]) => {
                await addressed.accept(server, copies, async () => homeId);
            },
        };
        const stopping = new AbortController();
        const loop = new ControlLoop(
            storage.database,
            [
                ...server.controllers().filter((controller) => controller.name !== HOME.name),
                new Outbox(storage.database).controller(delivery),
            ],
            {
                report: (_controller, _key, error) => {
                    throw error;
                },
            },
        ).run(stopping.signal);
        onTestFinished(async () => {
            stopping.abort();
            await loop;
        });

        // read the copies in the home, waiting until they match
        const bob = principal.user.reference("universe", "bob");
        const copies = async () =>
            (await storage.database
                .select({
                    scope: memo.table.scope,
                    origin: memo.table.origin,
                    text: memo.table.text,
                })
                .from(memo.table)
                .where(isNotNull(memo.table.origin))) as {
                scope: string;
                origin: string;
                text: string;
            }[];
        const until = async (expected: unknown) => {
            const waiting = AbortSignal.timeout(UNTIL_MILLISECONDS);
            await storage.database.log.until(
                async () => JSON.stringify(await copies()) === JSON.stringify(expected),
                waiting,
            );

            return copies();
        };

        // address a memo to bob, copied only once he may read it
        const created = await call("create", {
            recipient: Subject.key(bob),
            text: "Lunch?",
            note: "only for the author",
        });
        const hidden = await until([]);
        const grant = await call("grant", { id: created.id, relation: "reader", subject: bob });
        const shared = await until([{ scope: homeId, origin: spaceId, text: "Lunch?" }]);

        // refresh the copy as the memo changes, and withdraw it once bob may no longer read it
        await call("update", { id: created.id, text: "Lunch at noon?" });
        const updated = await until([{ scope: homeId, origin: spaceId, text: "Lunch at noon?" }]);
        await call("revoke", { id: created.id, relationshipId: grant.id });
        const revoked = await until([]);

        // copy it again once shared again, and withdraw it once the memo is deleted
        await call("grant", { id: created.id, relation: "reader", subject: bob });
        const regranted = await until([{ scope: homeId, origin: spaceId, text: "Lunch at noon?" }]);
        await call("delete", { id: created.id });
        const deleted = await until([]);

        // forget the deleted memo's relationships
        const relationships = async () =>
            (
                await storage.database
                    .select({ id: accessRelationship.id })
                    .from(accessRelationship)
                    .where(eq(accessRelationship.objectId, created.id))
            ).length;
        await storage.database.log.until(
            async () => (await relationships()) === 0,
            AbortSignal.timeout(UNTIL_MILLISECONDS),
        );
        const forgotten = await relationships();
        expect([forgotten, hidden, shared, updated, revoked, regranted, deleted]).toEqual([
            0,
            [],
            [{ scope: homeId, origin: spaceId, text: "Lunch?" }],
            [{ scope: homeId, origin: spaceId, text: "Lunch at noon?" }],
            [],
            [{ scope: homeId, origin: spaceId, text: "Lunch at noon?" }],
            [],
        ]);
    },
);
