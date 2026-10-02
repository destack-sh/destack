import { aligned } from "@destack/schema";
import { expect, test } from "@destack/test";
import { MimeError, MimeMessage, type Message } from "./index.ts";

/** The line length folding aims for, as RFC 2047 sets. */
const LINE_LIMIT = 76;

/** A message every test varies. */
const MESSAGE: Message = {
    from: "Destack <notices@destack.app>",
    to: ["ada@example.com"],
    subject: "New sign-in",
    date: new Date("2026-09-29T10:00:00Z"),
    key: "security-notice-01",
    text: "A new device signed in.",
};

/** The Message-ID and boundary digest of notices@destack.app and the key security-notice-01. */
const DIGEST = "82598023bb81d5b125fb169a337de9fd";

/** The text part of the message every test varies. */
const TEXT_PART = [
    "Content-Type: text/plain; charset=utf-8",
    "Content-Transfer-Encoding: quoted-printable",
    "",
    "A new device signed in.",
    "",
];

test("compose a text message with its structural and extra headers", async () => {
    const message = await MimeMessage.compose({
        ...MESSAGE,
        to: ["Ada Lovelace <ada@example.com>", "charles@example.com"],
        cc: ['"Babbage, Charles" <babbage@example.com>'],
        replyTo: "support@destack.app",
        headers: { "X-Destack-Kind": "security" },
    });

    expect(message.messageId).toBe(`<${DIGEST}@destack.app>`);
    expect(message.content).toBe(
        [
            "From: Destack <notices@destack.app>",
            "To: Ada Lovelace <ada@example.com>, charles@example.com",
            'Cc: "Babbage, Charles" <babbage@example.com>',
            "Reply-To: support@destack.app",
            "Subject: New sign-in",
            "Date: Tue, 29 Sep 2026 10:00:00 +0000",
            `Message-ID: <${DIGEST}@destack.app>`,
            "MIME-Version: 1.0",
            "X-Destack-Kind: security",
            ...TEXT_PART,
        ].join("\r\n"),
    );
});

test("compose html after text as multipart/alternative", async () => {
    const message = await MimeMessage.compose({
        ...MESSAGE,
        html: "<p>A new device signed in.</p>",
    });

    expect(message.content).toBe(
        [
            "From: Destack <notices@destack.app>",
            "To: ada@example.com",
            "Subject: New sign-in",
            "Date: Tue, 29 Sep 2026 10:00:00 +0000",
            `Message-ID: <${DIGEST}@destack.app>`,
            "MIME-Version: 1.0",
            "Content-Type: multipart/alternative;",
            ` boundary="=_${DIGEST}"`,
            "",
            `--=_${DIGEST}`,
            "Content-Type: text/plain; charset=utf-8",
            "Content-Transfer-Encoding: quoted-printable",
            "",
            "A new device signed in.",
            `--=_${DIGEST}`,
            "Content-Type: text/html; charset=utf-8",
            "Content-Transfer-Encoding: quoted-printable",
            "",
            "<p>A new device signed in.</p>",
            `--=_${DIGEST}--`,
            "",
        ].join("\r\n"),
    );
});

test("encode non-ASCII subjects, display names and header values as RFC 2047 encoded-words", async () => {
    const message = await MimeMessage.compose({
        ...MESSAGE,
        from: "Zoë Destack <notices@destack.app>",
        subject: "Neue Anmeldung auf einem Gerät in Zürich, Österreich und Übersee",
        headers: { "X-Note": "=?looks encoded?=", "X-Lines": "one\r\nBcc: injected@example.com" },
    });

    expect(message.content).toBe(
        [
            "From: =?UTF-8?B?Wm/DqyBEZXN0YWNr?= <notices@destack.app>",
            "To: ada@example.com",
            "Subject: =?UTF-8?B?TmV1ZSBBbm1lbGR1bmcgYXVmIGVpbmVtIEdlcsOkdCBpbiBa?=",
            " =?UTF-8?B?w7xyaWNoLCDDlnN0ZXJyZWljaCB1bmQgw5xiZXJzZWU=?=",
            "Date: Tue, 29 Sep 2026 10:00:00 +0000",
            `Message-ID: <${DIGEST}@destack.app>`,
            "MIME-Version: 1.0",
            "X-Note: =?looks encoded?=",
            "X-Lines: =?UTF-8?B?b25lDQpCY2M6IGluamVjdGVkQGV4YW1wbGUuY29t?=",
            ...TEXT_PART,
        ].join("\r\n"),
    );
});

test("fold long ASCII subjects, display names and address lists at spaces", async () => {
    const message = await MimeMessage.compose({
        ...MESSAGE,
        from: '"Destack Security Notices, for every account and every device" <notices@destack.app>',
        to: [
            "Ada Lovelace <ada.lovelace@analytical-engine.example.com>",
            "Charles Babbage <charles.babbage@difference-engine.example.com>",
        ],
        subject:
            "A new device signed in to your account from a browser we have not seen before today",
    });

    expect(message.content).toBe(
        [
            'From: "Destack Security Notices, for every account and every device"',
            " <notices@destack.app>",
            "To: Ada Lovelace <ada.lovelace@analytical-engine.example.com>, Charles",
            " Babbage <charles.babbage@difference-engine.example.com>",
            "Subject: A new device signed in to your account from a browser we have not",
            " seen before today",
            "Date: Tue, 29 Sep 2026 10:00:00 +0000",
            `Message-ID: <${DIGEST}@destack.app>`,
            "MIME-Version: 1.0",
            ...TEXT_PART,
        ].join("\r\n"),
    );
});

test("keep long structured headers valid, folding them at spaces", async () => {
    const unsubscribe = `https://destack.app/unsubscribe/${"a1b2c3d4".repeat(12)}`;
    const message = await MimeMessage.compose({
        ...MESSAGE,
        headers: {
            "List-Unsubscribe": `<${unsubscribe}>, <mailto:unsubscribe@destack.app?subject=unsubscribe>`,
            "List-Unsubscribe-Post": "List-Unsubscribe=One-Click",
            "In-Reply-To": "<0123456789abcdef0123456789abcdef@destack.app>",
            References:
                "<0123456789abcdef0123456789abcdef@destack.app> <fedcba9876543210fedcba9876543210@destack.app>",
        },
    });

    expect(message.content).toBe(
        [
            "From: Destack <notices@destack.app>",
            "To: ada@example.com",
            "Subject: New sign-in",
            "Date: Tue, 29 Sep 2026 10:00:00 +0000",
            `Message-ID: <${DIGEST}@destack.app>`,
            "MIME-Version: 1.0",
            "List-Unsubscribe:",
            ` <${unsubscribe}>,`,
            " <mailto:unsubscribe@destack.app?subject=unsubscribe>",
            "List-Unsubscribe-Post: List-Unsubscribe=One-Click",
            "In-Reply-To: <0123456789abcdef0123456789abcdef@destack.app>",
            "References: <0123456789abcdef0123456789abcdef@destack.app>",
            " <fedcba9876543210fedcba9876543210@destack.app>",
            ...TEXT_PART,
        ].join("\r\n"),
    );
});

test("encode bodies as quoted-printable UTF-8 with CRLF line breaks and soft breaks", async () => {
    const message = await MimeMessage.compose({
        ...MESSAGE,
        text: `Price = 5 €\nTrailing space \r\nTrailing tab\t\r${"x".repeat(80)}\n.leading dot`,
    });

    expect(message.content).toBe(
        [
            "From: Destack <notices@destack.app>",
            "To: ada@example.com",
            "Subject: New sign-in",
            "Date: Tue, 29 Sep 2026 10:00:00 +0000",
            `Message-ID: <${DIGEST}@destack.app>`,
            "MIME-Version: 1.0",
            "Content-Type: text/plain; charset=utf-8",
            "Content-Transfer-Encoding: quoted-printable",
            "",
            "Price =3D 5 =E2=82=AC",
            "Trailing space=20",
            "Trailing tab=09",
            `${"x".repeat(75)}=`,
            "xxxxx",
            ".leading dot",
            "",
        ].join("\r\n"),
    );
});

test("derive the Message-ID from the sender's address and the key alone", async () => {
    const first = await MimeMessage.compose(MESSAGE);
    const resent = await MimeMessage.compose({
        ...MESSAGE,
        from: "Destack Notices <notices@destack.app>",
        date: new Date("2026-09-30T10:00:00Z"),
        text: "A new device signed in again.",
    });
    const other = await MimeMessage.compose({ ...MESSAGE, key: "security-notice-02" });
    const sender = await MimeMessage.compose({ ...MESSAGE, from: "alerts@destack.app" });
    const relayed = await MimeMessage.compose({ ...MESSAGE, from: "notices@mail.destack.app" });

    // keep the Message-ID across resends of one key, and change it with the key or the address
    expect([
        first.messageId,
        resent.messageId,
        other.messageId,
        sender.messageId,
        relayed.messageId,
    ]).toEqual([
        `<${DIGEST}@destack.app>`,
        `<${DIGEST}@destack.app>`,
        "<0226d785f39f5d91d080a62a32401cf5@destack.app>",
        "<f4578e06603c0641d26aefac781ab9f5@destack.app>",
        "<e1c4cb0bfa85996a1e1500481fbdebfa@mail.destack.app>",
    ]);
});

test("refuse structural headers, malformed headers, invalid addresses, keys and dates", async () => {
    const longName = `X-${"a".repeat(997)}`;
    const failures = await Promise.all(
        [
            { ...MESSAGE, headers: { Subject: "Other" } },
            { ...MESSAGE, headers: { bcc: "eve@example.com" } },
            { ...MESSAGE, headers: { "Content-Transfer-Encoding": "8bit" } },
            { ...MESSAGE, headers: { "X Kind": "security" } },
            { ...MESSAGE, headers: { "X-Kind:": "security" } },
            { ...MESSAGE, headers: { [longName]: "security" } },
            { ...MESSAGE, subject: "x".repeat(998) },
            { ...MESSAGE, to: ["ada@example.com>\r\nBcc: eve@example.com"] },
            { ...MESSAGE, cc: ["Ada <ada at example.com>"] },
            { ...MESSAGE, to: [`${"a".repeat(65)}@example.com`] },
            { ...MESSAGE, to: [`ada@${"a".repeat(64)}.example.com`] },
            { ...MESSAGE, from: "notices" },
            { ...MESSAGE, key: "" },
            { ...MESSAGE, date: new Date("not a date") },
        ].map((message) => failureOf(MimeMessage.compose(message))),
    );

    expect(failures).toEqual([
        { code: "INVALID_HEADER", message: "header Subject is written by the composer" },
        { code: "INVALID_HEADER", message: "header bcc is written by the composer" },
        {
            code: "INVALID_HEADER",
            message: "header Content-Transfer-Encoding is written by the composer",
        },
        {
            code: "INVALID_HEADER",
            message: 'header name "X Kind" contains a character outside printable ASCII or a colon',
        },
        {
            code: "INVALID_HEADER",
            message:
                'header name "X-Kind:" contains a character outside printable ASCII or a colon',
        },
        { code: "INVALID_HEADER", message: `header ${longName} has a line over 998 characters` },
        { code: "INVALID_HEADER", message: "header Subject has a line over 998 characters" },
        {
            code: "INVALID_ADDRESS",
            message:
                'address "ada@example.com>\\r\\nBcc: eve@example.com" is not a dot-atom addr-spec',
        },
        {
            code: "INVALID_ADDRESS",
            message: 'address "ada at example.com" is not a dot-atom addr-spec',
        },
        {
            code: "INVALID_ADDRESS",
            message: `address "${"a".repeat(65)}@example.com" is not a dot-atom addr-spec`,
        },
        {
            code: "INVALID_ADDRESS",
            message: `address "ada@${"a".repeat(64)}.example.com" is not a dot-atom addr-spec`,
        },
        { code: "INVALID_ADDRESS", message: 'address "notices" is not a dot-atom addr-spec' },
        { code: "INVALID_KEY", message: "message key is empty" },
        { code: "INVALID_DATE", message: "message date is invalid" },
    ]);
});

test("roundtrip random subjects and bodies within the line limits", async () => {
    // compose messages from seeded random text over ASCII, controls, Latin-1, CJK and emoji
    const random = seeded(7);
    const alphabet = [...Array.from("abc XYZ.=?\t\r\n-_:;<>"), "é", "€", "日本", "😀", "\u0000"];
    for (let round = 0; round < 200; round += 1) {
        const pick = () =>
            Array.from({ length: Math.floor(random() * 120) }, () =>
                aligned(alphabet, Math.floor(random() * alphabet.length)),
            ).join("");
        const subject = pick().replaceAll(/[\r\n]/gu, "");
        const text = pick();
        const message = await MimeMessage.compose({ ...MESSAGE, subject, text });

        // keep every line within the limit, and decode the subject and body back
        const lines = message.content.split("\r\n");
        expect(lines.filter((line) => line.length > LINE_LIMIT)).toEqual([]);
        expect(decodeSubject(message.content)).toBe(subject);
        expect(decodeQuotedPrintable(bodyOf(message.content))).toBe(
            text.replaceAll(/\r\n|\r|\n/gu, "\r\n"),
        );
    }
});

/** Read a single-part message's body. */
function bodyOf(content: string): string {
    return content.slice(content.indexOf("\r\n\r\n") + 4);
}

/** Unfold the Subject field and decode its encoded-words. */
function decodeSubject(content: string): string {
    // unfold the header section and take the Subject value
    const headers = content.slice(0, content.indexOf("\r\n\r\n"));
    const unfolded = headers.replaceAll("\r\n ", " ");
    const field = /^Subject:(.*)$/mu.exec(unfolded);
    if (field === null) {
        throw new TypeError("message has no subject");
    }
    const value = aligned(field, 1).slice(1);
    if (!value.startsWith("=?UTF-8?B?")) {
        return value;
    }

    // join adjacent encoded-words, dropping the whitespace between them
    const words = value.split(" ").map((word) => {
        const encoded = /^=\?UTF-8\?B\?(.*)\?=$/u.exec(word);
        if (encoded === null) {
            throw new TypeError(`subject word ${word} is no encoded-word`);
        }

        return aligned(encoded, 1);
    });

    return words.map((word) => new TextDecoder().decode(Uint8Array.fromBase64(word))).join("");
}

/** Decode a quoted-printable body into text. */
function decodeQuotedPrintable(body: string): string {
    // drop the final line break and the soft breaks, then decode the escapes as UTF-8
    const joined = body.slice(0, -2).replaceAll("=\r\n", "");
    const bytes = joined.replaceAll(
        /=([0-9A-F]{2})|([\s\S])/gu,
        (_, hex: string, literal: string) =>
            hex === undefined ? literal : String.fromCharCode(Number.parseInt(hex, 16)),
    );

    return new TextDecoder().decode(Uint8Array.from(bytes, (character) => character.charCodeAt(0)));
}

/** Seed a pseudo-random generator: mulberry32. */
function seeded(seed: number): () => number {
    let state = seed;

    return () => {
        state = (state + 0x6d2b79f5) | 0;
        let mixed = Math.imul(state ^ (state >>> 15), 1 | state);
        mixed = (mixed + Math.imul(mixed ^ (mixed >>> 7), 61 | mixed)) ^ mixed;

        return ((mixed ^ (mixed >>> 14)) >>> 0) / 4_294_967_296;
    };
}

/** Read the code and message a composition fails with. */
async function failureOf(
    composed: Promise<MimeMessage>,
): Promise<{ code: string; message: string }> {
    try {
        await composed;
    } catch (error) {
        if (error instanceof MimeError) {
            return { code: error.code, message: error.message };
        }
        throw error;
    }
    throw new Error("composition succeeded");
}
