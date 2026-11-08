import { expect, refusal, test } from "@destack/test";
import { Failure } from "@destack/error";
import { schema } from "@destack/schema";
import type {} from "@destack/package/import-meta";
import { ResourceHandle, type Secret } from "@destack/resource";
import { describeTrigger, describeWebhook } from "../inspect/index.ts";
import { WebhookTrigger } from "../trigger/trigger.ts";
import { defineWebhook, WebhookPath, type WebhookDelivery } from "./webhook.ts";
import { StandardSignature } from "./signature.ts";
import { testSimulation } from "@destack/object/test";

/** The module the tests declare their webhooks in: this package. */
const MODULE = { package: import.meta.destack.package };

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
        [STANDARD_EXAMPLE.secret],
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
                [STANDARD_EXAMPLE.secret],
                {},
                NOW,
                crypto,
            ),
        ),
    ).toEqual(["BAD_REQUEST", "EVENT_TYPE_MISSING", "standard webhook payload has no type"]);
});

test("describe a declared webhook with its bound secret, and a trigger naming it by reference", () => {
    // declare a webhook verified with a bound secret, and a trigger firing on it
    const secret = new ResourceHandle<Secret>(MODULE.package, "payments-secret");
    const webhook = defineWebhook(
        "payments",
        { template: "/{account}", signature: new StandardSignature(), secret },
        MODULE,
    );
    const trigger = new WebhookTrigger("paid", { webhook, input: pushed });

    expect([describeWebhook(webhook), describeTrigger(trigger)]).toEqual([
        {
            name: "payments",
            template: "/{account}",
            verification: "standard",
            secret: { packageId: MODULE.package.id, name: "payments-secret" },
        },
        {
            name: "paid",
            kind: "webhook",
            webhook: { packageId: MODULE.package.id, name: "payments" },
        },
    ]);
});

test("match a path's parameters, refusing paths outside the template", async () => {
    const path = new WebhookPath("/repositories/{repository}");
    const matched = (pathname: string) => {
        try {
            return path.match(pathname);
        } catch (error) {
            return Failure.of(error).message;
        }
    };

    expect([
        matched("/repositories/acme%20site"),
        matched("/repositories"),
        matched("/repositories/acme/site"),
        matched("/projects/acme"),
        matched("/repositories/%E0"),
    ]).toEqual([
        { repository: "acme site" },
        "webhook path does not match: /repositories",
        "webhook path does not match: /repositories/acme/site",
        "webhook path does not match: /projects/acme",
        "webhook path does not match: /repositories/%E0",
    ]);
});

/** Read a declaration's refusal: a path's schema issues, or the message, else accepted. */
function verdict(declare: () => unknown): string | string[] {
    try {
        declare();

        return "accepted";
    } catch (error) {
        // read a refused path by its schema issues, and the others by their message
        return error instanceof schema.Error
            ? error.issues.map((issue) => issue.code)
            : Failure.of(error).message;
    }
}

test("refuse malformed path templates and a repeated parameter", () => {
    expect([
        verdict(() => new WebhookPath("/")),
        verdict(() => new WebhookPath("")),
        verdict(() => new WebhookPath("/{repository}/")),
        verdict(() => new WebhookPath("/{Repository}")),
        verdict(() => new WebhookPath("/{repository}/{repository}")),
    ]).toEqual([
        "accepted",
        ["invalid_format"],
        ["invalid_format"],
        ["invalid_format"],
        "webhook path repeats a parameter: /{repository}/{repository}",
    ]);
});
