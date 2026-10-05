import { aligned, schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { decrypt, encode, importBrowser, subscribeBrowser } from "../test/index.ts";
import { PushEncryption } from "./encryption.ts";
import { pushProvider } from "./push.ts";
import { Vapid } from "./vapid.ts";

/** RFC 8291's example: its keys, salt, message and the body it encrypts to. */
const EXAMPLE = {
    message: "When I grow up, I want to be a watermelon",
    sender: {
        publicKey:
            "BP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27mlmlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A8",
        privateKey: "yfWPiYE-n46HLnH0KqZOF1fJJU3MYrct3AELtAQ-oRw",
    },
    receiver: {
        keys: {
            p256dh: "BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4",
            auth: "BTBZMqHH6r4Tts7J_aSIgg",
        },
        privateKey: "q1dXpw3UpT5VOmu_cf_v6ih07Aems3njxI-JWgLcM94",
    },
    salt: "DGv6ra1nlYgDCS1FRnbzlw",
    body: "DGv6ra1nlYgDCS1FRnbzlwAAEABBBP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27mlmlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A_yl95bQpu6cVPTpK4Mqgkf1CXztLVBSt2Ks3oZwbuwXPXLWyouBWLVWGNWQexSgSxsj_Qulcy4a-fN",
};

test("encrypt RFC 8291's example message to its exact body, which the browser decrypts", async () => {
    // encrypt with the example's sender keys and salt
    const sender = await importBrowser(
        { p256dh: EXAMPLE.sender.publicKey, auth: EXAMPLE.receiver.keys.auth },
        EXAMPLE.sender.privateKey,
    );
    const body = await PushEncryption.encrypt(
        EXAMPLE.receiver.keys,
        new TextEncoder().encode(EXAMPLE.message),
        {
            sender: sender.pair,
            salt: new Uint8Array(Uint8Array.fromBase64(EXAMPLE.salt, { alphabet: "base64url" })),
        },
    );

    // match the RFC's body byte for byte, and decrypt it as its browser
    const browser = await importBrowser(EXAMPLE.receiver.keys, EXAMPLE.receiver.privateKey);
    expect([encode(body), await decrypt(browser, body)]).toEqual([EXAMPLE.body, EXAMPLE.message]);
});

test("encrypt each message under a fresh key and salt, which only its browser decrypts", async () => {
    const browser = await subscribeBrowser();
    const stranger = await subscribeBrowser();
    const message = new TextEncoder().encode(JSON.stringify({ title: "Alice mentioned you" }));

    // encrypt the same message twice to different bodies: the header, the message with its delimiter, and the tag
    const first = await PushEncryption.encrypt(browser.keys, message);
    const second = await PushEncryption.encrypt(browser.keys, message);
    expect([encode(first) === encode(second), first.length, second.length]).toEqual([
        false,
        86 + message.length + 1 + 16,
        86 + message.length + 1 + 16,
    ]);

    // decrypt with the browser's key, and refuse another browser's
    expect(await decrypt(browser, first)).toBe('{"title":"Alice mentioned you"}');
    await expect(decrypt(stranger, first)).rejects.toThrow(DOMException);
});

test("sign VAPID tokens for the push service's origin, verifiable with the application server key", async () => {
    // sign a token for a push resource at a fixed time
    const keys = await crypto.subtle.generateKey({ name: "ECDSA", namedCurve: "P-256" }, true, [
        "sign",
        "verify",
    ]);
    const vapid = new Vapid(keys, "mailto:push@destack.app");
    const header = await vapid.authorization(
        "https://fcm.googleapis.com/fcm/send/abc",
        1_790_000_000_000,
    );
    const match = /^vapid t=([^,]+), k=(.+)$/u.exec(header);
    if (match === null) {
        throw new TypeError(`authorization ${header} is no vapid header`);
    }
    const key = aligned(match, 2);
    const parts = aligned(match, 1).split(".");
    const [head, claims, signature] = [aligned(parts, 0), aligned(parts, 1), aligned(parts, 2)];

    // verify the signature with the key browsers subscribe with, and read the claims
    const verifier = await crypto.subtle.importKey(
        "raw",
        Uint8Array.fromBase64(key, { alphabet: "base64url" }),
        { name: "ECDSA", namedCurve: "P-256" },
        false,
        ["verify"],
    );
    const isValid = await crypto.subtle.verify(
        { name: "ECDSA", hash: "SHA-256" },
        verifier,
        Uint8Array.fromBase64(signature, { alphabet: "base64url" }),
        new TextEncoder().encode(`${head}.${claims}`),
    );
    expect([isValid, readPart(head), readPart(claims), key]).toEqual([
        true,
        { typ: "JWT", alg: "ES256" },
        { aud: "https://fcm.googleapis.com", exp: 1_790_043_200, sub: "mailto:push@destack.app" },
        await vapid.publicKey(),
    ]);

    // refuse a contact push services cannot find
    expect(() => new Vapid(keys, "push@destack.app")).toThrow(
        "a push sender's contact is a mailto: or https: URL",
    );
});

test("post encrypted pushes with RFC 8030's headers, which the browser decrypts, and read each status as an outcome", async () => {
    // answer each post with the next scripted response
    const keys = await crypto.subtle.generateKey({ name: "ECDSA", namedCurve: "P-256" }, true, [
        "sign",
        "verify",
    ]);
    const responses = [
        new Response(null, { status: 201 }),
        new Response("gone", { status: 410 }),
        new Response("slow down", { status: 429, headers: { "Retry-After": "30" } }),
        new Response("unavailable", { status: 503 }),
        new Response("payload too large", { status: 413 }),
    ];
    const posted: {
        url: string;
        headers: Record<string, string>;
        body: Uint8Array<ArrayBuffer>;
    }[] = [];
    const provider = pushProvider(new Vapid(keys, "https://destack.app"), async (url, options) => {
        // record the post
        const headers = Object.fromEntries(new Headers(options.headers).entries());
        if (!(options.body instanceof Uint8Array)) {
            throw new TypeError("push posts bytes");
        }
        posted.push({ url, headers, body: new Uint8Array(options.body) });

        // answer with the next response
        const response = responses.shift();
        if (response === undefined) {
            throw new TypeError("push service has no more responses");
        }

        return response;
    });

    // push five messages to one browser
    const browser = await subscribeBrowser();
    const message = {
        id: schema.identifier("message").parse("message-019f5530-8000-7000-8000-0000000000aa"),
        createdAt: 0,
        to: {
            channel: "push" as const,
            url: "https://updates.push.services.mozilla.com/wpush/v2/abc",
            keys: browser.keys,
        },
        content: {
            channel: "push" as const,
            data: { title: "Alice mentioned you" },
            urgency: "high" as const,
            ttl: 86_400,
            topic: "019f55308000700080000000000000aa",
        },
        secret: null,
    };
    const outcomes = [];
    for (let index = 0; index < 5; index++) {
        outcomes.push(await provider.send(message));
    }

    // read success, a forgotten endpoint, throttling, an outage and a refusal
    expect(outcomes).toEqual([
        { outcome: "sent" },
        { outcome: "failed", error: { code: "gone", message: "gone" } },
        { outcome: "retry", after: 30_000, error: { code: "429", message: "slow down" } },
        { outcome: "retry", error: { code: "503", message: "unavailable" } },
        { outcome: "failed", error: { code: "413", message: "payload too large" } },
    ]);

    // check the coding, lifetime, topic and urgency of each post, signed by the sender and decrypted by the browser
    const first = aligned(posted, 0);
    const authorization = first.headers["authorization"];
    if (authorization === undefined) {
        throw new TypeError("push post has no authorization");
    }
    expect({
        url: first.url,
        headers: { ...first.headers, authorization: authorization.slice(0, 8) },
        payload: await decrypt(browser, first.body),
    }).toEqual({
        url: "https://updates.push.services.mozilla.com/wpush/v2/abc",
        headers: {
            authorization: "vapid t=",
            "content-encoding": "aes128gcm",
            "content-type": "application/octet-stream",
            ttl: "86400",
            topic: "019f55308000700080000000000000aa",
            urgency: "high",
        },
        payload: '{"title":"Alice mentioned you"}',
    });
});

/** Read a JWT part's JSON. */
function readPart(part: string): unknown {
    return JSON.parse(
        new TextDecoder().decode(Uint8Array.fromBase64(part, { alphabet: "base64url" })),
    );
}
