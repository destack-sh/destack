import { principal } from "@destack/access";
import { and, type DatabaseConnection, defineDatabase, eq } from "@destack/db";
import { Derivation } from "@destack/identity";
import { message, MessageKey } from "@destack/message";
import { TestDatabase } from "@destack/db/test";
import { Account, account, client, key, residence, user } from "@destack/account/object";
import { setting } from "@destack/setting/object";
import { ObjectExtension } from "@destack/object/server";
import type { InstallationDirectory } from "@destack/service/workload";
import { PackageId } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import { Identifier, present, schema, Clock } from "@destack/schema";
import { defineService, type Service } from "@destack/service";
import { createClient } from "@destack/service/client";
import { Health } from "@destack/service/health";
import { RequestId } from "@destack/service/request";
import { Server, type ServiceImplementation } from "@destack/service/server";
import { testCallKey } from "@destack/service/test";
import { outbox } from "@destack/service/outbox";
import { until } from "@destack/service/timer";
import { journal } from "@destack/audit/stack";
import {
    type Mutation,
    type Publisher,
    Scope,
    Subject,
    type Subscription,
    type Uplink,
} from "@destack/sync";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { activity, announcement, delivery, notification, subscription } from "../src/index.ts";
import { implementInbox, serveActivities } from "../src/server/index.ts";
import { inboxService } from "../src/service/index.ts";
import { document, notifications } from "./fixture/document.ts";
import { INBOX_BUILD, INBOX_CATALOGS, NotificationFixture, unfetched } from "../src/test/index.ts";
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

/** The tables of a home's main database that the inbox uses, with its copies. */
const homeDatabase = defineDatabase({
    name: "main",
    tables: [...notification.tables, ...delivery.tables, journal],
    copies: [
        user.table,
        account.table,
        residence.table,
        setting.table,
        client.table,
        key.table,
        message.table,
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

/** An uplink streaming nothing until stopped and receiving no changes, as a machine with no chain to copy. */
const idle: Uplink = {
    async *stream(_request, signal) {
        await until(signal);
        yield* [];
    },
    receive: () => Promise.reject(new Error("the idle uplink receives no changes")),
};

test(
    "project a mention into the recipient's home within seconds, and refuse another space's installation",
    { timeout: 30_000 },
    async () => {
        // keep the documents' space and alice's home, and serve the documents as their installation
        const { accounts, homeId, spaceId, copies, home, documents } = await openScenario();
        const alice = Subject.key(actors.alice);
        const followed: Subscription[] = [];
        const follow = (caller: string): Publisher => {
            const reader = createClient(documentsService, documents.endpoint(caller));

            return {
                async *stream(request: Subscription, signal: AbortSignal) {
                    followed.push(request);
                    yield* await reader.replica.stream(request, { signal });
                },
            };
        };

        // serve alice's inbox in her home as the home's installation, following the documents at their peer address
        const caller = `installation:${homeId}:${INBOX}`;
        const copied: string[] = [];
        serve(
            ObjectExtension.compose(
                [
                    implementInbox({
                        catalogs: INBOX_CATALOGS,
                        key: async () => {
                            throw new Error("the inbox sends no messages here");
                        },
                    }),
                ],
                {
                    clock: Clock.system,
                    origin: { package: inboxService.package, service: inboxService.name },
                    database: home,
                    callKey: testCallKey,
                    installation: {
                        id: INBOX,
                        scope: homeId,
                        build: INBOX_BUILD,
                        publisher: {
                            stream: (request, signal) => {
                                copied.push(request.shape);

                                return idle.stream(request, signal);
                            },
                            receive: (mutation) => idle.receive(mutation),
                        },
                        settings: {},
                        publisherAt: () => follow(caller),
                        at: unfetched,
                        directory: installationDirectory(
                            accounts,
                            copies,
                            homeId,
                            INBOX,
                            inboxService,
                        ),
                    },
                },
            ).services[0],
            homeId,
        );

        // let bob mention alice on a document she views, once her inbox copies its chain without any source
        await expect.poll(() => copied.length > 0).toBe(true);
        expect(followed).toEqual([]);
        const bob = createClient(documentsService, documents.endpoint(`user:${actors.bob.id}`));
        const created = await bob.document.create({
            spaceId,
            requestId: RequestId.create(Date.now()),
            title: "Launch plan",
        });
        await bob.document.grant({
            spaceId,
            requestId: RequestId.create(Date.now()),
            id: created.id,
            relation: "viewer",
            subject: actors.alice,
        });
        await bob.document.remark({
            spaceId,
            requestId: RequestId.create(Date.now()),
            id: created.id,
            text: "@alice, a question",
            mentions: [actors.alice],
        });

        // receive the notification in her home within seconds of the mention
        const inbox = async () =>
            (await home.select().from(notification.table)).map((row) => [
                row.recipient,
                row.space,
                row.content.body,
            ]);
        await expect
            .poll(() => inbox(), { timeout: 5000 })
            .toEqual([[alice, spaceId, "@alice, a question"]]);

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
            `only ${homeId} follows the rows its residents receive`,
        ]);
    },
);

/** Open the account service's, the documents' and alice's home databases, and serve the documents. */
async function openScenario() {
    // keep the account service's users and addresses, the documents' space and alice's home
    const accounts = await open([...user.tables, ...account.tables, ...residence.tables]);
    const source = await open(documentsDatabase);
    const home = await open(homeDatabase);
    const spaceId = await NotificationFixture.openSpace(source, Date.now());
    const homeId = await NotificationFixture.openSpace(home, Date.now());
    const copies = [accounts, home];
    await signUp(copies, Subject.key(actors.alice), homeId);

    // serve the documents as their installation, recording whom their activities address
    const documents = serve(
        ObjectExtension.compose(
            [ObjectExtension.of(documentsService, serveActivities({ notifications }))],
            {
                clock: Clock.system,
                origin: { package: documentsService.package, service: documentsService.name },
                database: source,
                callKey: testCallKey,
                installation: {
                    id: DOCUMENTS,
                    scope: spaceId,
                    build: NOTES_BUILD,
                    publisher: idle,
                    settings: {},
                    publisherAt: () => idle,
                    at: unfetched,
                    directory: installationDirectory(
                        accounts,
                        copies,
                        spaceId,
                        DOCUMENTS,
                        documentsService,
                    ),
                },
            },
        ).services[0],
        spaceId,
    );

    return { accounts, source, home, spaceId, homeId, copies, documents };
}

/** Open a migrated SQLite database, closed after the test. */
async function open(
    database: Parameters<typeof TestDatabase.create>[1],
): Promise<DatabaseConnection> {
    const opened = await TestDatabase.create("sqlite", database, { isMigrated: true });
    onTestFinished(() => opened.close());

    return opened.database;
}

/** Sign alice up with her personal account at a home, in the account service's database and her home's copies. */
async function signUp(
    copies: readonly DatabaseConnection[],
    recipient: string,
    home: Identifier<"space">,
): Promise<void> {
    const id = schema.identifier("user").parse(Subject.read(recipient).id);
    const now = Date.now();
    for (const database of copies) {
        await database.insert(user.table).values({
            id,
            scope: Scope.universe.id,
            name: "alice",
            email: "alice@example.com",
            timeZone: "Europe/Vienna",
            locale: "de-AT",
            createdAt: now,
            updatedAt: now,
        });
        await database.insert(account.table).values({
            id: Identifier.derive("account", Identifier.uuid(home)),
            scope: id,
            handle: "alice",
            name: "alice",
            kind: "personal",
            residencyId: "eu",
            createdAt: now,
            updatedAt: now,
        });
    }
}

/** Confirm homes, read locales and record addresses as an installation directory does. */
function installationDirectory(
    accounts: DatabaseConnection,
    copies: readonly DatabaseConnection[],
    scope: Identifier<"space">,
    installation: Identifier<"installation">,
    service: Pick<Service, "package">,
): InstallationDirectory {
    return {
        isHome: async (subject, space) => {
            // compare the home of the personal account of the user the subject is
            const id = schema.identifier("user").parse(Subject.read(subject).id);
            const [personal] = await accounts
                .select({ id: account.table.id })
                .from(account.table)
                .where(and(eq(account.table.scope, id), eq(account.table.kind, "personal")));

            return personal !== undefined && Account.home(personal.id) === space;
        },
        locale: async (subject) => {
            // read the locale of the user the subject is
            const id = schema.identifier("user").parse(Subject.read(subject).id);
            const [person] = await accounts
                .select({ locale: user.table.locale })
                .from(user.table)
                .where(eq(user.table.id, id));

            return present(person?.locale, `the locale of ${subject}`);
        },
        messageKey: async (space) => {
            throw new Error(`no space ${space} is sent messages here`);
        },
        address: async (recipient) => {
            // keep the residence in the recipient's user scope and its chain copies
            const id = schema.identifier("user").parse(Subject.read(recipient).id);
            const now = Date.now();
            for (const database of copies) {
                await database
                    .insert(residence.table)
                    .values({
                        id: Identifier.create("residence"),
                        scope: id,
                        sourceId: schema.identifier("space").parse(scope),
                        installation,
                        service: PackageId.parse(service.package.id),
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
    // serve at once, as the test writes the copies itself and the idle uplink sends no snapshot
    const audience = PackageId.parse(implementation.service.package.id);
    const { ready: _ready, ...served } = implementation;
    const server = Server.start({
        clock: Clock.system,
        ...served,
        audience,
        scope,
        resources: new ResourceContext(),
        health: new Health(implementation.service.name),
        drainTimeout: 1000,
        authorizeMachine: async () => {},
        authenticate: async (request) => {
            // read the caller from the bearer
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

            return {
                subject,
                subjects: [subject],
                credential: {
                    kind: kind === "user" ? "user" : "installation",
                    id: subject.id,
                    audience,
                    scope,
                    verifiedAt: now,
                    expiresAt: now + 60_000,
                },
            };
        },
    });
    onTestFinished(() => server.close());

    return {
        server,
        endpoint: (caller: string) => ({
            url: "https://machine.test",
            headers: { authorization: `Bearer ${caller}` },
            fetch: (request: Request) => server.fetch(request),
        }),
    };
}

test(
    "seal an emailed notification to its home's message key in the inbox, so no outbox row, journal entry, change-log row or sent change carries its text",
    { timeout: 30_000 },
    async () => {
        // keep the documents' space and alice's home, serve the documents, and verify alice's email
        const { accounts, source, homeId, spaceId, copies, home, documents } = await openScenario();
        for (const database of copies) {
            await database.update(user.table).set({ emailVerified: true });
        }

        // grant the documents and the inbox what their workloads request, as approving them does
        await NotificationFixture.grant(
            source,
            spaceId,
            principal.installation.reference(spaceId, DOCUMENTS),
            [announcement.permission("expand"), document.permission("read")],
            Date.now(),
        );
        await NotificationFixture.grant(
            home,
            homeId,
            principal.installation.reference(homeId, INBOX),
            [
                notification.permission("read"),
                notification.permission("deliver"),
                delivery.permission("deliver"),
                message.permission("send"),
                message.permission("read"),
            ],
            Date.now(),
        );

        // serve alice's inbox emailing at once, sealing to the home's message key and sending its messages to its space
        const root = Derivation.of(
            await Derivation.root(crypto.getRandomValues(new Uint8Array(32))),
        );
        const sealing = await MessageKey.derive(root);
        const sent: Mutation[] = [];
        const caller = `installation:${homeId}:${INBOX}`;
        const inboxImplementation = ObjectExtension.compose(
            [
                implementInbox({
                    catalogs: INBOX_CATALOGS,
                    key: async () => sealing.key,
                    emailDelay: { milliseconds: 0 },
                }),
            ],
            {
                clock: Clock.system,
                origin: { package: inboxService.package, service: inboxService.name },
                database: home,
                callKey: testCallKey,
                installation: {
                    id: INBOX,
                    scope: homeId,
                    build: INBOX_BUILD,
                    publisher: {
                        stream: (request, signal) => idle.stream(request, signal),
                        receive: async (mutation) => {
                            sent.push(mutation);
                        },
                    },
                    settings: {},
                    publisherAt: () => {
                        const reader = createClient(documentsService, documents.endpoint(caller));

                        return {
                            stream: async function* (request, signal) {
                                yield* await reader.replica.stream(request, { signal });
                            },
                        };
                    },
                    at: unfetched,
                    directory: installationDirectory(accounts, copies, homeId, INBOX, inboxService),
                },
            },
        ).services[0];
        serve(inboxImplementation, homeId);

        // let bob mention alice on a document she views
        const text = "@alice, the sealed question";
        const bob = createClient(documentsService, documents.endpoint(`user:${actors.bob.id}`));
        const created = await bob.document.create({
            spaceId,
            requestId: RequestId.create(Date.now()),
            title: "Launch plan",
        });
        await bob.document.grant({
            spaceId,
            requestId: RequestId.create(Date.now()),
            id: created.id,
            relation: "viewer",
            subject: actors.alice,
        });
        await bob.document.remark({
            spaceId,
            requestId: RequestId.create(Date.now()),
            id: created.id,
            text,
            mentions: [actors.alice],
        });

        // wait for the email message the inbox sends its space
        const emailed = async () =>
            sent.flatMap((mutation) =>
                mutation.calls.filter(
                    (call) => call.type.type === message.name && call.method === "create",
                ),
            );
        await expect.poll(async () => (await emailed()).length, { timeout: 10_000 }).toBe(1);
        const [call] = await emailed();
        const ciphertext = schema
            .string()
            .parse(present(call, "the email message").input["ciphertext"]);

        // list where the text appears beside the notification itself: the outbox, the journal, the change log of the deliveries and messages, and the sent changes
        const traces = {
            outbox: await home.select().from(outbox),
            journal: await home.select().from(journal),
            log: (await home.log.read({ tables: [delivery.table, message.table], after: 0 }))
                .changes,
            sent,
        };
        const leaking = Object.entries(traces).flatMap(([name, rows]) =>
            JSON.stringify(rows).includes(text) ? [name] : [],
        );

        // open the content with the home's key as its message home does
        const opened = await sealing.open(ciphertext, homeId);

        expect({ leaking, opened }).toEqual({
            leaking: [],
            opened: { channel: "email", subject: "bob mentioned you", text },
        });
    },
);
