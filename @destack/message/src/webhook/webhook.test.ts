import { storeSecret } from "../test/index.ts";
import { mailProvider } from "../email/index.ts";
import { AccessFixture } from "@destack/access/test";
import { account, space } from "@destack/account/object";
import { defineDatabase } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { defineServer, ObjectStore } from "@destack/object/server";
import { testEvents } from "@destack/object/test";
import { Package } from "@destack/package";
import { testCallKey } from "@destack/runtime/test";
import { Clock } from "@destack/schema";
import { Scope } from "@destack/sync";
import { onTestFinished, refusal } from "@destack/test";
import vaultServer from "@destack/vault/server";
import { vaultDatabase } from "@destack/vault/test";
import messagePackage from "../package.ts";
import { expect, test } from "@destack/test";
import { PackageId } from "@destack/package";
import { aligned, present, schema } from "@destack/schema";
import { ObjectReference, Subject } from "@destack/sync";
import type { MessageProvider } from "../provider/index.ts";
import { WebhookSignature, webhookProvider } from "./webhook.ts";
import { principal } from "@destack/access";
import { type Endpoint, endpoint } from "../object/endpoint.ts";
import { mail } from "../object/index.ts";

/** The machine serving the space and its vaults. */
const MACHINE = schema.identifier("machine").parse("machine-01996ab0-0000-7000-8000-0000000000a1");

/** The specification's example signing secret, kept in a vault. */
const SECRET = "whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw";

/** The endpoint every message posts to, whose vault secret is the example secret. */
const ENDPOINT: Endpoint = {
    id: schema.identifier("endpoint").parse("endpoint-01996ab0-0000-7000-8000-000000000008"),
    scope: schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000004"),
    url: "https://hooks.example.com/destack",
    events: "all",
    format: "event",
    authentication: { kind: "signature" },
    secret: {
        packageId: PackageId.parse("package-01996ab0-0000-7000-8000-000000000003"),
        type: "secret",
        scope: "space-01996ab0-0000-7000-8000-000000000004",
        id: "secret-01996ab0-0000-7000-8000-000000000005",
    },
    status: "enabled",
    conditions: {},
    generation: 1,
    observedGeneration: 1,
    revision: 1,
    tags: {},
    deletedAt: null,
    ...ObjectReference.columns("deletedBy", undefined),
    createdAt: 0,
    ...ObjectReference.columns("createdBy", undefined),
    updatedAt: 0,
    ...ObjectReference.columns("updatedBy", undefined),
};

/** A webhook message to the endpoint, which its source names. */
const message = {
    id: schema.identifier("message").parse("message-01996ab0-0000-7000-8000-000000000001"),
    scope: "space-01996ab0-0000-7000-8000-000000000004",
    createdAt: Date.UTC(2026, 9, 5),
    to: { channel: "webhook" },
    source: endpoint.reference(ENDPOINT.scope, ENDPOINT.id),
    content: {
        channel: "webhook",
        type: "issue.regressed",
        format: "event",
        data: { issue: "issue-1" },
    },
} as const satisfies Parameters<MessageProvider["send"]>[0];

test("sign as the Standard Webhooks specification's example does", async () => {
    // reproduce the specification's signature of its example message
    expect(
        await WebhookSignature.sign(
            "whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw",
            "msg_p5jXN8AQM9LWM0D4loKWxJek",
            "1614265330",
            '{"test": 2432232314}',
        ),
    ).toBe("v1,g0hM9SsE+OTPJTGt/tmIKtSyZlE3uFJELVlNIOLJ1OE=");
});

/** The account whose space has the endpoint. */
const ACCOUNT = schema.identifier("account").parse("account-01996ab0-0000-7000-8000-000000000009");

/** The message installation serving the endpoint, as which its messages go out. */
const INSTALLATION = principal.installation.reference(
    "space-01996ab0-0000-7000-8000-000000000004",
    "installation-01996ab0-0000-7000-8000-00000000000b",
);

/** Another installation of the space, which writes the secret without the message installation. */
const WRITER = principal.installation.reference(
    "space-01996ab0-0000-7000-8000-000000000004",
    "installation-01996ab0-0000-7000-8000-00000000000c",
);

/** Post webhooks to the endpoint answering each status in turn, its secret kept for a consumer. */
async function postTo(statuses: readonly number[], consumer: Subject = INSTALLATION) {
    // serve the endpoint and its vault from a space's store, owned by the space
    const storage = await TestDatabase.create(
        "sqlite",
        defineDatabase({
            name: "main",
            tables: [...endpoint.tables, ...vaultDatabase.tables],
        }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const { database } = storage;
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: INSTALLATION,
        servers: [
            defineServer(messagePackage, {
                objects: [endpoint],
                providers: [mailProvider("fixture", mail)],
            }),
            vaultServer,
        ],
        policies: [space],
        database,
        callKey: testCallKey,
        events: await testEvents(database),
        origin: { package: Package.of(messagePackage) },
        machine: MACHINE,
    });
    const access = new AccessFixture(database);
    const reference = space.reference(ACCOUNT, ENDPOINT.scope);
    await access.replicateScope(account.reference(Scope.universe.id, ACCOUNT));
    await access.replicateScope(reference);
    await access.bindOwner(reference, principal.space.reference(Scope.universe.id, ENDPOINT.scope));

    // let the installation read the space's endpoints, as its stack grants it
    const reading = {
        name: "deliverer",
        description: "Reads the space's endpoints",
        permissions: [endpoint.permission("read")],
    };
    await access.replicateRole(reference, reading, INSTALLATION);

    // store the secret for its consumer, and the endpoint pointing at it
    await storeSecret(store, ENDPOINT.secret, SECRET, consumer);
    await database.insert(endpoint.table).values(ENDPOINT);

    // post through the provider, answering each status in turn
    const requests: { url: string; headers: Headers; body: string }[] = [];
    const provider = webhookProvider(async (request) => {
        requests.push({
            url: request.url,
            headers: new Headers(request.headers),
            body: await request.text(),
        });
        const status = present(statuses[requests.length - 1], "a status for each post");

        return new Response(null, { status });
    });

    return { requests, send: () => provider.send(message, store) };
}

test("post an event to its endpoint signed over its identifier, time and body, with the secret its installation consumes", async () => {
    const { requests, send } = await postTo([204]);
    await send();

    // post the event with a signature over its identifier, time and body
    const first = aligned(requests, 0);
    expect({
        url: first.url,
        id: first.headers.get("webhook-id"),
        body: schema.json().parse(JSON.parse(first.body)),
        isSigned:
            first.headers.get("webhook-signature") ===
            (await WebhookSignature.sign(
                SECRET,
                message.id,
                String(first.headers.get("webhook-timestamp")),
                first.body,
            )),
    }).toEqual({
        url: "https://hooks.example.com/destack",
        id: message.id,
        body: {
            type: "issue.regressed",
            timestamp: "2026-10-05T00:00:00.000Z",
            data: { issue: "issue-1" },
        },
        isSigned: true,
    });
});

test("refuse posting to an endpoint whose installation does not consume its secret", async () => {
    // let only another installation consume the endpoint's secret, and post nothing
    const { requests, send } = await postTo([204], WRITER);
    expect({ refused: await refusal(send()), requests }).toEqual({
        refused: ["NOT_FOUND", `no secret ${ENDPOINT.secret.id}`],
        requests: [],
    });
});

test("read the endpoint's answers as sent, retried or failed", async () => {
    const { send } = await postTo([204, 503, 429, 410]);
    const outcomes = [];
    for (let sent = 0; sent < 4; sent++) {
        outcomes.push(await send());
    }

    expect(outcomes).toEqual([
        { kind: "sent" },
        { kind: "retry", error: { code: "503", message: "" } },
        { kind: "retry", error: { code: "429", message: "" } },
        { kind: "failed", error: { code: "gone", message: "" } },
    ]);
});
