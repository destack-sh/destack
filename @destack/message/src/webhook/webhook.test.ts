import type { CallServer } from "@destack/object";
import { UNQUERIED } from "../test/index.ts";
import { expect, test } from "@destack/test";
import { PackageId } from "@destack/package";
import { aligned, present, schema } from "@destack/schema";
import { type ObjectReference, Subject } from "@destack/sync";
import type { MessageProvider } from "../provider/index.ts";
import { WebhookSignature, webhookProvider } from "./webhook.ts";

/** The specification's example signing secret, kept in a vault. */
const SECRET = "whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw";

/** A webhook message to an endpoint, signed with the vault secret holding the example secret. */
const message = {
    id: schema.identifier("message").parse("message-01996ab0-0000-7000-8000-000000000001"),
    scope: "account-01996ab0-0000-7000-8000-000000000002",
    createdAt: Date.UTC(2026, 9, 5),
    to: {
        channel: "webhook",
        url: "https://hooks.example.com/destack",
        authentication: { kind: "signature" },
        secret: {
            packageId: PackageId.parse("package-01996ab0-0000-7000-8000-000000000003"),
            type: "secret",
            scope: "space-01996ab0-0000-7000-8000-000000000004",
            id: "secret-01996ab0-0000-7000-8000-000000000005",
        },
        reader: {
            packageId: PackageId.parse("package-01996ab0-0000-7000-8000-000000000006"),
            type: "user",
            scope: "universe",
            id: "user-01996ab0-0000-7000-8000-000000000007",
        },
    },
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

/** Post webhooks to an endpoint answering with each status in turn, keeping the requests and the secrets read. */
function postTo(statuses: readonly number[]) {
    const requests: { url: string; headers: Headers; body: string }[] = [];
    const read: [ObjectReference, Subject][] = [];
    const vault = async (
        _server: Pick<CallServer, "query" | "clock">,
        secret: ObjectReference,
        reader: Subject,
    ) => {
        read.push([secret, reader]);

        return SECRET;
    };
    const provider = webhookProvider(vault, async (request) => {
        requests.push({
            url: request.url,
            headers: new Headers(request.headers),
            body: await request.text(),
        });
        const status = present(statuses[requests.length - 1], "a status for each post");

        return new Response(null, { status });
    });

    return { requests, read, send: () => provider.send(message, UNQUERIED) };
}

test("post an event signed over its identifier, time and body with the secret read as the message's reader", async () => {
    const { requests, read, send } = postTo([204]);
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
        read,
    }).toEqual({
        url: "https://hooks.example.com/destack",
        id: message.id,
        body: {
            type: "issue.regressed",
            timestamp: "2026-10-05T00:00:00.000Z",
            data: { issue: "issue-1" },
        },
        isSigned: true,
        read: [[message.to.secret, message.to.reader]],
    });
});

test("read the endpoint's answers as sent, retried or failed", async () => {
    const { send } = postTo([204, 503, 429, 410]);
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
