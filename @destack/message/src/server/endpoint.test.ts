import { Policy, principal } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { AuditCall, defineAuditAction } from "@destack/audit";
import { AuditHistory } from "@destack/audit/history";
import { AuditRecorder } from "@destack/audit/server";
import * as audit from "@destack/audit";
import { journal } from "@destack/audit/stack";
import { DatabasePersonalKeyring, EventStore } from "@destack/event";
import { eventTables } from "@destack/event/stack";
import { MemoryBucket } from "@destack/bucket/test";
import { Outbox, outbox } from "@destack/service/outbox";
import { defineDatabase, type DatabaseConnection, eq, Snapshot } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { Derivation, LocalKeyring } from "@destack/identity";
import { ObjectServer } from "@destack/object/server";
import { PackageId } from "@destack/package";
import type {} from "@destack/package/import-meta";
import { Identifier, present, schema } from "@destack/schema";
import { type Controller, ControlLoop } from "@destack/service/control";
import { testCallKey } from "@destack/service/test";
import { type ObjectReference, Scope, Subject } from "@destack/sync";
import { expect, onTestFinished, test } from "@destack/test";
import { type Endpoint, endpoint, message, MessageKey } from "../object/index.ts";
import { messageService } from "../service/index.ts";
import { messageTables } from "../stack/index.ts";
import { WebhookSignature, webhookProvider } from "../webhook/index.ts";
import { EndpointReader, serveEndpoints } from "./endpoint.ts";
import { serveMessages } from "./server.ts";

/** The scope type the endpoints and their messages live in, as an organisation is in the universe. */
const place = new Policy(import.meta.destack.package, {
    name: "place",
    permissions: {},
    scope: true,
});

/** A database keeping endpoints, their messages and the audit history they deliver, as a machine keeps them. */
const endpointDatabase = defineDatabase({
    name: "endpoint",
    tables: [...messageTables, journal, ...eventTables([audit.call]), outbox],
});

/** The keyring sealing people's personal values in the history. */
const KEYRING = await LocalKeyring.read(LocalKeyring.generate());

/** The root secret the scope's message key derives from. */
const ROOT = Derivation.of(await Derivation.root(crypto.getRandomValues(new Uint8Array(32))));

/** How refused sends retry here: twice at 10 ms. */
const RETRY = {
    initialInterval: 10,
    maximumInterval: 10,
    maximumAttempts: 2,
    backoffCoefficient: 1,
};

/** The scope whose history the endpoint receives. */
const SCOPE = place.reference(Scope.universe.id, "place-019f5530-8000-7000-8000-0000000000f1");

/** The space whose server keeps the endpoints and owns their scope. */
const HOST = principal.space.reference(
    Scope.universe.id,
    "space-019f5530-8000-7000-8000-0000000000f7",
);

/** A scope inside the endpoint's scope. */
const NESTED = place.reference(SCOPE.id, "place-019f5530-8000-7000-8000-0000000000f6");

/** The user who created the endpoint, as whom its secret is read. */
const USER = principal.user.reference(
    Scope.universe.id,
    "user-019f5530-8000-7000-8000-0000000000f2",
);

/** The endpoint's secret, a secret of a vault in a space. */
const SECRET: ObjectReference = {
    packageId: PackageId.parse("package-019f5530-8000-7000-8000-0000000000f3"),
    type: "secret",
    scope: "space-019f5530-8000-7000-8000-0000000000f4",
    id: "secret-019f5530-8000-7000-8000-0000000000f5",
};

/** The value the vault keeps in the endpoint's secret, the Standard Webhooks example secret. */
const SECRET_VALUE = "whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw";

/** The URL every endpoint posts to. */
const URL = "https://hooks.example.com/destack";

/** A person's rename of a document, the event endpoints subscribe to. */
const documentRename = defineAuditAction(
    {
        name: "document.rename",
        target: schema.object({ type: schema.literal("document"), id: schema.string() }),
        details: schema.object({ name: schema.string() }),
    },
    { package: import.meta.destack.package },
);

/** A person's deletion of a document, an event no endpoint here subscribes to. */
const documentDelete = defineAuditAction(
    {
        name: "document.delete",
        target: schema.object({ type: schema.literal("document"), id: schema.string() }),
        details: schema.object({}),
    },
    { package: import.meta.destack.package },
);

/** The envelope of a posted event. */
const Envelope = schema.looseObject({ type: schema.string(), data: schema.json() });

/** Run some controllers over a database until the test ends, failing it on a reported failure. */
function run(database: DatabaseConnection, controllers: readonly Controller[]): void {
    const stopping = new AbortController();
    const running = new ControlLoop(database, controllers, {
        report: (_controller, _key, error) => {
            throw error;
        },
    }).run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });
}

/** Record a person's finished call in a scope. */
function finished(
    scope: ObjectReference,
    action: typeof documentRename | typeof documentDelete,
    outcome: "success" | "failure" = "success",
): AuditCall {
    const recorder = new AuditRecorder(
        {
            caller: { subject: USER },
            package: import.meta.destack.package,
            service: "document",
            scope: scope.id,
        },
        { record: async () => {}, clock: Date.now },
    );
    const target = { type: "document" as const, id: "one" };
    const running =
        action === documentRename
            ? recorder.begin(documentRename, { target, details: { name: "renamed" } })
            : recorder.begin(documentDelete, { target, details: {} });

    return recorder.finish(
        running,
        outcome === "success"
            ? { kind: "success" }
            : { kind: "failure", error: { code: "CONFLICT", status: 409, message: "conflict" } },
    );
}

/** Serve endpoints over an audit history and the messages they send, posting to a URL answering each post with the next status in turn. */
async function serveHistory(statuses: readonly number[]) {
    // post through the webhook provider and keep the posts and the secrets read
    const posts: { headers: Headers; body: string }[] = [];
    const reads: [ObjectReference, Subject][] = [];
    const provider = webhookProvider(
        async (_system, secret, reader) => {
            reads.push([secret, reader]);

            return SECRET_VALUE;
        },
        async (request) => {
            posts.push({ headers: new Headers(request.headers), body: await request.text() });
            const status = present(statuses[posts.length - 1], "a status for each post");

            return new Response(null, { status, statusText: `answered ${status}` });
        },
    );

    // keep the endpoints with their messages and the history they read, which records their deliveries too
    const storage = await TestDatabase.create("sqlite", endpointDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    const store = openEvents(storage.database);
    const history = new AuditHistory(store);
    const objects = new ObjectServer({
        principal: HOST,
        objects: {
            ...serveMessages({
                providers: { webhook: provider },
                retry: RETRY,
                deriver: async () => ROOT,
            }),
            endpoint: serveEndpoints({
                events: store,
                key: async () => (await MessageKey.derive(ROOT)).key,
            }),
        },
        policies: [place],
        database: storage.database,
        callKey: testCallKey,
        origin: { package: messageService.package, service: messageService.name },
        history: { ingest: (batch) => history.ingest(batch) },
    });
    const access = new AccessFixture(storage.database);
    await access.copyScope({ ...SCOPE });
    await access.copyScope({ ...NESTED });
    await access.copyOwner(SCOPE, HOST);

    return {
        database: storage.database,
        history,
        objects,
        posts,
        reads,
        read: (id: Identifier<"endpoint">) => readDelivery(storage.database, id),
        start: () => run(storage.database, [...objects.controllers(), ...store.controllers]),
    };
}

/** Open the events keeping the audit history, routing each call to the scopes enclosing its scope. */
function openEvents(database: DatabaseConnection): EventStore {
    const store = new EventStore({
        database,
        kinds: [audit.call],
        files: () => new MemoryBucket(),
        personal: new DatabasePersonalKeyring(database, KEYRING),
        outbox: new Outbox(database),
        targets: async (_route, scope) => {
            const chains = await Scope.chains(Snapshot.live(database), [scope]);

            return (chains.get(scope) ?? []).slice(1).map((link) => link.object.id);
        },
        deliver: (_kind, events) => store.receive(audit.call.key, events),
    });

    return store;
}

/** Read how many messages an endpoint has and its Delivered condition. */
async function readDelivery(database: DatabaseConnection, id: Identifier<"endpoint">) {
    const row = await readEndpoint(database, id);
    const sent = await database
        .select({ id: message.table.id })
        .from(message.table)
        .where(eq(message.table.source, endpoint.reference(SCOPE.id, id)));
    const delivered = row.conditions["Delivered"];

    return {
        messages: sent.length,
        delivered: delivered && [delivered.status, delivered.reason, delivered.message],
    };
}

/** Keep an enabled endpoint of the scope delivering from before every call, as its creator wrote it. */
async function subscribe(
    database: DatabaseConnection,
    fields: Pick<Endpoint, "events" | "format" | "authentication">,
): Promise<Identifier<"endpoint">> {
    const id = Identifier.create("endpoint");
    await database.insert(endpoint.table).values({
        id,
        scope: SCOPE.id,
        url: URL,
        ...fields,
        status: "enabled",
        userId: USER.id,
        secret: SECRET,
        createdAt: 1,
        updatedAt: 1,
    });
    await EndpointReader.of(id).start(database, Date.now());

    return id;
}

/** Read an endpoint as its table keeps it. */
async function readEndpoint(
    database: DatabaseConnection,
    id: Identifier<"endpoint">,
): Promise<Endpoint> {
    const [row] = await database.select().from(endpoint.table).where(eq(endpoint.table.id, id));

    return present(row, "the endpoint");
}

/** Read posted events in the order of their data: their data, their types, and whether the secret signed each. */
async function readEvents(posts: readonly { readonly headers: Headers; readonly body: string }[]) {
    // order the envelopes by their data
    const bodies = posts
        .map((post) => Envelope.parse(JSON.parse(post.body)))
        .toSorted((left, right) =>
            JSON.stringify(left.data).localeCompare(JSON.stringify(right.data)),
        );

    // check each post's signature over its identifier, time and body
    const signatures = await Promise.all(
        posts.map(
            async (post) =>
                post.headers.get("webhook-signature") ===
                (await WebhookSignature.sign(
                    SECRET_VALUE,
                    String(post.headers.get("webhook-id")),
                    String(post.headers.get("webhook-timestamp")),
                    post.body,
                )),
        ),
    );

    return {
        data: bodies.map((body) => body.data),
        types: bodies.map((body) => body.type),
        signatures,
    };
}

test("send each subscribed succeeded call of the scope and the scopes inside it as one event signed with the secret read as the endpoint's user", async () => {
    const served = await serveHistory([204, 204]);
    const id = await subscribe(served.database, {
        events: [documentRename.name],
        format: "event",
        authentication: { kind: "signature" },
    });

    // keep renames in and inside the scope with a failed rename and an unsubscribed deletion
    const outer = finished(SCOPE, documentRename);
    const nested = finished(NESTED, documentRename);
    await served.history.ingest({
        calls: [
            outer,
            finished(SCOPE, documentRename, "failure"),
            finished(SCOPE, documentDelete),
            nested,
        ],
    });
    served.start();

    // post both renames once each in either order and mark the endpoint delivered
    await expect
        .poll(() => served.read(id), { timeout: 10_000 })
        .toEqual({ messages: 2, delivered: ["true", "Delivered", ""] });
    expect({ ...(await readEvents(served.posts)), reads: served.reads }).toEqual({
        data: [outer, nested]
            .map((delivered) => ({
                scope: delivered.execution.context.scope,
                target: delivered.execution.target,
                details: delivered.execution.details,
            }))
            .toSorted((left, right) => JSON.stringify(left).localeCompare(JSON.stringify(right))),
        types: [documentRename.name, documentRename.name],
        signatures: [true, true],
        reads: [
            [SECRET, USER],
            [SECRET, USER],
        ],
    });
});

test("send a page of calls as one NDJSON batch carrying the secret in the endpoint's header", async () => {
    const served = await serveHistory([204]);
    const id = await subscribe(served.database, {
        events: "all",
        format: "ndjson",
        authentication: { kind: "header", name: "api-key" },
    });

    // keep a rename and a deletion
    const calls = [finished(SCOPE, documentRename), finished(NESTED, documentDelete)];
    await served.history.ingest({ calls });
    served.start();

    // post both calls as lines of JSON with the secret in the header and no signature
    await expect
        .poll(() => served.read(id), { timeout: 10_000 })
        .toEqual({ messages: 1, delivered: ["true", "Delivered", ""] });
    const [post] = served.posts;
    expect({
        posts: served.posts.length,
        calls: post?.body
            .trim()
            .split("\n")
            .map((line) => AuditCall.parse(JSON.parse(line)).execution.id),
        key: post?.headers.get("api-key"),
        signature: post?.headers.get("webhook-signature"),
        type: post?.headers.get("content-type"),
    }).toEqual({
        posts: 1,
        calls: calls.map((call) => call.execution.id),
        key: SECRET_VALUE,
        signature: null,
        type: "application/x-ndjson",
    });
});

test("advance an endpoint past each page and create no message twice when a page repeats", async () => {
    const served = await serveHistory([204, 204]);
    const id = await subscribe(served.database, {
        events: "all",
        format: "event",
        authentication: { kind: "signature" },
    });
    const position = () => served.database.log.slot(EndpointReader.of(id).name);
    const before = await position();

    // deliver one call and advance the endpoint past it
    await served.history.ingest({ calls: [finished(SCOPE, documentRename)] });
    served.start();
    await expect.poll(async () => (await position()) !== before, { timeout: 10_000 }).toBe(true);

    // repeat the page as a reconcile that failed to advance the endpoint does
    await served.database.log.advance(EndpointReader.of(id).name, before ?? 0, Date.now() + 60_000);
    await expect.poll(async () => (await position()) !== before, { timeout: 10_000 }).toBe(true);

    // keep the one message the first page created
    expect(await served.read(id)).toEqual({ messages: 1, delivered: ["true", "Delivered", ""] });
});

test("keep a disabled endpoint's position and deliver only the calls after it once enabled again", async () => {
    const served = await serveHistory([204, 204]);
    const id = await subscribe(served.database, {
        events: "all",
        format: "event",
        authentication: { kind: "signature" },
    });

    // deliver a first call
    const first = finished(SCOPE, documentRename);
    await served.history.ingest({ calls: [first] });
    served.start();
    await expect
        .poll(() => served.read(id), { timeout: 10_000 })
        .toEqual({ messages: 1, delivered: ["true", "Delivered", ""] });

    // disable the endpoint through the space's own settings and keep a second call while it rests
    const update = async (status: "enabled" | "disabled") =>
        served.objects.execute(
            HOST,
            endpoint,
            "update",
            [
                {
                    scope: SCOPE.id,
                    target: await readEndpoint(served.database, id),
                    input: { status },
                },
            ],
            Date.now(),
            "settings",
        );
    const position = () => served.database.log.slot(EndpointReader.of(id).name);
    const delivered = await position();
    await update("disabled");
    const second = finished(SCOPE, documentDelete);
    await served.history.ingest({ calls: [second] });
    expect(await position()).toBe(delivered);

    // enable it and send the second call alone
    await update("enabled");
    await expect
        .poll(
            async () => ({
                ...(await served.read(id)),
                types: served.posts.map((post) => Envelope.parse(JSON.parse(post.body)).type),
            }),
            { timeout: 10_000 },
        )
        .toEqual({
            messages: 2,
            delivered: ["true", "Delivered", ""],
            types: [documentRename.name, documentDelete.name],
        });
});

test("mark an endpoint failing once the message service gives up a message its URL keeps refusing", async () => {
    const served = await serveHistory([500, 500]);
    const id = await subscribe(served.database, {
        events: "all",
        format: "json",
        authentication: { kind: "signature" },
    });

    // give the batch up after the retries and flag the endpoint with the URL's answer
    await served.history.ingest({ calls: [finished(SCOPE, documentRename)] });
    served.start();
    await expect
        .poll(() => served.read(id), { timeout: 10_000 })
        .toEqual({ messages: 1, delivered: ["false", "Failing", "answered 500"] });
});
