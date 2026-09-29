import { AuditOutbox, auditOutboxTables } from "@destack/audit/outbox";
import { expect, onTestFinished, test } from "@destack/test";
import {
    ACCESS_TABLES,
    accessRelationship,
    AccessFollower,
    Authorizer,
    GLOBAL_SCOPE,
    none,
    principal,
    relation,
    Relationship,
    type AccessRelay,
} from "@destack/access";
import { AuditRecorder } from "@destack/audit";
import type { DatabaseConnection } from "@destack/db";
import { copyOwner, copyRole, copyScope } from "@destack/access/test";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { identifier, schema } from "@destack/schema";
import { Bookmark } from "@destack/service/bookmark";
import { Journal } from "@destack/service/database";
import { RequestId } from "@destack/service/request";
import type { ServiceContext } from "@destack/service/server";
import { Feed, Replica } from "@destack/sync";
import { v7 } from "uuid";
import { defineObject, field, method } from "../src/index.ts";
import { ObjectServer } from "../src/server/index.ts";
import { request } from "./schema.ts";

/** The account holding the space. */
const accountId = identifier("account").parse("account-01996ab0-0000-7000-8000-000000000001");

/** The empty space the documents go into. */
const spaceId = identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002");

/** Another space of the account, which no restriction here reveals. */
const otherSpaceId = identifier("space").parse("space-01996ab0-0000-7000-8000-000000000003");

/** Accounts, whose members roles bind to. */
const account = defineObject({
    name: "account",
    plural: "accounts",
    scope: "global",
    isScope: true,
    fields: {},
    relations: { member: { subjects: [principal.user] } },
    permissions: { read: relation("member") },
});

/** Spaces within accounts. */
const space = defineObject({
    name: "space",
    plural: "spaces",
    scope: account,
    isScope: true,
    fields: {},
    permissions: { read: none() },
});

/** Documents, which only roles grant. */
const document = defineObject({
    name: "document",
    plural: "documents",
    scope: space,
    fields: { title: field.string(schema.string().min(1)) },
    permissions: { read: none(), write: none() },
    methods: {
        list: method.list("read"),
        create: method.create("write"),
    },
});

test.each(TEST_DIALECTS)(
    "decide calls over a relayed copy of access as the holder does, through the account's members and restricted credentials, on %s",
    async (dialect) => {
        const home = await TestDatabase.create(dialect, ACCESS_TABLES, { isMigrated: true });
        const workload = await TestDatabase.create(
            dialect,
            [...auditOutboxTables, ...document.tables, request],
            {
                isMigrated: true,
            },
        );
        onTestFinished(async () => {
            await Promise.all([home.close(), workload.close()]);
        });

        // hold an account owned by alice, a member role carol holds through the account, and its empty space at home
        const alice = principal.user.reference("global", "alice");
        const carol = principal.user.reference("global", "carol");
        const accountObject = account.reference(GLOBAL_SCOPE, accountId);
        await copyScope(home.database, accountObject);
        await copyScope(home.database, space.reference(accountId, spaceId));
        await copyOwner(home.database, accountObject, alice);
        await copyRole(
            home.database,
            accountObject,
            {
                name: "member",
                description: "Writes documents in the account's spaces",
                permissions: [
                    space.permission("read"),
                    document.permission("read"),
                    document.permission("write"),
                ],
            },
            { ...accountObject, relation: "member" },
        );
        await home.database.insert(accessRelationship).values(
            Relationship.encode(
                {
                    id: identifier("relationship").parse(`relationship-${v7()}`),
                    object: accountObject,
                    relation: "member",
                    subject: carol,
                    createdAt: 0,
                    expiresAt: null,
                },
                accountId,
            ),
        );

        // serve the documents over the workload's database to the user each call names, restricted to documents in one space
        let caller: { user: string; restricted?: string } = { user: "alice" };
        const server = new ObjectServer({
            objects: { document },
            database: workload.database,
            context: () => ({
                subject: principal.user.reference("global", caller.user),
                subjects: [principal.user.reference("global", caller.user)],
                ...(caller.restricted === undefined
                    ? {}
                    : {
                          permissions: [
                              document.permission("read"),
                              document.permission("write"),
                          ].map((permission) => ({ ...permission, scope: caller.restricted! })),
                      }),
                now: Date.now(),
                attributes: {},
            }),
            journal: new Journal(request),
            audit: AuditRecorder.service(new AuditOutbox(workload.database), {
                package: document.package,
                service: "test",
            }),
        });

        // copy the space's chain into the workload's database from a relay of the home database
        const feed = new Feed(home.database, ACCESS_TABLES);
        const relay: AccessRelay = {
            scope: spaceId,
            watch: (watched, signal) =>
                feed.subscribe(
                    Authorizer.replicaOf(watched.scope, watched.held).queries,
                    watched.after,
                    signal,
                ),
        };
        const follower = new AccessFollower(workload.database, server.authorizer.held, relay);
        const controller = new AbortController();
        const following: Promise<void>[] = [];
        onTestFinished(async () => {
            controller.abort();
            await Promise.allSettled(following);
        });
        const head = await home.database.log.position();
        for (const scope of [spaceId, accountId]) {
            following.push(follower.follow(scope, controller.signal));
            await Replica.reach(workload.database, scope, head, controller.signal);
        }
        expect(await follower.list()).toEqual([spaceId, accountId]);

        // create through credentials restricted to the space or another one, and unrestricted, then list as alice
        const context = {
            scope: spaceId,
            requireCaller: () => ({ id: caller.user }),
            bookmark: new Bookmark(),
            observed: new Bookmark(),
        } as unknown as ServiceContext;
        const create = async (user: string, title: string, restricted?: string) => {
            caller = restricted === undefined ? { user } : { user, restricted };

            return server
                .call(
                    document,
                    "create",
                    { spaceId, requestId: RequestId.create(), title },
                    context,
                )
                .then(
                    (created) => (created as { title: string }).title,
                    (error: { code: string; message: string }) => [error.code, error.message],
                );
        };
        const outcomes = [
            await create("alice", "Plan", spaceId),
            await create("alice", "Elsewhere", otherSpaceId),
            await create("dave", "Leak", spaceId),
            await create("dave", "Leak"),
            await create("carol", "Budget"),
        ];
        caller = { user: "alice" };
        const listed = (await server.query(document, "list", { spaceId }, context)) as {
            items: { title: string }[];
        };

        // admit the owner through a credential naming the empty space and the account's member, and hide the space from the rest
        const hidden = ["NOT_FOUND", `no scope ${spaceId}`];
        expect(outcomes).toEqual(["Plan", hidden, hidden, hidden, "Budget"]);
        expect(listed.items.map((item) => item.title).sort()).toEqual(["Budget", "Plan"]);
    },
);
