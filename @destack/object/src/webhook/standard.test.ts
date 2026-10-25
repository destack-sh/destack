import { testSimulation } from "../test/index.ts";
import { expect, refusal, test } from "@destack/test";
import { HmacKey } from "./hmac.ts";
import { StandardWebhook } from "./standard.ts";

/** The Standard Webhooks reference example of "sign function works". */
const EXAMPLE = {
    secret: "whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw",
    message: {
        id: "msg_p5jXN8AQM9LWM0D4loKWxJek",
        timestamp: 1614265330,
        body: '{"test": 2432232314}',
    },
    signature: "v1,g0hM9SsE+OTPJTGt/tmIKtSyZlE3uFJELVlNIOLJ1OE=",
};

/** The example's sending time in epoch milliseconds. */
const SENT_AT = EXAMPLE.message.timestamp * 1000;

/** Five minutes, the tolerance the reference libraries allow. */
const TOLERANCE = 5 * 60 * 1000;

/** Post a body with headers. */
function post(headers: Headers, body: string): Request {
    return new Request("https://hooks.test/webhooks", { method: "POST", headers, body });
}

test("sign the Standard Webhooks reference example exactly", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const webhook = await StandardWebhook.of(EXAMPLE.secret, world.crypto);
    expect([...(await webhook.sign(EXAMPLE.message)).entries()]).toEqual([
        ["webhook-id", EXAMPLE.message.id],
        ["webhook-signature", EXAMPLE.signature],
        ["webhook-timestamp", "1614265330"],
    ]);
});

test("verify a signed message within the tolerance, among other signatures", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // sign the example, then add an unknown version and a foreign signature before it
    const webhook = await StandardWebhook.of(EXAMPLE.secret, world.crypto);
    const headers = await webhook.sign(EXAMPLE.message);
    headers.set(
        "webhook-signature",
        `v2,abc v1,bm90IGEgc2lnbmF0dXJl ${headers.get("webhook-signature")}`,
    );
    const verify = (now: number) => webhook.verify(post(headers, EXAMPLE.message.body), now);

    // answer the message at both edges of the tolerance
    expect([await verify(SENT_AT - TOLERANCE), await verify(SENT_AT + TOLERANCE)]).toEqual([
        EXAMPLE.message,
        EXAMPLE.message,
    ]);
});

test("refuse changed bodies, times outside the tolerance or not in seconds, missing headers, unknown versions and other secrets", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // sign the example, and sign it again with another secret
    const webhook = await StandardWebhook.of(EXAMPLE.secret, world.crypto);
    const headers = await webhook.sign(EXAMPLE.message);
    const other = await StandardWebhook.of("whsec_c2Vjb25kIHNlY3JldA==", world.crypto);
    const forged = await other.sign(EXAMPLE.message);
    const { body } = EXAMPLE.message;
    const missing = new Headers(headers);
    missing.delete("webhook-timestamp");
    const exponent = new Headers(headers);
    exponent.set("webhook-timestamp", "1.614265330e9");
    const unknown = new Headers(headers);
    unknown.set("webhook-signature", "v2,abc");

    // refuse each
    const timestamp = "webhook timestamp is outside the tolerance";
    expect([
        await refusal(webhook.verify(post(headers, `${body} `), SENT_AT)),
        await refusal(webhook.verify(post(headers, body), SENT_AT + TOLERANCE + 1000)),
        await refusal(webhook.verify(post(headers, body), SENT_AT - TOLERANCE - 1000)),
        await refusal(webhook.verify(post(missing, body), SENT_AT)),
        await refusal(webhook.verify(post(exponent, body), SENT_AT)),
        await refusal(webhook.verify(post(unknown, body), SENT_AT)),
        await refusal(webhook.verify(post(forged, body), SENT_AT)),
    ]).toEqual([
        ["UNAUTHORIZED", "SIGNATURE_MISMATCH", "webhook signature does not match"],
        ["UNAUTHORIZED", "TIMESTAMP_OUTSIDE_TOLERANCE", timestamp],
        ["UNAUTHORIZED", "TIMESTAMP_OUTSIDE_TOLERANCE", timestamp],
        ["UNAUTHORIZED", "SIGNATURE_MISSING", "missing standard webhook headers"],
        ["UNAUTHORIZED", "TIMESTAMP_OUTSIDE_TOLERANCE", timestamp],
        ["UNAUTHORIZED", "SIGNATURE_MISMATCH", "webhook signature does not match"],
        ["UNAUTHORIZED", "SIGNATURE_MISMATCH", "webhook signature does not match"],
    ]);
});

test("verify the body's exact bytes, a byte order mark and invalid UTF-8 included", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // sign bytes that decoding as UTF-8 would change
    const webhook = await StandardWebhook.of(EXAMPLE.secret, world.crypto);
    const bytes = new Uint8Array([0xef, 0xbb, 0xbf, 0x7b, 0xff, 0x7d]);
    const prefix = new TextEncoder().encode(`${EXAMPLE.message.id}.1614265330.`);
    const signed = new Uint8Array([...prefix, ...bytes]);
    const key = await HmacKey.of(
        Uint8Array.fromBase64(EXAMPLE.secret.slice("whsec_".length)),
        world.crypto,
    );
    const digest = await key.sign(signed);
    const headers = new Headers({
        "webhook-id": EXAMPLE.message.id,
        "webhook-timestamp": "1614265330",
        "webhook-signature": `v1,${digest.toBase64()}`,
    });
    const request = new Request("https://hooks.test/webhooks", {
        method: "POST",
        headers,
        body: bytes,
    });
    expect(await webhook.verify(request, SENT_AT)).toEqual({
        id: EXAMPLE.message.id,
        timestamp: EXAMPLE.message.timestamp,
        body: new TextDecoder().decode(bytes),
    });
});

test("refuse a secret without the whsec_ prefix or with malformed base64", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    expect([
        await refusal(StandardWebhook.of("MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw", world.crypto)),
        await refusal(StandardWebhook.of("whsec_!!", world.crypto)),
    ]).toEqual([
        ["PRECONDITION_FAILED", "SECRET_INVALID", "standard webhook secret must start with whsec_"],
        ["PRECONDITION_FAILED", "SECRET_INVALID", "standard webhook secret must be base64"],
    ]);
});
