import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
import type { MessageProvider } from "../provider/index.ts";
import { WebhookSignature, webhookProvider } from "./webhook.ts";

/** A webhook message to an endpoint, signed with the specification's example secret. */
const message = {
    id: schema.identifier("message").parse("message-01996ab0-0000-7000-8000-000000000001"),
    createdAt: Date.UTC(2026, 9, 5),
    to: { channel: "webhook", url: "https://hooks.example.com/destack" },
    content: { channel: "webhook", type: "issue.regressed", data: { issue: "issue-1" } },
    secret: "whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw",
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

test("post a signed event, and read the endpoint's answer as sent, retried or failed", async () => {
    // answer with each status in turn, keeping the requests
    const requests: { url: string; headers: Headers; body: string }[] = [];
    const statuses = [204, 503, 429, 410];
    const provider = webhookProvider(async (url, request) => {
        requests.push({
            url,
            headers: new Headers(request.headers),
            body: typeof request.body === "string" ? request.body : "",
        });

        return new Response(null, { status: statuses[requests.length - 1] ?? 500 });
    });
    const outcomes = [];
    for (let sent = 0; sent < statuses.length; sent++) {
        outcomes.push(await provider.send(message));
    }

    // post the event with a signature over its identifier, time and body
    const [first] = requests;
    expect(
        first && {
            url: first.url,
            id: first.headers.get("webhook-id"),
            body: schema.json().parse(JSON.parse(first.body)),
            isSigned:
                first.headers.get("webhook-signature") ===
                (await WebhookSignature.sign(
                    message.secret,
                    message.id,
                    String(first.headers.get("webhook-timestamp")),
                    first.body,
                )),
        },
    ).toEqual({
        url: "https://hooks.example.com/destack",
        id: message.id,
        body: {
            type: "issue.regressed",
            timestamp: "2026-10-05T00:00:00.000Z",
            data: { issue: "issue-1" },
        },
        isSigned: true,
    });

    // send on success, retry server failures and throttling, and fail on a gone endpoint
    expect(outcomes.map((outcome) => outcome.outcome)).toEqual([
        "sent",
        "retry",
        "retry",
        "failed",
    ]);
});
