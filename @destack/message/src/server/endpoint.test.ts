import { AccessContext, Caller, Policy, principal } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { CallEvent, call, defineAuditAction } from "@destack/object";
import { AuditRecorder, Journal } from "@destack/object/server";
import { testEvents } from "@destack/object/test";
import { outbox } from "@destack/runtime/outbox";
import { defineDatabase, type DatabaseConnection, eq } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { Derivation } from "@destack/identity";
import { defineServer, ObjectStore } from "@destack/object/server";
import { Package, PackageId } from "@destack/package";
import type {} from "@destack/package/import-meta";
import { Clock, Identifier, present, schema } from "@destack/schema";
import { type Controller, ControlLoop } from "@destack/runtime/control";
import { testCallKey } from "@destack/runtime/test";
import { ObjectReference, Scope } from "@destack/sync";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { Endpoint, endpoint, mail, message, messageAttempt } from "../object/index.ts";
import { mailProvider } from "../email/index.ts";
import messagePackage from "../package.ts";
import { messageTables } from "../stack/index.ts";
import { WebhookSignature, webhookProvider } from "../webhook/index.ts";
import { keepSecret } from "../test/index.ts";
import { account, space } from "@destack/account/object";
import vaultServer from "@destack/vault/server";
import { secret } from "@destack/vault/object";
import { vaultDatabase } from "@destack/vault/test";
import { implementEndpoint } from "./endpoint.ts";
import { implementMessage } from "./message.ts";
import { RequestId } from "@destack/runtime/request";

/** The scope type of the endpoints and their messages, as an organisation is in the universe. */
const place = new Policy(import.meta.destack.package, {
    name: "place",
    permissions: {},
    scope: true,
});

/** A database with endpoints, their messages and the calls they deliver, as a space keeps them. */
const endpointDatabase = defineDatabase({
    name: "endpoint",
    tables: [...messageTables, outbox, ...vaultDatabase.tables],
});

/** The fields of an endpoint posting each call as a signed event. */
const EVENT_FIELDS = {
    events: "all",
    format: "event",
    authentication: { kind: "signature" },
} as const;

/** A user other than the endpoint's creator. */
const SENDER_ID = schema.identifier("user").parse("user-019f5530-8000-7000-8000-0000000000f9");

/** The longest a test waits for an endpoint's delivery, far above three retries at 10 ms. */
const SETTLE_MILLISECONDS = 5000;

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

/** The identifier of the user whose calls the endpoints receive. */
const USER_ID = schema.identifier("user").parse("user-019f5530-8000-7000-8000-0000000000f2");

/** The machine serving the space, which keeps its vaults. */
const MACHINE = schema.identifier("machine").parse("machine-01996ab0-0000-7000-8000-0000000000a1");

/** The user whose calls the endpoints receive. */
const USER = principal.user.reference(Scope.universe.id, USER_ID);

/** The account whose space keeps the endpoints' secret. */
const ACCOUNT = schema.identifier("account").parse("account-019f5530-8000-7000-8000-0000000000f6");

/** The message installation serving the endpoints, as which their messages go out. */
const INSTALLATION = principal.installation.reference(
    "space-019f5530-8000-7000-8000-0000000000f4",
    "installation-019f5530-8000-7000-8000-0000000000f8",
);

/** Another installation of the space, which writes and consumes the second secret. */
const WRITER = principal.installation.reference(
    "space-019f5530-8000-7000-8000-0000000000f4",
    "installation-019f5530-8000-7000-8000-0000000000fa",
);

/** The endpoint's secret, a secret of a vault in a space. */
const SECRET: ObjectReference = {
    packageId: PackageId.parse("package-019f5530-8000-7000-8000-0000000000f3"),
    type: "secret",
    scope: "space-019f5530-8000-7000-8000-0000000000f4",
    id: "secret-019f5530-8000-7000-8000-0000000000f5",
};

/** The secret an endpoint rotates to, in the same vault. */
const ROTATED: ObjectReference = { ...SECRET, id: "secret-019f5530-8000-7000-8000-0000000000f9" };

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
function run(database: DatabaseConnection, controllers: readonly Controller[]): ControlLoop {
    const stopping = new AbortController();
    const loop = new ControlLoop(database, controllers, {
        clock: Clock.system,
        report: (_controller, _key, error) => {
            throw error;
        },
    });
    const running = loop.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });

    return loop;
}

/** Record a person's finished call in a scope. */
function finished(
    recording: Journal,
    action: typeof documentRename | typeof documentDelete,
    outcome: "success" | "failure" = "success",
): CallEvent {
    const recorder = new AuditRecorder(
        { caller: { subject: USER }, package: import.meta.destack.package, scope: SCOPE.id },
        recording,
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

test("refuse a webhook message to an endpoint from a sender who does not manage it", async () => {
    // keep an endpoint, and a sender who may send messages but not manage endpoints
    const { database, objects } = await serveHistory([]);
    const id = await subscribe(database, EVENT_FIELDS);
    const sender = principal.user.reference(Scope.universe.id, SENDER_ID);
    const role = {
        name: "sender",
        description: "Sends messages",
        permissions: [message.permission("send")],
    };
    await new AccessFixture(database).replicateRole(SCOPE, role, sender, 0);

    // refuse its webhook message posting to the endpoint with the endpoint's secret
    const now = Date.now();
    const created = objects.mutate(AccessContext.of(Caller.of(sender), Date.now()), {
        id: RequestId.create(now),
        calls: [
            message.call("create", {
                ...message.target({ scope: SCOPE.id, id: Identifier.create("message") }),
                to: { channel: "webhook" },
                ...ObjectReference.columns("source", endpoint.reference(SCOPE.id, id)),
                dueAt: now,
                status: "scheduled",
            }),
        ],
    });
    expect(await refusal(created)).toEqual(["FORBIDDEN", "permission denied: manage"]);
});

test("let the installation consume an endpoint's new secret as the manager who sets it, and release the old one", async () => {
    // keep an endpoint, a second secret, and a manager of endpoints
    const { database, objects } = await serveHistory([]);
    const id = await subscribe(database, EVENT_FIELDS);
    await keepSecret(objects, ROTATED, SECRET_VALUE, WRITER);
    const manager = principal.user.reference(Scope.universe.id, SENDER_ID);
    const access = new AccessFixture(database);
    const managing = {
        name: "manager",
        description: "Manages endpoints",
        permissions: [endpoint.permission("manage")],
    };
    await access.replicateRole(SCOPE, managing, manager, 0);

    // point the endpoint at the second secret as the manager
    const rotate = () => {
        const now = Date.now();

        return objects.mutate(AccessContext.of(Caller.of(manager), Date.now()), {
            id: RequestId.create(now),
            calls: [
                endpoint.call("update", {
                    ...endpoint.target({ scope: SCOPE.id, id }),
                    secret: ROTATED,
                }),
            ],
        });
    };

    // refuse a manager who may not let software use the secret, then accept one who may
    const refused = await refusal(rotate());
    const binding = {
        name: "binder",
        description: "Lets software use the space's secrets",
        permissions: [secret.permission("bind")],
    };
    await access.replicateRole(space.reference(ACCOUNT, SECRET.scope), binding, manager, 0);
    await rotate();

    // read each secret as the installation, which consumes only the second one
    const read = (kept: ObjectReference) =>
        objects.query(
            AccessContext.of(Caller.of(INSTALLATION), Date.now()),
            secret.call(
                "read",
                secret.target({ scope: kept.scope, id: secret.identifier(kept.id) }),
            ),
        );
    expect({
        refused,
        old: await refusal(read(SECRET)),
        rotated: await read(ROTATED),
    }).toEqual({
        refused: ["NOT_FOUND", `no secret ${ROTATED.id}`],
        old: ["NOT_FOUND", `no secret ${SECRET.id}`],
        rotated: { version: 1, value: { encoding: "text", value: SECRET_VALUE } },
    });
});

/** Serve endpoints over an audit history and their messages. */
async function serveHistory(statuses: readonly number[]) {
    // post through the webhook provider and keep the posts
    const posts: { headers: Headers; body: string }[] = [];
    const provider = webhookProvider(async (request) => {
        posts.push({ headers: new Headers(request.headers), body: await request.text() });
        const status = present(statuses[posts.length - 1], "a status for each post");

        return new Response(null, { status, statusText: `answered ${status}` });
    });

    // keep the endpoints with their messages and the calls they read, which record their deliveries too
    const storage = await TestDatabase.create("sqlite", endpointDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    const store = await testEvents(storage.database);
    const recording = new Journal(storage.database, testCallKey, {
        clock: Clock.system,
        events: store,
    });
    const objects = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: INSTALLATION,
        servers: [
            defineServer(messagePackage, {
                objects: [
                    implementMessage({
                        providers: { webhook: provider },
                        retry: RETRY,
                        keys: ROOT,
                    }),
                    messageAttempt,
                    implementEndpoint(ROOT),
                ],
                providers: [mailProvider("fixture", mail)],
            }),
            vaultServer,
        ],
        policies: [place, space],
        database: storage.database,
        callKey: testCallKey,
        events: store,
        origin: { package: Package.of(messagePackage) },
        machine: MACHINE,
    });
    const access = new AccessFixture(storage.database);
    await access.replicateScope({ ...SCOPE });
    await access.bindOwner(SCOPE, INSTALLATION);

    // keep the endpoints' secret in the vault of its space, consumed by the installation
    const vaulted = space.reference(ACCOUNT, SECRET.scope);
    await access.replicateScope(account.reference(Scope.universe.id, ACCOUNT));
    await access.replicateScope(vaulted);
    await access.bindOwner(vaulted, principal.space.reference(Scope.universe.id, SECRET.scope));
    await keepSecret(objects, SECRET, SECRET_VALUE, INSTALLATION);

    return {
        database: storage.database,
        recording,
        record: (calls: readonly CallEvent[]) =>
            store.append(
                call.eventKind(),
                calls.map((ended) => Journal.event(ended)),
            ),
        objects,
        posts,
        read: (id: Identifier<"endpoint">) => readDelivery(storage.database, id),
        until: (id: Identifier<"endpoint">, check: (delivery: Delivery) => boolean) =>
            TestDatabase.settle(
                storage.database,
                async () => check(await readDelivery(storage.database, id)),
                SETTLE_MILLISECONDS,
            ),
        start: () => run(storage.database, [...objects.controllers(), ...store.controllers]),
    };
}

/** An endpoint's messages and Delivered condition as a test reads them. */
type Delivery = Awaited<ReturnType<typeof readDelivery>>;

/** Read how many messages an endpoint has and its Delivered condition. */
async function readDelivery(database: DatabaseConnection, id: Identifier<"endpoint">) {
    const row = await readEndpoint(database, id);
    const sent = await database
        .select({ id: message.table.id })
        .from(message.table)
        .where(eq(message.table.sourceId, id));
    const delivered = row.conditions["Delivered"];

    return {
        messages: sent.length,
        delivered: delivered && [delivered.status, delivered.reason, delivered.message],
    };
}

/** Keep an enabled endpoint delivering from before every call, as its creator wrote it. */
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
        secret: SECRET,
        createdAt: 1,
        updatedAt: 1,
    });
    await endpoint.reader("endpoint").at(id).start(Date.now(), database);

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

/** Read posted events in data order: data, type, and whether the secret signed each. */
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

test("send each subscribed succeeded call of the scope as one event signed with the secret read as the endpoint's user", async () => {
    const served = await serveHistory([204, 204]);
    const id = await subscribe(served.database, {
        events: [documentRename.name],
        format: "event",
        authentication: { kind: "signature" },
    });

    // keep two renames with a failed rename and an unsubscribed deletion
    const first = finished(served.recording, documentRename);
    const second = finished(served.recording, documentRename);
    await served.record([
        first,
        finished(served.recording, documentRename, "failure"),
        finished(served.recording, documentDelete),
        second,
    ]);
    served.start();

    // post both renames once each in either order and mark the endpoint delivered
    await served.until(
        id,
        (delivery) => delivery.messages === 2 && delivery.delivered?.[0] === "true",
    );
    expect(await served.read(id)).toEqual({ messages: 2, delivered: ["true", "Delivered", ""] });
    expect(await readEvents(served.posts)).toEqual({
        data: [first, second]
            .map((delivered) => ({
                scope: delivered.execution.context.scope,
                target: delivered.execution.target,
                details: delivered.execution.details,
            }))
            .toSorted((left, right) => JSON.stringify(left).localeCompare(JSON.stringify(right))),
        types: [documentRename.name, documentRename.name],
        signatures: [true, true],
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
    const calls = [
        finished(served.recording, documentRename),
        finished(served.recording, documentDelete),
    ];
    await served.record(calls);
    served.start();

    // post both calls as lines of JSON with the secret in the header and no signature
    await served.until(
        id,
        (delivery) => delivery.messages === 1 && delivery.delivered?.[0] === "true",
    );
    expect(await served.read(id)).toEqual({ messages: 1, delivered: ["true", "Delivered", ""] });
    const [post] = served.posts;
    expect({
        posts: served.posts.length,
        calls: post?.body
            .trim()
            .split("\n")
            .map((line) => CallEvent.parse(JSON.parse(line)).execution.id),
        key: post?.headers.get("api-key"),
        signature: post?.headers.get("webhook-signature"),
        type: post?.headers.get("content-type"),
    }).toEqual({
        posts: 1,
        calls: calls.map((ended) => ended.execution.id),
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
    const position = () => served.database.log.slot(endpoint.reader("endpoint").at(id).name);
    const before = await position();

    // deliver one call and advance the endpoint past it
    await served.record([finished(served.recording, documentRename)]);
    served.start();
    const isAdvanced = async () => (await position()) !== before;
    await TestDatabase.settle(served.database, isAdvanced, SETTLE_MILLISECONDS);

    // repeat the page as a reconcile that failed to advance the endpoint does
    await served.database.log.advance(
        endpoint.reader("endpoint").at(id).name,
        before ?? 0,
        Date.now() + 60_000,
    );
    await TestDatabase.settle(served.database, isAdvanced, SETTLE_MILLISECONDS);

    // keep the one message the first page created, delivered once its attempt lands
    await served.until(id, (delivery) => delivery.delivered?.[0] === "true");
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
    const first = finished(served.recording, documentRename);
    await served.record([first]);
    const loop = served.start();
    await served.until(
        id,
        (delivery) => delivery.messages === 1 && delivery.delivered?.[0] === "true",
    );
    expect(await served.read(id)).toEqual({ messages: 1, delivered: ["true", "Delivered", ""] });

    // disable the endpoint through the space's own settings and keep a second call while it rests
    const update = async (status: "enabled" | "disabled") =>
        served.objects.mutate(
            AccessContext.of(Caller.of(INSTALLATION), Date.now()),
            {
                id: RequestId.create(Date.now()),
                calls: [
                    endpoint.call("update", {
                        ...endpoint.target(await readEndpoint(served.database, id)),
                        status,
                    }),
                ],
            },
            { component: "settings" },
        );
    const position = () => served.database.log.slot(endpoint.reader("endpoint").at(id).name);
    await update("disabled");
    await loop.idle();
    const delivered = await position();
    const second = finished(served.recording, documentDelete);
    await served.record([second]);
    await loop.idle();
    expect(await position()).toBe(delivered);

    // enable it and send only the second call
    await update("enabled");
    await served.until(id, (delivery) => delivery.messages === 2 && served.posts.length === 2);
    expect({
        ...(await served.read(id)),
        types: served.posts.map((post) => Envelope.parse(JSON.parse(post.body)).type),
    }).toEqual({
        messages: 2,
        delivered: ["true", "Delivered", ""],
        types: [documentRename.name, documentDelete.name],
    });
});

test("mark an endpoint failing once the message controller gives up a message its URL keeps refusing", async () => {
    const served = await serveHistory([500, 500]);
    const id = await subscribe(served.database, {
        events: "all",
        format: "json",
        authentication: { kind: "signature" },
    });

    // give the batch up after the retries and flag the endpoint with the URL's answer
    await served.record([finished(served.recording, documentRename)]);
    served.start();
    await served.until(id, (delivery) => delivery.delivered?.[1] === "Failing");
    expect(await served.read(id)).toEqual({
        messages: 1,
        delivered: ["false", "Failing", "answered 500"],
    });
});
