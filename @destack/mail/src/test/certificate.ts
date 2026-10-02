/** The DER tags a certificate uses. */
const TAG = {
    integer: 0x02,
    bitString: 0x03,
    octetString: 0x04,
    identifier: 0x06,
    utf8String: 0x0c,
    sequence: 0x30,
    set: 0x31,
    utcTime: 0x17,
    version: 0xa0,
    extensions: 0xa3,
    ipAddress: 0x87,
} as const;

/** The DER contents of the object identifiers a certificate uses. */
const OBJECT = {
    /** The ecdsa-with-SHA256 algorithm, 1.2.840.10045.4.3.2. */
    ecdsaWithSha256: [0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x02],
    /** The commonName attribute, 2.5.4.3. */
    commonName: [0x55, 0x04, 0x03],
    /** The subjectAltName extension, 2.5.29.17. */
    subjectAltName: [0x55, 0x1d, 0x11],
} as const;

/** The X.509 version field of a v3 certificate, which DER writes as 2. */
const VERSION_3 = 2;

/** How long a test certificate stays valid either side of its creation: one hour. */
const VALIDITY_MILLISECONDS = 3_600_000;

/** The longest DER contents a two-byte length encodes. */
const MAX_DER_LENGTH = 0xffff;

/** The years a UTCTime carries, as RFC 5280 section 4.1.2.5.1 maps its two digits. */
const UTC_TIME_YEARS = { first: 1950, last: 2049 } as const;

/** A self-signed ECDSA P-256 certificate for 127.0.0.1, which test servers present. */
export class TestCertificate {
    /** The certificate, in PEM. */
    readonly certificate: string;
    /** The PKCS #8 private key, in PEM. */
    readonly key: string;

    /** Create a certificate from its PEM and its key's PEM. */
    private constructor(certificate: string, key: string) {
        this.certificate = certificate;
        this.key = key;
    }

    /** Generate a fresh key and a certificate it signs for 127.0.0.1. */
    static async generate(): Promise<TestCertificate> {
        // generate the key pair
        const pair = await crypto.subtle.generateKey({ name: "ECDSA", namedCurve: "P-256" }, true, [
            "sign",
        ]);
        const publicKey = new Uint8Array(await crypto.subtle.exportKey("spki", pair.publicKey));
        const privateKey = new Uint8Array(await crypto.subtle.exportKey("pkcs8", pair.privateKey));

        // describe the certificate: issuer and subject 127.0.0.1, valid an hour either side of now
        const algorithm = der(TAG.sequence, der(TAG.identifier, OBJECT.ecdsaWithSha256));
        const name = der(
            TAG.sequence,
            der(
                TAG.set,
                der(
                    TAG.sequence,
                    der(TAG.identifier, OBJECT.commonName),
                    der(TAG.utf8String, new TextEncoder().encode("127.0.0.1")),
                ),
            ),
        );
        const now = Date.now();
        const validity = der(
            TAG.sequence,
            der(TAG.utcTime, utcTime(new Date(now - VALIDITY_MILLISECONDS))),
            der(TAG.utcTime, utcTime(new Date(now + VALIDITY_MILLISECONDS))),
        );

        // list 127.0.0.1 as the subject's IP address, which TLS clients verify
        const alternativeNames = der(TAG.sequence, der(TAG.ipAddress, [127, 0, 0, 1]));
        const extensions = der(
            TAG.extensions,
            der(
                TAG.sequence,
                der(
                    TAG.sequence,
                    der(TAG.identifier, OBJECT.subjectAltName),
                    der(TAG.octetString, alternativeNames),
                ),
            ),
        );
        const body = der(
            TAG.sequence,
            der(TAG.version, der(TAG.integer, [VERSION_3])),
            der(TAG.integer, [1]),
            algorithm,
            name,
            validity,
            name,
            publicKey,
            extensions,
        );

        // sign the body and wrap it with its signature
        const signature = await crypto.subtle.sign(
            { name: "ECDSA", hash: "SHA-256" },
            pair.privateKey,
            body,
        );
        const certificate = der(
            TAG.sequence,
            body,
            algorithm,
            der(TAG.bitString, [0], ecdsaSignature(new Uint8Array(signature))),
        );

        return new TestCertificate(pem("CERTIFICATE", certificate), pem("PRIVATE KEY", privateKey));
    }
}

/** Encode one DER element: its tag, its length and its contents. */
function der(tag: number, ...contents: readonly ArrayLike<number>[]): Uint8Array<ArrayBuffer> {
    // refuse contents too long for a two-byte length
    const length = contents.reduce((sum, part) => sum + part.length, 0);
    if (length > MAX_DER_LENGTH) {
        throw new RangeError(`der contents of ${length} bytes exceed ${MAX_DER_LENGTH}`);
    }

    // write the length in its shortest form, as DER requires
    const header =
        length < 0x80
            ? [tag, length]
            : length < 0x100
              ? [tag, 0x81, length]
              : [tag, 0x82, (length >> 8) & 0xff, length & 0xff];

    // copy the contents after the header
    const element = new Uint8Array(header.length + length);
    element.set(header);
    let offset = header.length;
    for (const part of contents) {
        element.set(part, offset);
        offset += part.length;
    }

    return element;
}

/** Encode a WebCrypto ECDSA signature, r and s side by side, as the DER sequence X.509 carries. */
function ecdsaSignature(raw: Uint8Array): Uint8Array {
    const half = raw.length / 2;

    return der(TAG.sequence, integer(raw.slice(0, half)), integer(raw.slice(half)));
}

/** Encode unsigned big-endian bytes as a DER integer, as short as its sign allows. */
function integer(bytes: Uint8Array): Uint8Array {
    // drop leading zeros, keeping one byte
    let start = 0;
    while (start < bytes.length - 1 && bytes[start] === 0) {
        start += 1;
    }
    const trimmed = bytes.slice(start);
    const first = trimmed[0];
    if (first === undefined) {
        throw new TypeError("integer has no bytes");
    }

    // pad a high first byte so the integer stays positive
    return der(TAG.integer, first >= 0x80 ? [0, ...trimmed] : trimmed);
}

/** Write a time as a DER UTCTime, such as 260929100000Z. */
function utcTime(date: Date): Uint8Array {
    // refuse a year outside the two digits
    const year = date.getUTCFullYear();
    if (year < UTC_TIME_YEARS.first || year > UTC_TIME_YEARS.last) {
        throw new RangeError(
            `utc time year ${year} is outside ${UTC_TIME_YEARS.first} through ${UTC_TIME_YEARS.last}`,
        );
    }

    // keep the year's last two digits through the seconds
    const digits = date.toISOString().replaceAll(/[-:T]/gu, "").slice(2, 14);

    return new TextEncoder().encode(`${digits}Z`);
}

/** Write DER bytes as PEM under a label. */
function pem(label: string, bytes: Uint8Array): string {
    const lines = bytes.toBase64().match(/.{1,64}/gu);
    if (lines === null) {
        throw new TypeError("pem has no bytes");
    }

    return `-----BEGIN ${label}-----\n${lines.join("\n")}\n-----END ${label}-----\n`;
}
