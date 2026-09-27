import { expect, test } from "@destack/test";
import { describeWebhook } from "../inspect/index.ts";
import { GitHubSignature, StandardSignature, WEBHOOK_SIGNATURES } from "./signature.ts";
import { defineWebhook } from "./webhook.ts";
import { defineService } from "../declare/service.ts";

/** The secret, message and signature the Standard Webhooks reference library signs in its "sign function works" test. */
const STANDARD_EXAMPLE = {
    secret: "whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw",
    message: {
        id: "msg_p5jXN8AQM9LWM0D4loKWxJek",
        event: "",
        body: '{"test": 2432232314}',
        sentAt: 1614265330 * 1000,
    },
    signature: "v1,g0hM9SsE+OTPJTGt/tmIKtSyZlE3uFJELVlNIOLJ1OE=",
};

/** The secret, payload and signature GitHub documents under "Testing the webhook payload validation". */
const GITHUB_EXAMPLE = {
    secret: "It's a Secret to Everybody",
    body: "Hello, World!",
    signature: "sha256=757107ea0eb2509fc211221cce984b8a37570b6d7586c22c46f4379c8b043e17",
};

/** The time the fixture deliveries arrive, a minute after the Standard Webhooks example was sent. */
const NOW = STANDARD_EXAMPLE.message.sentAt + 60_000;

/** Post a signed body the way its sender does. */
function post(headers: Headers, body: string): Request {
    return new Request("https://hooks.test/webhooks/github", { method: "POST", headers, body });
}

test("sign the Standard Webhooks and GitHub reference examples exactly", async () => {
    // sign the Standard Webhooks example message and the GitHub example body
    const standard = await new StandardSignature().sign(
        STANDARD_EXAMPLE.message,
        STANDARD_EXAMPLE.secret,
    );
    const github = await new GitHubSignature().sign(
        { id: "delivery-1", event: "push", body: GITHUB_EXAMPLE.body, sentAt: NOW },
        GITHUB_EXAMPLE.secret,
    );
    expect([...standard.entries()]).toEqual([
        ["webhook-id", "msg_p5jXN8AQM9LWM0D4loKWxJek"],
        ["webhook-signature", STANDARD_EXAMPLE.signature],
        ["webhook-timestamp", "1614265330"],
    ]);
    expect([...github.entries()]).toEqual([
        ["x-github-delivery", "delivery-1"],
        ["x-github-event", "push"],
        ["x-hub-signature-256", GITHUB_EXAMPLE.signature],
    ]);
});

test("verify a Standard Webhooks delivery and refuse forged, stale and unsigned ones", async () => {
    // accept the reference signature among others, reading the event from the payload's type
    const signature = WEBHOOK_SIGNATURES.standard;
    const body = '{"type":"repository.pushed","data":{"ref":"refs/heads/main"}}';
    const message = { ...STANDARD_EXAMPLE.message, body };
    const headers = await signature.sign(message, STANDARD_EXAMPLE.secret);
    headers.set("webhook-signature", `v1,bm90IGEgc2lnbmF0dXJl ${headers.get("webhook-signature")}`);
    expect(await signature.verify(post(headers, body), STANDARD_EXAMPLE.secret, NOW)).toEqual({
        id: "msg_p5jXN8AQM9LWM0D4loKWxJek",
        event: "repository.pushed",
        payload: { type: "repository.pushed", data: { ref: "refs/heads/main" } },
        receivedAt: NOW,
    });

    // refuse a changed body, a delivery beyond five minutes and a request without its headers
    const refusal = (request: Request, now: number) =>
        signature.verify(request, STANDARD_EXAMPLE.secret, now).then(
            () => "accepted",
            (error: { code: string; message: string }) => `${error.code}: ${error.message}`,
        );
    expect([
        await refusal(post(headers, body.replace("main", "next")), NOW),
        await refusal(post(headers, body), message.sentAt + 5 * 60_000 + 1000),
        await refusal(post(new Headers(), body), NOW),
    ]).toEqual([
        "UNAUTHORIZED: webhook signature does not match",
        "UNAUTHORIZED: webhook timestamp is outside the tolerance",
        "UNAUTHORIZED: missing standard webhook headers",
    ]);
});

test("verify a GitHub delivery and refuse a signature made with another secret", async () => {
    // accept the signed push with its delivery identifier and event
    const signature = WEBHOOK_SIGNATURES.github;
    const body = '{"ref":"refs/heads/main","after":"aa218f56b14c9653891f9e74264a383fa43fefbd"}';
    const message = {
        id: "72d3162e-cc78-11e3-81ab-4c9367dc0958",
        event: "push",
        body,
        sentAt: NOW,
    };
    const headers = await signature.sign(message, GITHUB_EXAMPLE.secret);
    expect(await signature.verify(post(headers, body), GITHUB_EXAMPLE.secret, NOW)).toEqual({
        id: "72d3162e-cc78-11e3-81ab-4c9367dc0958",
        event: "push",
        payload: { ref: "refs/heads/main", after: "aa218f56b14c9653891f9e74264a383fa43fefbd" },
        receivedAt: NOW,
    });

    // refuse the same delivery signed with another secret
    const forged = await signature.sign(message, "another secret");
    await expect(
        signature.verify(post(forged, body), GITHUB_EXAMPLE.secret, NOW),
    ).rejects.toMatchObject({ code: "UNAUTHORIZED", message: "webhook signature does not match" });
});

test("describe a declared webhook with its verification and secret for the manifest", () => {
    const secret = { package: defineService("hooks", {}).package, name: "github-webhook" };
    const webhook = defineWebhook({ name: "github", verification: "github", secret });

    expect(describeWebhook(webhook)).toEqual({
        name: "github",
        version: 1,
        verification: "github",
        secret: { packageId: webhook.package.id, name: "github-webhook" },
    });
});
