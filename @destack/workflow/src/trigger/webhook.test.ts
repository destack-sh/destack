import { expect, refusal, test } from "@destack/test";
import { describeTrigger } from "../inspect/index.ts";
import { ResourceContext } from "@destack/resource/context";
import { WebhookOn } from "./webhook.ts";
import { Trigger } from "./trigger.ts";
import { StandardSignature, type WebhookDelivery } from "./webhook.ts";
import { testKeys } from "@destack/object/test";

/** Read the push of the repository in a delivery's path. */
function pushed(delivery: WebhookDelivery) {
    // require the path's repository
    const repository = delivery.parameters["repository"];
    if (repository === undefined) {
        throw new TypeError("a push delivery's path has a repository");
    }

    return { repository, payload: delivery.payload };
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

/** The delivery time, a minute after the Standard Webhooks example. */
const NOW = STANDARD_EXAMPLE.message.sentAt + 60_000;

/** Post a signed body. */
function post(headers: Headers, body: string): Request {
    return new Request("https://hooks.test/webhooks/standard", { method: "POST", headers, body });
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

test("describe a declared webhook trigger with its verification and path for the manifest", () => {
    const trigger = Trigger.of("pushes", {
        kind: "webhook",
        webhook: {
            signature: new StandardSignature(),
            path: "/{repository}",
            secret: async () => STANDARD_EXAMPLE.secret,
        },
        input: pushed,
    });

    expect(describeTrigger(trigger)).toEqual({
        name: "pushes",
        kind: "webhook",
        webhook: { verification: "standard", path: "/{repository}" },
    });
});

test("receive a delivery with its path's parameters and the secret they resolve, and build its call", async () => {
    // resolve each repository's secret
    const requested: unknown[] = [];
    const webhook = {
        signature: new StandardSignature(),
        path: "/repositories/{repository}",
        secret: async (parameters: Readonly<Record<string, string>>) => {
            requested.push(parameters);

            return parameters["repository"] === "acme site"
                ? STANDARD_EXAMPLE.secret
                : "whsec_YW5vdGhlciBzZWNyZXQ=";
        },
    };
    const trigger = Trigger.of("pushes", {
        kind: "webhook",
        webhook,
        input: pushed,
    });
    const context = {
        resources: new ResourceContext(),
        keys: await testKeys(),
    };

    // accept the repository's signed delivery
    const body = '{"type":"push","ref":"refs/heads/main"}';
    const message = { id: "delivery-1", event: "push", body, sentAt: NOW };
    const headers = await new StandardSignature().sign(message, STANDARD_EXAMPLE.secret);
    const verify = webhook.signature.verifier();
    const delivery = await WebhookOn.receive(
        webhook,
        verify,
        post(headers, body),
        "/repositories/acme%20site",
        NOW,
        context,
    );
    expect([delivery, pushed(delivery), trigger.kind === "webhook" && trigger.webhook]).toEqual([
        {
            id: "delivery-1",
            event: "push",
            payload: { type: "push", ref: "refs/heads/main" },
            parameters: { repository: "acme site" },
            receivedAt: NOW,
        },
        { repository: "acme site", payload: { type: "push", ref: "refs/heads/main" } },
        webhook,
    ]);

    // refuse another repository's secret and paths outside the path template
    const verdict = (path: string) =>
        refusal(WebhookOn.receive(webhook, verify, post(headers, body), path, NOW, context)).then(
            (refused) => (refused === "done" ? "accepted" : refused.join(": ")),
        );
    expect([
        await verdict("/repositories/other"),
        await verdict("/repositories"),
        await verdict("/repositories/acme/site"),
        await verdict("/projects/acme"),
        await verdict("/repositories/%E0"),
    ]).toEqual([
        "UNAUTHORIZED: webhook signature does not match",
        "NOT_FOUND: webhook path does not match: /repositories",
        "NOT_FOUND: webhook path does not match: /repositories/acme/site",
        "NOT_FOUND: webhook path does not match: /projects/acme",
        "NOT_FOUND: webhook path does not match: /repositories/%E0",
    ]);
    expect(requested).toEqual([{ repository: "acme site" }, { repository: "other" }]);
});

test("refuse webhook paths with malformed segments or a repeated parameter", () => {
    const verdict = (path: string) => {
        try {
            Trigger.of("pushes", {
                kind: "webhook",
                webhook: { signature: new StandardSignature(), path, secret: async () => "secret" },
                input: pushed,
            });

            return "accepted";
        } catch (error) {
            return error instanceof TypeError ? error.message : "invalid path";
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
        "invalid path",
        "invalid path",
        "invalid path",
        "webhook path repeats a parameter: /{repository}/{repository}",
    ]);
});
