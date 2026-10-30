import { principal } from "@destack/access";
import { eq } from "@destack/db";
import { StandardWebhook } from "@destack/object";
import { schema } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { expect, refusal, test } from "@destack/test";
import { secret } from "@destack/vault/object";
import vaultPackage from "@destack/vault/package";
import { endpoint, message } from "../object/index.ts";
import messagePackage from "../package.ts";
import { documentsPackage, TestMessage } from "../test/index.ts";

/** The value of the endpoints' secret, the Standard Webhooks example secret. */
const SECRET_VALUE = "whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw";

/** The fields of an endpoint posting each call as a signed event. */
const EVENTS = { events: "all", format: "event", authentication: { kind: "signature" } } as const;

/** The envelope of a posted event. */
const Envelope = schema.looseObject({ type: schema.string(), data: schema.json() });

/** A record of a posted batch: a call's method as its type, with its scope, target and details. */
const BatchRecord = schema.object({
    type: schema.string(),
    scope: schema.string(),
    target: schema.json(),
    details: schema.json(),
});

/** Create, rename and delete documents as the owner, the space's history the endpoints read. */
function documents(fixture: TestMessage) {
    const client = fixture.owner.client(documentsPackage);
    const spaceId = fixture.space.id;
    const requestId = () => RequestId.create(fixture.environment);

    return {
        create: (name: string) => client.document.create({ spaceId, requestId: requestId(), name }),
        rename: (id: string, name: string) =>
            client.document.update({ spaceId, id, requestId: requestId(), name }),
        delete: (id: string) => client.document.delete({ spaceId, id, requestId: requestId() }),
    };
}

/** Read an endpoint's message count and Delivered condition. */
async function delivery(fixture: TestMessage, id: string) {
    const [row] = await fixture.served.database
        .select({ conditions: endpoint.table.conditions })
        .from(endpoint.table)
        .where(eq(endpoint.table.id, endpoint.identifier(id)));
    const sent = await fixture.served.database.select({ id: message.table.id }).from(message.table);
    const delivered = row?.conditions["Delivered"];

    return {
        messages: sent.length,
        delivered: delivered && [delivered.status, delivered.reason, delivered.message],
    };
}

/** Read posted bodies' types, refusing a post the secret does not verify. */
async function readPosts(fixture: TestMessage) {
    return Promise.all(
        fixture.hooks.requests.map(async (post) => {
            const received = new Request(post.url, {
                method: "POST",
                headers: post.headers,
                body: post.body,
            });
            const webhook = await StandardWebhook.of(SECRET_VALUE, fixture.environment.crypto);
            const { body } = await webhook.verify(received, fixture.clock.now());

            return Envelope.parse(JSON.parse(body)).type;
        }),
    );
}

test("refuse a webhook message to an endpoint from a sender who does not manage it", async () => {
    // create an endpoint, and a member who may send messages but not manage endpoints
    const fixture = await TestMessage.open();
    const id = await fixture.subscribe({ ...EVENTS, secret: await fixture.secret(SECRET_VALUE) });
    const member = await fixture.member([message.permission("send")]);

    // refuse the member's webhook message posting to the endpoint with the endpoint's secret
    const created = fixture.space
        .person(fixture.served, member)
        .client(messagePackage)
        .message.create({
            ...message.target({
                scope: fixture.space.id,
                id: message.generateId(fixture.environment),
            }),
            requestId: RequestId.create(fixture.environment),
            to: { channel: "webhook" },
            sourcePackageId: endpoint.package.id,
            sourceType: endpoint.name,
            sourceScope: fixture.space.id,
            sourceId: id,
            ciphertext: await fixture.seal({
                channel: "webhook",
                type: "x",
                format: "event",
                data: {},
            }),
        });
    expect(await refusal(created)).toEqual([
        "FORBIDDEN",
        "PERMISSION_DENIED",
        "permission denied: manage",
    ]);
});

test("let the installation consume an endpoint's new secret as the manager who sets it, and release the old one", async () => {
    // create an endpoint with a secret, a second secret, and a member managing endpoints
    const fixture = await TestMessage.open();
    const first = await fixture.secret(SECRET_VALUE);
    const rotated = await fixture.secret("whsec_cm90YXRlZA==");
    const id = await fixture.subscribe({ ...EVENTS, secret: first });
    const person = await fixture.member([endpoint.permission("manage")]);
    const manager = fixture.space.person(fixture.served, person).client(messagePackage);
    const rotate = () =>
        manager.endpoint.update({
            scope: fixture.space.id,
            id,
            requestId: RequestId.create(fixture.environment),
            secret: rotated,
        });

    // refuse a manager who may not let software use the secret, then accept one who may
    const refused = await refusal(rotate());
    await fixture.allow(person, [endpoint.permission("manage"), secret.permission("bind")]);
    await rotate();

    // read each secret as the installation, which consumes only the second one
    const read = (kept: typeof first) =>
        fixture.served
            .as(principal.installation.reference(fixture.space.id, fixture.installation))
            .client(vaultPackage)
            .secret.read({ scope: fixture.space.id, id: secret.identifier(kept.id) });
    expect({ refused, old: await refusal(read(first)), rotated: await read(rotated) }).toEqual({
        refused: ["NOT_FOUND", "NOT_FOUND", `secret ${rotated.id} not found`],
        old: ["NOT_FOUND", "NOT_FOUND", `secret ${first.id} not found`],
        rotated: { version: 1, value: { encoding: "text", value: "whsec_cm90YXRlZA==" } },
    });
});

test("send each subscribed succeeded call as one event signed with the secret the installation consumes", async () => {
    // subscribe to renames, then create, rename twice, fail a rename and delete a document
    const fixture = await TestMessage.open();
    const id = await fixture.subscribe({
        ...EVENTS,
        events: ["document.update"],
        secret: await fixture.secret(SECRET_VALUE),
    });
    const history = documents(fixture);
    const created = await history.create("draft");
    await history.rename(created.id, "first");
    await history.rename(created.id, "second");
    await refusal(history.rename("document-019f5530-8000-7000-8000-0000000000ff", "missing"));
    await history.delete(created.id);
    await fixture.simulation.idle();

    // post both renames once each, signed, and mark the endpoint delivered
    expect({ delivery: await delivery(fixture, id), posts: await readPosts(fixture) }).toEqual({
        delivery: { messages: 2, delivered: ["true", "Delivered", ""] },
        posts: ["document.update", "document.update"],
    });
});

test("send a page of calls as one NDJSON batch with the secret in the endpoint's header", async () => {
    // subscribe in NDJSON with the secret in a header, then create and delete a document
    const fixture = await TestMessage.open();
    await fixture.subscribe({
        events: ["document.create", "document.delete"],
        format: "ndjson",
        authentication: { kind: "header", name: "api-key" },
        secret: await fixture.secret(SECRET_VALUE),
    });
    const history = documents(fixture);
    const created = await history.create("draft");
    await history.delete(created.id);
    await fixture.simulation.idle();

    // post both calls as lines of JSON in the pages they landed in, with the secret in the header and no signature
    const posts = fixture.hooks.requests;
    expect({
        methods: posts.flatMap((post) =>
            post.body
                .trim()
                .split("\n")
                .map((line) => BatchRecord.parse(JSON.parse(line)).type),
        ),
        headers: [
            ...new Set(
                posts.map((post) =>
                    JSON.stringify([
                        post.headers.get("api-key"),
                        post.headers.get("webhook-signature"),
                        post.headers.get("content-type"),
                    ]),
                ),
            ),
        ],
    }).toEqual({
        methods: ["document.create", "document.delete"],
        headers: [JSON.stringify([SECRET_VALUE, null, "application/x-ndjson"])],
    });
});

test("hold a disabled endpoint's position and deliver only the calls after it once enabled again", async () => {
    // deliver a first call
    const fixture = await TestMessage.open();
    const id = await fixture.subscribe({
        ...EVENTS,
        events: ["document.create", "document.delete"],
        secret: await fixture.secret(SECRET_VALUE),
    });
    const history = documents(fixture);
    const created = await history.create("draft");
    await fixture.simulation.idle();
    const before = await delivery(fixture, id);

    // disable the endpoint, delete the document while it rests, and enable it again
    const client = fixture.owner.client(messagePackage);
    const update = (status: "enabled" | "disabled") =>
        client.endpoint.update({
            scope: fixture.space.id,
            id,
            requestId: RequestId.create(fixture.environment),
            status,
        });
    await update("disabled");
    await history.delete(created.id);
    await fixture.simulation.idle();
    const resting = fixture.hooks.requests.length;
    await update("enabled");
    await fixture.simulation.idle();
    expect({
        before,
        resting,
        after: await delivery(fixture, id),
        types: await readPosts(fixture),
    }).toEqual({
        before: { messages: 1, delivered: ["true", "Delivered", ""] },
        resting: 1,
        after: { messages: 2, delivered: ["true", "Delivered", ""] },
        types: ["document.create", "document.delete"],
    });
});

test("mark an endpoint failing once the message controller gives up a message its URL refuses every time", async () => {
    // refuse every post with a server failure
    const fixture = await TestMessage.open({ hooks: Array.from({ length: 8 }, () => 500) });
    const id = await fixture.subscribe({
        ...EVENTS,
        events: ["document.create"],
        format: "json",
        secret: await fixture.secret(SECRET_VALUE),
    });
    await documents(fixture).create("draft");
    await fixture.simulation.idle();

    // give the batch up after the retries, and flag the endpoint with the URL's answer
    const [sent] = await fixture.served.database
        .select({ id: message.table.id })
        .from(message.table);
    for (let attempt = 1; attempt < 8; attempt++) {
        await fixture.retry(message.identifier(String(sent?.id)));
    }
    await fixture.simulation.idle();
    expect(await delivery(fixture, id)).toEqual({
        messages: 1,
        delivered: ["false", "Failing", "PROVIDER_UNAVAILABLE: answered 500"],
    });
});

test("fail a webhook message once its endpoint is disabled or deleted, or answers with a redirect it never follows", async () => {
    // subscribe three endpoints to one call each, the first two answered with a server failure
    const fixture = await TestMessage.open({ hooks: [500, 500, 302] });
    const stored = await fixture.secret(SECRET_VALUE);
    const subscribe = (event: string) =>
        fixture.subscribe({ ...EVENTS, events: [event], secret: stored });
    const disabled = await subscribe("document.create");
    const deleted = await subscribe("document.update");
    const redirected = await subscribe("document.delete");
    const client = fixture.owner.client(messagePackage);
    const target = (id: string) => ({
        scope: fixture.space.id,
        id,
        requestId: RequestId.create(fixture.environment),
    });
    const sentTo = async (id: string) => {
        const [row] = await fixture.served.database
            .select({ id: message.table.id })
            .from(message.table)
            .where(eq(message.table.sourceId, id));

        return message.identifier(String(row?.id));
    };

    // defer the first message, disable its endpoint, and try it again
    const history = documents(fixture);
    const created = await history.create("draft");
    await fixture.simulation.idle();
    await client.endpoint.update({ ...target(disabled), status: "disabled" });
    await fixture.retry(await sentTo(disabled));

    // defer the second message, delete its endpoint, and try it again
    await history.rename(created.id, "final");
    await fixture.simulation.idle();
    await client.endpoint.delete(target(deleted));
    await fixture.retry(await sentTo(deleted));

    // post the third message once, answered with a redirect
    await history.delete(created.id);
    await fixture.simulation.idle();
    const read = async (id: string) => {
        const sending = await fixture.sending(await sentTo(id));

        return [sending.status, sending.reason, sending.attempts];
    };
    expect({
        disabled: await read(disabled),
        deleted: await read(deleted),
        redirected: await read(redirected),
        posts: fixture.hooks.requests.length,
    }).toEqual({
        disabled: ["failed", "RECIPIENT_DISABLED", ["deferred", "failed"]],
        deleted: ["failed", "RECIPIENT_GONE", ["deferred", "failed"]],
        redirected: ["failed", "MESSAGE_REJECTED", ["failed"]],
        posts: 3,
    });
});

test("refuse an endpoint whose secret header is one every webhook post sets itself", async () => {
    const fixture = await TestMessage.open();
    const stored = await fixture.secret(SECRET_VALUE);
    const subscribe = (name: string) =>
        refusal(
            fixture.subscribe({
                ...EVENTS,
                authentication: { kind: "header", name },
                secret: stored,
            }),
        );
    expect([await subscribe("Webhook-Signature"), await subscribe("Content-Type")]).toEqual([
        ["BAD_REQUEST", "HEADER_RESERVED", "every webhook post sets Webhook-Signature itself"],
        ["BAD_REQUEST", "HEADER_RESERVED", "every webhook post sets Content-Type itself"],
    ]);
});
