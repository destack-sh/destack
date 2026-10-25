import { PackageId } from "@destack/package";
import { nth, present, schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { decrypt, encode, importBrowser, subscribeBrowser, TestMessage } from "../test/index.ts";
import { PushEncryption } from "./encryption.ts";
import { PUSH_DATA_BYTES, PushContent } from "./message.ts";
import { pushProvider } from "./push.ts";
import { Vapid } from "./vapid.ts";
import { KeyPair } from "@destack/identity";
import { testSimulation } from "@destack/object/test";

/** The time every push is signed at. */
const NOW = 1_790_000_000_000;

/** The world every push is sent in, at the signing time. */
const world = testSimulation(NOW).environment("push");

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
        world.crypto,
    );
    const body = await PushEncryption.encrypt(
        EXAMPLE.receiver.keys,
        new TextEncoder().encode(EXAMPLE.message),
        world,
        {
            sender: sender.pair,
            salt: new Uint8Array(Uint8Array.fromBase64(EXAMPLE.salt, { alphabet: "base64url" })),
        },
    );

    // match the RFC's body byte for byte, and decrypt it as its browser
    const browser = await importBrowser(
        EXAMPLE.receiver.keys,
        EXAMPLE.receiver.privateKey,
        world.crypto,
    );
    expect([encode(body), await decrypt(browser, body)]).toEqual([EXAMPLE.body, EXAMPLE.message]);
});

test("encrypt each message under a fresh key and salt, which only its browser decrypts", async () => {
    const browser = await subscribeBrowser(world);
    const stranger = await subscribeBrowser(world);
    const message = new TextEncoder().encode(JSON.stringify({ title: "Alice mentioned you" }));

    // encrypt the same message twice to different bodies: the header, the message with its delimiter, and the tag
    const first = await PushEncryption.encrypt(browser.keys, message, world);
    const second = await PushEncryption.encrypt(browser.keys, message, world);
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
    const { pair: keys } = await KeyPair.generate(world.crypto);
    const vapid = new Vapid(keys, "mailto:push@destack.app", world.crypto);
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
        key: vapid.publicKey(),
    });

    // refuse a contact push services cannot find
    expect(() => new Vapid(keys, "push@destack.app", world.crypto)).toThrow(
        new TypeError("push sender contact push@destack.app is not a mailto: or https: URL"),
    );
});

/** Push to a browser through a push service answering with the next response, signed per scope. */
async function pushTo(responses: Response[]) {
    // sign with one key pair
    const { pair: keys } = await KeyPair.generate(world.crypto);
    const service = answerInTurn(responses);
    const vapid = new Vapid(keys, "https://destack.app", world.crypto);
    const provider = pushProvider(async () => vapid, service.fetch);

    // address one browser
    const browser = await subscribeBrowser(world);
    const message = {
        id: schema.identifier("message").parse("message-019f5530-8000-7000-8000-0000000000aa"),
        scope: "space-019f5530-8000-7000-8000-0000000000ab",
        createdAt: 0,
        to: {
            channel: "push" as const,
            client: schema
                .identifier("client")
                .parse("client-019f5530-8000-7000-8000-0000000000a5"),
            user: schema.identifier("user").parse("user-019f5530-8000-7000-8000-0000000000a6"),
            url: "https://updates.push.services.mozilla.com/wpush/v2/abc",
            keys: browser.keys,
        },
        source: {
            packageId: PackageId.parse("package-019f5530-8000-7000-8000-0000000000ac"),
            type: "notification",
            scope: "space-019f5530-8000-7000-8000-0000000000ab",
            id: "notification-019f5530-8000-7000-8000-0000000000ad",
        },
        content: {
            channel: "push" as const,
            data: { title: "Alice mentioned you" },
            urgency: "high" as const,
            ttl: 86_400,
            topic: "019f55308000700080000000000000aa",
        },
    };
    const send = () => provider.send(message, TestMessage.store(world));

    return { provider, vapid, browser, message, service, send };
}

test("answer the application server key, and post a push signed by it and encrypted with RFC 8030's headers", async () => {
    const { provider, vapid, browser, service, send } = await pushTo([
        new Response(null, { status: 201 }),
    ]);
    await send();

    // sign with the installation's key, its application server key
    expect(await provider.applicationServerKey()).toBe(vapid.publicKey());

    // check the coding, lifetime, topic and urgency of the post, signed by the sender and decrypted by the browser
    const first = nth(service.posted, 0);
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
            key: vapid.publicKey(),
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

/** A push service answering each post with the next response, recording the posts. */
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

/** Read a VAPID Authorization header: whether its key verifies its token, the token and key. */
async function readVapid(authorization: string) {
    // split the token and the key, and the token into its header, claims and signature
    const match = /^vapid t=([^,]+), k=(.+)$/u.exec(authorization);
    if (match === null) {
        throw new TypeError(`authorization ${authorization} is not a vapid header`);
    }
    const key = nth(match, 2);
    const segments = nth(match, 1).split(".");
    const [header, claims, signature] = [nth(segments, 0), nth(segments, 1), nth(segments, 2)];

    // verify the signature with the key
    const verifier = await world.crypto.importKey(
        "raw",
        Uint8Array.fromBase64(key, { alphabet: "base64url" }),
        { name: "ECDSA", namedCurve: "P-256" },
        false,
        ["verify"],
    );
    const isValid = await world.crypto.verify(
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

test("refuse a push payload past one record, at sealing and at encryption", async () => {
    // fill a payload one byte past the most a push carries
    const environment = testSimulation().environment("push");
    const data = "a".repeat(PUSH_DATA_BYTES - 1);
    const payload = new TextEncoder().encode(JSON.stringify(data));
    const content = { channel: "push" as const, data, urgency: "normal" as const, ttl: 60 };

    // refuse it as content, and refuse encrypting its bytes
    const encrypted = await PushEncryption.encrypt(
        EXAMPLE.receiver.keys,
        payload,
        environment,
    ).catch((error: unknown) => error);
    expect([PushContent.safeParse(content).success, payload.length, encrypted]).toEqual([
        false,
        PUSH_DATA_BYTES + 1,
        new RangeError(
            `a push carries at most ${PUSH_DATA_BYTES} bytes, not ${PUSH_DATA_BYTES + 1}`,
        ),
    ]);
});
