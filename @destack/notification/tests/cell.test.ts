import { principal } from "@destack/access";
import { type DatabaseConnection, defineDatabase, eq } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { address, user } from "@destack/account/object";
import { ObjectServer } from "@destack/object/server";
import type { CellDirectory } from "@destack/service/workload";
import { PackageId } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import { aligned, Identifier, present, schema } from "@destack/schema";
import { defineService } from "@destack/service";
import { Authentication } from "@destack/service/authentication";
import { createClient } from "@destack/service/client";
import { Health } from "@destack/service/health";
import { RequestId } from "@destack/service/request";
import { Server, type ServiceImplementation } from "@destack/service/server";
import { testCallKey } from "@destack/service/test";
import { until } from "@destack/service/timer";
import { journal } from "@destack/audit/stack";
import { type Publisher, Scope, Subject, type Subscription, type Uplink } from "@destack/sync";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { activity, announcement, notification, subscription } from "../src/index.ts";
import { serveNotifications, serveInbox } from "../src/server/index.ts";
import { inboxService } from "../src/service/index.ts";
import { inboxDatabase } from "../src/stack/index.ts";
import { document, notifications } from "./fixture/document.ts";
import { INBOX_BUILD, INBOX_CATALOGS, NotificationFixture } from "../src/test/index.ts";
import { actors, NOTES_BUILD } from "./fixture/space.ts";

/** The documents' service, keeping their activities. */
const documentsService = defineService("documents", {
    objects: { document, subscription, activity, announcement },
});

/** The documents' database. */
const documentsDatabase = defineDatabase({
    name: "main",
    tables: [
        journal,
        ...[document, subscription, activity, announcement].flatMap((object) => object.tables),
    ],
});

/** The installation serving the documents. */
const DOCUMENTS = schema
    .identifier("installation")
    .parse("installation-019f5530-8000-7000-8000-0000000000d1");

/** The inbox installation of each of alice's homes. */
const INBOX = schema
    .identifier("installation")
    .parse("installation-019f5530-8000-7000-8000-0000000000d2");

/** An uplink streaming nothing until stopped and receiving no changes, as a cell with no chain to copy. */
const idle: Uplink = {
    async *stream(_request, signal) {
        await until(signal);
        yield* [];
    },
    receive: () => Promise.reject(new Error("the idle cell receives no changes")),
};

test(
    "project a mention into the recipient's home within seconds, follow the recipient to a new home of their account, dropping their rows from the old one, and refuse another space's installation",
    { timeout: 30_000 },
    async () => {
        // keep the account service's users and addresses, the documents' space and alice's two homes in databases of their own
        const accounts = await open([...user.tables, ...address.tables]);
        const source = await open(documentsDatabase);
        const homes = [await open(inboxDatabase), await open(inboxDatabase)];
        const spaceId = await NotificationFixture.openSpace(source, Date.now());
        const first = await NotificationFixture.openSpace(aligned(homes, 0), Date.now());
        const second = await NotificationFixture.openSpace(aligned(homes, 1), Date.now());
        const alice = Subject.key(actors.alice);
        const copies = [accounts, ...homes];
        await settle(copies, alice, first);

        // serve the documents as their installation, recording whom their activities address in the recipients' user scopes
        const documents = serve(
            ObjectServer.serve(documentsService, {
                database: source,
                callKey: testCallKey,
                handled: Object.values(serveNotifications({ notifications })),
                installation: {
                    id: DOCUMENTS,
                    scope: spaceId,
                    build: NOTES_BUILD,
                    publisher: idle,
                    publisherAt: () => idle,
                    directory: installationDirectory(accounts, copies, spaceId, DOCUMENTS),
                },
            }),
            spaceId,
        );
        const followed: Subscription[] = [];
        const streams: [string, "open" | "closed"][] = [];
        const follow = (caller: string): Publisher => {
            const client = createClient(documentsService, documents.endpoint(caller));

            return {
                async *stream(request: Subscription, signal: AbortSignal) {
                    followed.push(request);
                    streams.push([caller, "open"]);
                    try {
                        yield* await client.replica.stream(request, { signal });
                    } finally {
                        streams.push([caller, "closed"]);
                    }
                },
            };
        };
        const following = (caller: string) =>
            streams.filter(([entry, event]) => entry === caller && event === "open").length -
            streams.filter(([entry, event]) => entry === caller && event === "closed").length;

        // serve alice's inbox in each home as the home's installation, following the documents at their peer address
        const callers = [first, second].map((home) => `installation:${home}:${INBOX}`);
        const copied: string[] = [];
        for (const [index, scope] of [first, second].entries()) {
            const caller = aligned(callers, index);
            serve(
                ObjectServer.serve(inboxService, {
                    database: aligned(homes, index),
                    callKey: testCallKey,
                    handled: Object.values(serveInbox({ catalogs: INBOX_CATALOGS })),
                    installation: {
                        id: INBOX,
                        scope,
                        build: INBOX_BUILD,
                        publisher: {
                            stream: (request, signal) => {
                                copied.push(request.shape);

                                return idle.stream(request, signal);
                            },
                            receive: (mutation) => idle.receive(mutation),
                        },
                        publisherAt: () => follow(caller),
                        directory: installationDirectory(accounts, copies, scope, INBOX),
                    },
                }),
                scope,
            );
        }

        // let bob mention alice on a document she views, once her inboxes copy their chains without any source
        await expect.poll(() => copied.length > 0).toBe(true);
        expect(followed).toEqual([]);
        const bob = createClient(documentsService, documents.endpoint(`user:${actors.bob.id}`));
        const mention = async (title: string, text: string) => {
            const created = await bob.document.create({
                spaceId,
                requestId: RequestId.create(),
                title,
            });
            await bob.document.grant({
                spaceId,
                requestId: RequestId.create(),
                id: created.id,
                relation: "viewer",
                subject: actors.alice,
            });
            await bob.document.remark({
                spaceId,
                requestId: RequestId.create(),
                id: created.id,
                text,
                mentions: [actors.alice],
            });
        };
        const inbox = async (index: number) =>
            (await aligned(homes, index).select().from(notification.table)).map((row) => [
                row.recipient,
                row.space,
                row.content.body,
            ]);
        await mention("Launch plan", "@alice, a question");

        // receive the notification in the first home within seconds of the mention
        await expect
            .poll(() => inbox(0), { timeout: 5000 })
            .toEqual([[alice, spaceId, "@alice, a question"]]);

        // move alice's home: the first home stops following the documents and drops her rows, and the second receives the next mention
        await settle(copies, alice, second);
        await expect.poll(() => callers.map((caller) => following(caller))).toEqual([0, 1]);
        await mention("Budget", "@alice, the budget");
        await expect
            .poll(() => inbox(1), { timeout: 5000 })
            .toEqual([
                [alice, spaceId, "@alice, a question"],
                [alice, spaceId, "@alice, the budget"],
            ]);
        await expect.poll(() => inbox(0), { timeout: 5000 }).toEqual([]);

        // refuse the projection to an installation of another space
        const stranger = follow(`installation:${spaceId}:${DOCUMENTS}`);
        const projection = present(followed[0], "the inbox's projection");
        const refused = await refusal(
            (async () => {
                for await (const _page of stranger.stream(projection, AbortSignal.timeout(5000))) {
                    return;
                }
            })(),
        );
        expect(refused).toEqual([
            "FORBIDDEN",
            `only ${first} follows the rows its residents receive`,
        ]);
    },
);

/** Open a migrated SQLite database, closed after the test. */
async function open(
    database: Parameters<typeof TestDatabase.create>[1],
): Promise<DatabaseConnection> {
    const opened = await TestDatabase.create("sqlite", database, { isMigrated: true });
    onTestFinished(() => opened.close());

    return opened.database;
}

/** Keep alice's user row living in a home, as the account service's database and her homes' chain copies keep it. */
async function settle(
    copies: readonly DatabaseConnection[],
    recipient: string,
    home: Identifier<"space">,
): Promise<void> {
    const id = schema.identifier("user").parse(Subject.read(recipient).id);
    const now = Date.now();
    for (const database of copies) {
        await database
            .insert(user.table)
            .values({
                id,
                scope: Scope.universe.id,
                name: "alice",
                email: "alice@example.com",
                home,
                createdAt: now,
                updatedAt: now,
            })
            .onConflictDoUpdate({ target: user.table.id, set: { home, updatedAt: now } });
    }
}

/** Confirm homes, read no locales and record addresses as an installation in a scope does through its cell. */
function installationDirectory(
    accounts: DatabaseConnection,
    copies: readonly DatabaseConnection[],
    scope: Identifier<"space">,
    installation: Identifier<"installation">,
): CellDirectory {
    return {
        isHome: async (subject, space) => {
            // compare the home of the user the subject is
            const id = schema.identifier("user").parse(Subject.read(subject).id);
            const [person] = await accounts
                .select({ home: user.table.home })
                .from(user.table)
                .where(eq(user.table.id, id));

            return person?.home === space;
        },
        locale: async () => undefined,
        address: async (recipient) => {
            // keep the address in the recipient's user scope and its chain copies
            const id = schema.identifier("user").parse(Subject.read(recipient).id);
            const now = Date.now();
            for (const database of copies) {
                await database
                    .insert(address.table)
                    .values({
                        id: Identifier.create("address"),
                        scope: id,
                        source: scope,
                        installation,
                        createdAt: now,
                        updatedAt: now,
                    })
                    .onConflictDoNothing();
            }
        },
    };
}

/** Serve an object service over HTTP in a space, authenticating `user:<id>` and `installation:<space>:<id>` bearers. */
function serve(implementation: ServiceImplementation, scope: string) {
    const audience = PackageId.parse(implementation.service.package.id);
    const server = Server.start({
        ...implementation,
        audience,
        scope,
        resources: new ResourceContext(),
        health: new Health(implementation.service.name),
        drainTimeout: 1000,
        authorizeHost: async () => {},
        authenticate: async (request) => {
            // read the caller the bearer names
            const bearer = present(request.headers.get("authorization"), "the bearer");
            const [kind, ...rest] = bearer.slice("Bearer ".length).split(":");
            const subject =
                kind === "user"
                    ? principal.user.reference("universe", present(rest[0], "the user"))
                    : principal.installation.reference(
                          present(rest[0], "the space"),
                          present(rest[1], "the installation"),
                      );
            const now = Date.now();

            return new Authentication({
                subject,
                subjects: [subject],
                credential: { kind: kind === "user" ? "user" : "installation", id: subject.id },
                audience,
                scope,
                verifiedAt: now,
                expiresAt: now + 60_000,
            });
        },
    });
    onTestFinished(() => server.close());

    return {
        server,
        endpoint: (caller: string) => ({
            url: "https://cell.test",
            headers: { authorization: `Bearer ${caller}` },
            fetch: (request: Request) => server.fetch(request),
        }),
    };
}
