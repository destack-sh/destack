import { expect, refusal, test } from "@destack/test";
import { Failure } from "@destack/error";
import { schema } from "@destack/schema";
import { describeTrigger } from "../inspect/index.ts";
import { ResourceContext } from "@destack/resource/context";
import { WebhookReceiver } from "./webhook.ts";
import { WebhookTrigger } from "./trigger.ts";
import { StandardSignature, type WebhookDelivery } from "./webhook.ts";
import { testKeys, testSimulation } from "@destack/object/test";

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
    const world = testSimulation().environment("sender");
    const standard = await new StandardSignature().sign(
        STANDARD_EXAMPLE.message,
        STANDARD_EXAMPLE.secret,
        world.crypto,
    );
    expect([...standard.entries()]).toEqual([
        ["webhook-id", "msg_p5jXN8AQM9LWM0D4loKWxJek"],
        ["webhook-signature", STANDARD_EXAMPLE.signature],
        ["webhook-timestamp", "1614265330"],
    ]);
});

test("read a verified Standard Webhooks delivery's event type from its payload", async () => {
    // accept the reference signature among others
    const { crypto } = testSimulation().environment("hooks");
    const signature = new StandardSignature();
    const body = '{"type":"repository.pushed","data":{"ref":"refs/heads/main"}}';
    const message = { ...STANDARD_EXAMPLE.message, body };
    const headers = await signature.sign(message, STANDARD_EXAMPLE.secret, crypto);
    headers.set("webhook-signature", `v1,bm90IGEgc2lnbmF0dXJl ${headers.get("webhook-signature")}`);
    const verified = await signature.verify(
        post(headers, body),
        STANDARD_EXAMPLE.secret,
        {},
        NOW,
        crypto,
    );
    expect(verified).toEqual({
        id: "msg_p5jXN8AQM9LWM0D4loKWxJek",
        event: "repository.pushed",
        payload: { type: "repository.pushed", data: { ref: "refs/heads/main" } },
        parameters: {},
        receivedAt: NOW,
    });

    // refuse a payload without an event type
    const untyped = '{"data":{}}';
    const untypedHeaders = await signature.sign(
        { ...message, body: untyped },
        STANDARD_EXAMPLE.secret,
        crypto,
    );
    expect(
        await refusal(
            signature.verify(
                post(untypedHeaders, untyped),
                STANDARD_EXAMPLE.secret,
                {},
                NOW,
                crypto,
            ),
        ),
    ).toEqual(["BAD_REQUEST", "EVENT_TYPE_MISSING", "standard webhook payload has no type"]);
});

test("describe a declared webhook trigger with its verification and path for the manifest", () => {
    const trigger = new WebhookTrigger("pushes", {
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
    const world = testSimulation().environment("hooks");
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
    const trigger = new WebhookTrigger("pushes", {
        webhook,
        input: pushed,
    });
    const context = {
        resources: new ResourceContext(),
        keys: await testKeys(world),
        environment: world,
    };

    // accept the repository's signed delivery
    const body = '{"type":"push","ref":"refs/heads/main"}';
    const message = { id: "delivery-1", event: "push", body, sentAt: NOW };
    const headers = await new StandardSignature().sign(
        message,
        STANDARD_EXAMPLE.secret,
        world.crypto,
    );
    const receiver = new WebhookReceiver(webhook, webhook.signature.verifier(context), context);
    const delivery = await receiver.receive(post(headers, body), "/repositories/acme%20site", NOW);
    expect([delivery, pushed(delivery), trigger.webhook]).toEqual([
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
    const verdict = (path: string) => refusal(receiver.receive(post(headers, body), path, NOW));
    expect([
        await verdict("/repositories/other"),
        await verdict("/repositories"),
        await verdict("/repositories/acme/site"),
        await verdict("/projects/acme"),
        await verdict("/repositories/%E0"),
    ]).toEqual([
        ["UNAUTHORIZED", "SIGNATURE_MISMATCH", "webhook signature does not match"],
        ["NOT_FOUND", "PATH_MISMATCH", "webhook path does not match: /repositories"],
        ["NOT_FOUND", "PATH_MISMATCH", "webhook path does not match: /repositories/acme/site"],
        ["NOT_FOUND", "PATH_MISMATCH", "webhook path does not match: /projects/acme"],
        ["NOT_FOUND", "PATH_MISMATCH", "webhook path does not match: /repositories/%E0"],
    ]);
    expect(requested).toEqual([{ repository: "acme site" }, { repository: "other" }]);
});

test("refuse webhook paths with malformed segments or a repeated parameter", () => {
    const verdict = (path: string) => {
        try {
            const trigger = new WebhookTrigger("pushes", {
                webhook: {
                    signature: new StandardSignature(),
                    path,
                    secret: async () => "secret",
                },
                input: pushed,
            });

            return `accepted ${trigger.webhook.path}`;
        } catch (error) {
            // read a refused path by its schema issues, and a repeated parameter by its message
            return error instanceof schema.Error
                ? error.issues.map((issue) => `${issue.path.join(".")}: ${issue.code}`)
                : Failure.of(error).message;
        }
    };

    expect([
        verdict("/"),
        verdict(""),
        verdict("/{repository}/"),
        verdict("/{Repository}"),
        verdict("/{repository}/{repository}"),
    ]).toEqual([
        "accepted /",
        ["path: invalid_format"],
        ["path: invalid_format"],
        ["path: invalid_format"],
        "webhook path repeats a parameter: /{repository}/{repository}",
    ]);
});
