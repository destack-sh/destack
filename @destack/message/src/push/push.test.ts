import { aligned, present, schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { decrypt, encode, importBrowser, subscribeBrowser, UNQUERIED } from "../test/index.ts";
import { PushEncryption } from "./encryption.ts";
import { pushProvider } from "./push.ts";
import { Vapid } from "./vapid.ts";

/** The time every push is signed at. */
const NOW = 1_790_000_000_000;

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
    const header = await vapid.authorization("https://fcm.googleapis.com/fcm/send/abc", NOW);

    // verify the signature with the key browsers subscribe with, and read the claims
    expect(await readVapid(header)).toEqual({
        isValid: true,
        header: { typ: "JWT", alg: "ES256" },
        claims: {
            aud: "https://fcm.googleapis.com",
            exp: 1_790_043_200,
            sub: "mailto:push@destack.app",
        },
        key: await vapid.publicKey(),
    });

    // refuse a contact push services cannot find
    expect(() => new Vapid(keys, "push@destack.app")).toThrow(
        "a push sender's contact is a mailto: or https: URL",
    );
});

/** Push to one browser through a push service answering each post with the next response, signing with one scope's keys. */
async function pushTo(responses: Response[]) {
    // sign with one key pair, keeping the scopes it is derived for
    const keys = await crypto.subtle.generateKey({ name: "ECDSA", namedCurve: "P-256" }, true, [
        "sign",
        "verify",
    ]);
    const service = answerInTurn(responses);
    const keyed: string[] = [];
    const vapid = new Vapid(keys, "https://destack.app");
    const deriveVapid = async (scope: string) => {
        keyed.push(scope);

        return vapid;
    };
    const provider = pushProvider(deriveVapid, service.fetch);

    // address one browser
    const browser = await subscribeBrowser();
    const message = {
        id: schema.identifier("message").parse("message-019f5530-8000-7000-8000-0000000000aa"),
        scope: "space-019f5530-8000-7000-8000-0000000000ab",
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
    };
    const send = () => provider.send(message, { ...UNQUERIED, clock: () => NOW });

    return { provider, vapid, keyed, browser, message, service, send };
}

test("read each status a push service answers as an outcome", async () => {
    const { send } = await pushTo([
        new Response(null, { status: 201 }),
        new Response(null, { status: 410, statusText: "Gone" }),
        new Response(null, {
            status: 429,
            statusText: "Too Many Requests",
            headers: { "Retry-After": "30" },
        }),
        new Response(null, { status: 503, statusText: "Service Unavailable" }),
        new Response(null, { status: 413, statusText: "Content Too Large" }),
    ]);
    const outcomes = [];
    for (let index = 0; index < 5; index++) {
        outcomes.push(await send());
    }

    expect(outcomes).toEqual([
        { kind: "sent" },
        { kind: "failed", error: { code: "gone", message: "Gone" } },
        { kind: "retry", after: 30_000, error: { code: "429", message: "Too Many Requests" } },
        { kind: "retry", error: { code: "503", message: "Service Unavailable" } },
        { kind: "failed", error: { code: "413", message: "Content Too Large" } },
    ]);
});

test("answer a scope's application server key, and post a push signed by it and encrypted with RFC 8030's headers", async () => {
    const { provider, vapid, keyed, browser, message, service, send } = await pushTo([
        new Response(null, { status: 201 }),
    ]);
    await send();

    // sign with the scope's key, which its application server key is
    expect(await provider.applicationServerKey(message.scope)).toBe(await vapid.publicKey());
    expect(new Set(keyed)).toEqual(new Set([message.scope]));

    // check the coding, lifetime, topic and urgency of the post, signed by the sender and decrypted by the browser
    const first = aligned(service.posted, 0);
    const { authorization, ...headers } = first.headers;
    expect({
        url: first.url,
        authorization: await readVapid(present(authorization, "the post's authorization")),
        headers,
        payload: await decrypt(browser, first.body),
    }).toEqual({
        url: "https://updates.push.services.mozilla.com/wpush/v2/abc",
        authorization: {
            isValid: true,
            header: { typ: "JWT", alg: "ES256" },
            claims: {
                aud: "https://updates.push.services.mozilla.com",
                exp: 1_790_043_200,
                sub: "https://destack.app",
            },
            key: await vapid.publicKey(),
        },
        headers: {
            "content-encoding": "aes128gcm",
            "content-type": "application/octet-stream",
            ttl: "86400",
            topic: "019f55308000700080000000000000aa",
            urgency: "high",
        },
        payload: '{"title":"Alice mentioned you"}',
    });
});

/** A push service answering each post with the next response, keeping the posts. */
function answerInTurn(responses: Response[]) {
    const posted: {
        url: string;
        headers: Record<string, string>;
        body: Uint8Array<ArrayBuffer>;
    }[] = [];
    const fetch = async (request: Request) => {
        // record the post
        const headers = Object.fromEntries(request.headers.entries());
        posted.push({ url: request.url, headers, body: await request.bytes() });

        // answer with the next response
        const response = responses.shift();
        if (response === undefined) {
            throw new TypeError("push service has no more responses");
        }

        return response;
    };

    return { fetch, posted };
}

/** Read a VAPID Authorization header: whether its key verifies its token, the token's header and claims, and the key. */
async function readVapid(authorization: string) {
    // split the token and the key, and the token into its header, claims and signature
    const match = /^vapid t=([^,]+), k=(.+)$/u.exec(authorization);
    if (match === null) {
        throw new TypeError(`authorization ${authorization} is no vapid header`);
    }
    const key = aligned(match, 2);
    const segments = aligned(match, 1).split(".");
    const [header, claims, signature] = [
        aligned(segments, 0),
        aligned(segments, 1),
        aligned(segments, 2),
    ];

    // verify the signature with the key
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
        new TextEncoder().encode(`${header}.${claims}`),
    );

    return { isValid, header: readSegment(header), claims: readSegment(claims), key };
}

/** Read a JWT segment's JSON. */
function readSegment(segment: string): unknown {
    return JSON.parse(
        new TextDecoder().decode(Uint8Array.fromBase64(segment, { alphabet: "base64url" })),
    );
}
