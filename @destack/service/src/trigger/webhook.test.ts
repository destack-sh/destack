import { expect, refusal, test } from "@destack/test";
import { describeTrigger } from "../inspect/index.ts";
import { ResourceContext } from "@destack/resource/context";
import { WebhookOn } from "./webhook.ts";
import { defineTrigger, Trigger } from "./trigger.ts";
import { StandardSignature, type WebhookDelivery } from "./webhook.ts";
import { GitHubSignature } from "../github/index.ts";

/** Push the delivered branch to the repository its route names. */
function push(delivery: WebhookDelivery) {
    // require the route's repository
    const repository = delivery.parameters["repository"];
    if (repository === undefined) {
        throw new TypeError("a push delivery's route has a repository");
    }

    return {
        method: "repository.push",
        input: { repository, payload: delivery.payload },
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

/** Read the body digest of a GitHub signature, which identifies its delivery. */
function digestOf(headers: Headers): string {
    // require the signature header
    const signature = headers.get("x-hub-signature-256");
    if (signature === null) {
        throw new TypeError("a GitHub delivery carries a signature");
    }

    return signature.slice("sha256=".length);
}

test("sign the Standard Webhooks reference example exactly", async () => {
    const standard = await new StandardSignature().sign(
        STANDARD_EXAMPLE.message,
        STANDARD_EXAMPLE.secret,
    );
    expect([...standard.entries()]).toEqual([
        ["webhook-id", "msg_p5jXN8AQM9LWM0D4loKWxJek"],
        ["webhook-signature", STANDARD_EXAMPLE.signature],
        ["webhook-timestamp", "1614265330"],
    ]);
});

test("verify a Standard Webhooks delivery and refuse forged, stale and unsigned ones", async () => {
    // accept the reference signature among others
    const signature = new StandardSignature();
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
    const verdict = (request: Request, now: number) =>
        refusal(signature.verify(request, STANDARD_EXAMPLE.secret, {}, now)).then((refused) =>
            refused === "done" ? "accepted" : refused.join(": "),
        );
    expect([
        await verdict(post(headers, body.replace("main", "next")), NOW),
        await verdict(post(headers, body), message.sentAt + 5 * 60_000 + 1000),
        await verdict(post(new Headers(), body), NOW),
    ]).toEqual([
        "UNAUTHORIZED: webhook signature does not match",
        "UNAUTHORIZED: webhook timestamp is outside the tolerance",
        "UNAUTHORIZED: missing standard webhook headers",
    ]);
});

test("describe a declared webhook trigger with its verification and route for the manifest", () => {
    const trigger = defineTrigger({
        name: "github",
        on: {
            webhook: {
                signature: new GitHubSignature(),
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
    const declared = defineTrigger({
        name: "pushes",
        on: {
            webhook: {
                signature: new GitHubSignature(),
                route: "/repositories/{repository}",
                secret: async (parameters) => {
                    requested.push(parameters);

                    return parameters["repository"] === "acme site"
                        ? GITHUB_EXAMPLE.secret
                        : "another secret";
                },
            },
        },
        call: push,
    });
    const trigger = Trigger.of(declared, "webhook");
    if (trigger === undefined) {
        throw new TypeError("a trigger on webhooks fires on webhooks");
    }
    const resources = new ResourceContext();

    // accept the repository's signed delivery
    const body = '{"ref":"refs/heads/main"}';
    const message = { id: "delivery-1", event: "push", body, sentAt: NOW };
    const headers = await new GitHubSignature().sign(message, GITHUB_EXAMPLE.secret);
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
    const verdict = (path: string) =>
        refusal(
            WebhookOn.receive(trigger.on.webhook, post(headers, body), path, NOW, resources),
        ).then((refused) => (refused === "done" ? "accepted" : refused.join(": ")));
    expect([
        await verdict("/repositories/other"),
        await verdict("/repositories"),
        await verdict("/repositories/acme/site"),
        await verdict("/projects/acme"),
        await verdict("/repositories/%E0"),
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
    const verdict = (route: string) => {
        try {
            defineTrigger({
                name: "pushes",
                on: {
                    webhook: {
                        signature: new GitHubSignature(),
                        route,
                        secret: async () => "secret",
                    },
                },
                call: push,
            });

            return "accepted";
        } catch (error) {
            return error instanceof TypeError ? error.message : "invalid route";
        }
    };

    expect([
        verdict("/"),
        verdict(""),
        verdict("/{repository}/"),
        verdict("/{Repository}"),
        verdict("/{repository}/{repository}"),
    ]).toEqual([
        "accepted",
        "invalid route",
        "invalid route",
        "invalid route",
        "webhook route repeats a parameter: /{repository}/{repository}",
    ]);
});
