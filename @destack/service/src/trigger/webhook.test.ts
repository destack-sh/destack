import { expect, test } from "@destack/test";
import { describeTrigger } from "../inspect/index.ts";
import { ResourceContext } from "@destack/resource/context";
import { WebhookOn } from "./webhook.ts";
import { defineTrigger, type WebhookTrigger } from "./trigger.ts";
import {
    GitHubSignature,
    StandardSignature,
    WEBHOOK_SIGNATURES,
    type WebhookDelivery,
} from "./webhook.ts";

/** Push the delivered branch to the repository its route names. */
function push(delivery: WebhookDelivery) {
    return {
        method: "repository.push",
        input: { repository: delivery.parameters.repository!, payload: delivery.payload },
        release: "2026.9.0",
    };
}

/** The Standard Webhooks reference example of "sign function works". */
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

/** The GitHub example of "Testing the webhook payload validation". */
const GITHUB_EXAMPLE = {
    secret: "It's a Secret to Everybody",
    body: "Hello, World!",
    signature: "sha256=757107ea0eb2509fc211221cce984b8a37570b6d7586c22c46f4379c8b043e17",
};

/** The delivery time, a minute after the Standard Webhooks example. */
const NOW = STANDARD_EXAMPLE.message.sentAt + 60_000;

/** Post a signed body. */
function post(headers: Headers, body: string): Request {
    return new Request("https://hooks.test/webhooks/github", { method: "POST", headers, body });
}

/** Read the body digest a GitHub signature carries, which identifies its delivery. */
function digestOf(headers: Headers): string {
    return headers.get("x-hub-signature-256")!.slice("sha256=".length);
}

test("sign the Standard Webhooks and GitHub reference examples exactly", async () => {
    // sign both examples
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
    // accept the reference signature among others
    const signature = WEBHOOK_SIGNATURES.standard;
    const body = '{"type":"repository.pushed","data":{"ref":"refs/heads/main"}}';
    const message = { ...STANDARD_EXAMPLE.message, body };
    const headers = await signature.sign(message, STANDARD_EXAMPLE.secret);
    headers.set("webhook-signature", `v1,bm90IGEgc2lnbmF0dXJl ${headers.get("webhook-signature")}`);
    expect(await signature.verify(post(headers, body), STANDARD_EXAMPLE.secret, {}, NOW)).toEqual({
        id: "msg_p5jXN8AQM9LWM0D4loKWxJek",
        event: "repository.pushed",
        payload: { type: "repository.pushed", data: { ref: "refs/heads/main" } },
        parameters: {},
        receivedAt: NOW,
    });

    // refuse a changed body, a stale delivery and missing headers
    const refusal = (request: Request, now: number) =>
        signature.verify(request, STANDARD_EXAMPLE.secret, {}, now).then(
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
    // accept the signed push
    const signature = WEBHOOK_SIGNATURES.github;
    const body = '{"ref":"refs/heads/main","after":"aa218f56b14c9653891f9e74264a383fa43fefbd"}';
    const message = {
        id: "72d3162e-cc78-11e3-81ab-4c9367dc0958",
        event: "push",
        body,
        sentAt: NOW,
    };
    const headers = await signature.sign(message, GITHUB_EXAMPLE.secret);

    // know the delivery by its signed body, since GitHub signs no delivery header
    expect(await signature.verify(post(headers, body), GITHUB_EXAMPLE.secret, {}, NOW)).toEqual({
        id: digestOf(headers),
        event: "push",
        payload: { ref: "refs/heads/main", after: "aa218f56b14c9653891f9e74264a383fa43fefbd" },
        parameters: {},
        receivedAt: NOW,
    });

    // refuse another secret
    const forged = await signature.sign(message, "another secret");
    await expect(
        signature.verify(post(forged, body), GITHUB_EXAMPLE.secret, {}, NOW),
    ).rejects.toMatchObject({ code: "UNAUTHORIZED", message: "webhook signature does not match" });
});

test("describe a declared webhook trigger with its verification and route for the manifest", () => {
    const trigger = defineTrigger({
        name: "github",
        on: {
            webhook: {
                verification: "github",
                route: "/{repository}",
                secret: async () => "secret",
            },
        },
        call: push,
    });

    expect(describeTrigger(trigger)).toEqual({
        name: "github",
        on: { webhook: { verification: "github", route: "/{repository}" } },
    });
});

test("receive a delivery with its route's parameters and the secret they resolve, and build its call", async () => {
    // resolve each repository's secret
    const requested: unknown[] = [];
    const trigger = defineTrigger({
        name: "pushes",
        on: {
            webhook: {
                verification: "github",
                route: "/repositories/{repository}",
                secret: async (parameters) => {
                    requested.push(parameters);

                    return parameters.repository === "acme site"
                        ? GITHUB_EXAMPLE.secret
                        : "another secret";
                },
            },
        },
        call: push,
    }) as WebhookTrigger;
    const resources = new ResourceContext();

    // accept the repository's signed delivery
    const body = '{"ref":"refs/heads/main"}';
    const message = { id: "delivery-1", event: "push", body, sentAt: NOW };
    const headers = await WEBHOOK_SIGNATURES.github.sign(message, GITHUB_EXAMPLE.secret);
    const delivery = await WebhookOn.receive(
        trigger.on.webhook,
        post(headers, body),
        "/repositories/acme%20site",
        NOW,
        resources,
    );
    expect([delivery, trigger.call(delivery)]).toEqual([
        {
            id: digestOf(headers),
            event: "push",
            payload: { ref: "refs/heads/main" },
            parameters: { repository: "acme site" },
            receivedAt: NOW,
        },
        {
            method: "repository.push",
            input: { repository: "acme site", payload: { ref: "refs/heads/main" } },
            release: "2026.9.0",
        },
    ]);

    // refuse another repository's secret and paths outside the route
    const refusal = (path: string) =>
        WebhookOn.receive(trigger.on.webhook, post(headers, body), path, NOW, resources).then(
            () => "accepted",
            (error: { code: string; message: string }) => `${error.code}: ${error.message}`,
        );
    expect([
        await refusal("/repositories/other"),
        await refusal("/repositories"),
        await refusal("/repositories/acme/site"),
        await refusal("/projects/acme"),
        await refusal("/repositories/%E0"),
    ]).toEqual([
        "UNAUTHORIZED: webhook signature does not match",
        "NOT_FOUND: webhook route does not match: /repositories",
        "NOT_FOUND: webhook route does not match: /repositories/acme/site",
        "NOT_FOUND: webhook route does not match: /projects/acme",
        "NOT_FOUND: webhook route does not match: /repositories/%E0",
    ]);
    expect(requested).toEqual([{ repository: "acme site" }, { repository: "other" }]);
});

test("refuse webhook routes with malformed segments or a repeated parameter", () => {
    const refusal = (route: string) => {
        try {
            defineTrigger({
                name: "pushes",
                on: { webhook: { verification: "github", route, secret: async () => "secret" } },
                call: push,
            });

            return "accepted";
        } catch (error) {
            return error instanceof TypeError ? error.message : "invalid route";
        }
    };

    expect([
        refusal("/"),
        refusal(""),
        refusal("/{repository}/"),
        refusal("/{Repository}"),
        refusal("/{repository}/{repository}"),
    ]).toEqual([
        "accepted",
        "invalid route",
        "invalid route",
        "invalid route",
        "webhook route repeats a parameter: /{repository}/{repository}",
    ]);
});
