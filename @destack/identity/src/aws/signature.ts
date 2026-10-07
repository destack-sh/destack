/** The algorithm name Signature Version 4 writes into its string to sign, its header and its query. */
export const AWS_ALGORITHM = "AWS4-HMAC-SHA256";

/** The payload hash of a request whose body the signature leaves out. */
export const UNSIGNED_PAYLOAD = "UNSIGNED-PAYLOAD";

/** The hexadecimal SHA-256 of no bytes. */
export const EMPTY_HASH = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

/** The last element of every credential scope. */
const TERMINATOR = "aws4_request";

/** The signing keys a signer keeps, one per day it signed on recently, as a day's key signs every request of that day. */
const CACHED_DAYS = 2;

/** An AWS identity's credentials: an access key, its secret, and a session token for temporary ones. */
export interface AwsCredentials {
    /** The access key id. */
    readonly accessKeyId: string;
    /** The secret access key. */
    readonly secretAccessKey: string;
    /** The session token of temporary credentials. */
    readonly sessionToken?: string;
}

/** The parts of an HTTP request a signature covers, as its service canonicalizes them. */
export interface SignableRequest {
    /** The method, such as POST. */
    readonly method: string;
    /** The canonical path, written by `AwsSigner.path`. */
    readonly path: string;
    /** The decoded query parameters, which the signer encodes and sorts. */
    readonly query: readonly (readonly [string, string])[];
    /** The headers the signature covers by name, the host among them. */
    readonly headers: Readonly<Record<string, string>>;
    /** The hexadecimal SHA-256 of the body, or `UNSIGNED-PAYLOAD`. */
    readonly payloadHash: string;
}

/** A signature over a request and what it covers. */
export interface AwsSignature {
    /** The signing time as SigV4 writes it, such as 20150830T123600Z. */
    readonly timestamp: string;
    /** The credential scope, such as 20150830/us-east-1/s3/aws4_request. */
    readonly scope: string;
    /** The signed header names, lowercase, sorted and joined by semicolons. */
    readonly signedHeaders: string;
    /** The canonical request. */
    readonly canonicalRequest: string;
    /** The string to sign. */
    readonly stringToSign: string;
    /** The hexadecimal signature. */
    readonly signature: string;
}

/** AWS Signature Version 4 of one service in one region over WebCrypto, keeping the signing key of each recent day. */
export class AwsSigner {
    /** The region, such as eu-central-1. */
    readonly region: string;
    /** The service's signing name, such as ses or s3. */
    readonly service: string;
    /** The signing credentials. */
    readonly credentials: AwsCredentials;
    /** The signing keys of recent days, by day. */
    readonly #keys = new Map<string, Promise<CryptoKey>>();

    /** Sign for a service in a region with credentials. */
    constructor(options: {
        readonly region: string;
        readonly service: string;
        readonly credentials: AwsCredentials;
    }) {
        this.region = options.region;
        this.service = options.service;
        this.credentials = options.credentials;
    }

    /** Write a URL's path as SigV4 canonicalizes it: each segment encoded again, or as S3 reads it, decoded and encoded once. */
    static path(pathname: string, encoding: "double" | "single"): string {
        // encode each segment of the once-encoded path again
        if (encoding === "double") {
            return pathname === "" ? "/" : pathname.split("/").map(encode).join("/");
        }

        return encode(decodeURIComponent(pathname)).replaceAll("%2F", "/");
    }

    /** Write a time as SigV4's basic ISO 8601 form, such as 20150830T123600Z. */
    static timestamp(date: Date): string {
        return date
            .toISOString()
            .replaceAll(/[-:]/gu, "")
            .replace(/\.\d{3}/u, "");
    }

    /** Digest a body as the hexadecimal SHA-256 a signature covers. */
    static async payloadHash(body: string | Uint8Array<ArrayBuffer>): Promise<string> {
        const bytes = typeof body === "string" ? new TextEncoder().encode(body) : body;

        return hex(await crypto.subtle.digest("SHA-256", bytes));
    }

    /** Sign a request at a time, its headers covering the signing time and any session token. */
    async sign(request: SignableRequest, date: Date): Promise<AwsSignature> {
        // write the canonical headers in lower case, sorted, values trimmed with inner runs of spaces collapsed
        const timestamp = AwsSigner.timestamp(date);
        const canonical = new Map<string, string>();
        for (const [name, value] of Object.entries(request.headers)) {
            canonical.set(name.toLowerCase(), value.trim().replaceAll(/\s+/gu, " "));
        }
        const names = [...canonical.keys()].toSorted();
        const signedHeaders = names.join(";");

        // write the canonical request: method, path, query, headers, signed names and the payload hash
        const canonicalRequest = [
            request.method,
            request.path,
            writeQuery(request.query),
            ...names.map((name) => `${name}:${canonical.get(name) ?? ""}`),
            "",
            signedHeaders,
            request.payloadHash,
        ].join("\n");

        // write the string to sign over the canonical request's digest and the credential scope
        const scope = this.scope(timestamp);
        const stringToSign = [
            AWS_ALGORITHM,
            timestamp,
            scope,
            await AwsSigner.payloadHash(canonicalRequest),
        ].join("\n");

        return {
            timestamp,
            scope,
            signedHeaders,
            canonicalRequest,
            stringToSign,
            signature: await this.signString(stringToSign, timestamp),
        };
    }

    /** Sign an HTTP request with an Authorization header, answering the headers to send. */
    async authorize(
        request: Omit<SignableRequest, "headers"> & {
            /** The headers to sign beside the date and session token the signer adds. */
            readonly headers: Readonly<Record<string, string>>;
        },
        date: Date,
    ): Promise<Record<string, string>> {
        // add the signing time and a session token to the signed headers
        const token = this.credentials.sessionToken;
        const headers: Record<string, string> = {
            ...request.headers,
            "x-amz-date": AwsSigner.timestamp(date),
            ...(token === undefined || token === "" ? {} : { "x-amz-security-token": token }),
        };

        // sign them and write the Authorization header
        const signed = await this.sign({ ...request, headers }, date);
        const credential = `${this.credentials.accessKeyId}/${signed.scope}`;
        const authorization = `${AWS_ALGORITHM} Credential=${credential}, SignedHeaders=${signed.signedHeaders}, Signature=${signed.signature}`;

        return { ...headers, authorization };
    }

    /** Presign a request for a lifetime in seconds, answering the query parameters that authenticate it, the signature last. */
    async presign(
        request: SignableRequest,
        date: Date,
        expiresIn: number,
    ): Promise<[string, string][]> {
        // add the authentication parameters to the signed query
        const timestamp = AwsSigner.timestamp(date);
        const names = Object.keys(request.headers)
            .map((name) => name.toLowerCase())
            .toSorted()
            .join(";");
        const token = this.credentials.sessionToken;
        const authentication: [string, string][] = [
            ["X-Amz-Algorithm", AWS_ALGORITHM],
            ["X-Amz-Credential", `${this.credentials.accessKeyId}/${this.scope(timestamp)}`],
            ["X-Amz-Date", timestamp],
            ["X-Amz-Expires", String(expiresIn)],
        ];
        if (token !== undefined && token !== "") {
            authentication.push(["X-Amz-Security-Token", token]);
        }
        authentication.push(["X-Amz-SignedHeaders", names]);

        // sign the query with them and append the signature
        const signed = await this.sign(
            { ...request, query: [...request.query, ...authentication] },
            date,
        );

        return [...authentication, ["X-Amz-Signature", signed.signature]];
    }

    /** Sign a string to sign with the key of a timestamp's day, as streamed chunks are signed. */
    async signString(stringToSign: string, timestamp: string): Promise<string> {
        const key = await this.#key(timestamp.slice(0, 8));
        const signed = await crypto.subtle.sign(
            "HMAC",
            key,
            new TextEncoder().encode(stringToSign),
        );

        return hex(signed);
    }

    /** Write the credential scope of a timestamp's day. */
    scope(timestamp: string): string {
        return `${timestamp.slice(0, 8)}/${this.region}/${this.service}/${TERMINATOR}`;
    }

    /** Derive the signing key of a day once, keeping the keys of the most recent days. */
    #key(day: string): Promise<CryptoKey> {
        // reuse a recent day's key
        const kept = this.#keys.get(day);
        if (kept !== undefined) {
            return kept;
        }

        // derive it from the secret through the day, region, service and terminator
        const deriving = (async () => {
            let key = await hmac(
                new TextEncoder().encode(`AWS4${this.credentials.secretAccessKey}`),
                day,
            );
            for (const part of [this.region, this.service, TERMINATOR]) {
                key = await hmac(key, part);
            }

            return crypto.subtle.importKey("raw", key, { name: "HMAC", hash: "SHA-256" }, false, [
                "sign",
            ]);
        })();

        // forget the oldest day beyond the kept ones
        this.#keys.set(day, deriving);
        const [oldest] = this.#keys.keys();
        if (this.#keys.size > CACHED_DAYS && oldest !== undefined) {
            this.#keys.delete(oldest);
        }

        return deriving;
    }
}

/** Write the canonical query: each pair encoded, sorted by name, then by value. */
function writeQuery(query: readonly (readonly [string, string])[]): string {
    return query
        .map(([name, value]) => [encode(name), encode(value)] as const)
        .toSorted(([leftName, leftValue], [rightName, rightValue]) =>
            leftName === rightName ? compare(leftValue, rightValue) : compare(leftName, rightName),
        )
        .map(([name, value]) => `${name}=${value}`)
        .join("&");
}

/** Order two strings by code unit, as SigV4 sorts. */
function compare(left: string, right: string): number {
    return left < right ? -1 : left > right ? 1 : 0;
}

/** Percent-encode every UTF-8 byte outside RFC 3986's unreserved characters, in upper-case hex. */
function encode(text: string): string {
    return encodeURIComponent(text).replaceAll(
        /[!'()*]/gu,
        (character) => `%${character.charCodeAt(0).toString(16).toUpperCase()}`,
    );
}

/** Sign text with HMAC-SHA256 under a raw key. */
async function hmac(key: Uint8Array<ArrayBuffer>, text: string): Promise<Uint8Array<ArrayBuffer>> {
    const imported = await crypto.subtle.importKey(
        "raw",
        key,
        { name: "HMAC", hash: "SHA-256" },
        false,
        ["sign"],
    );

    return new Uint8Array(
        await crypto.subtle.sign("HMAC", imported, new TextEncoder().encode(text)),
    );
}

/** Write bytes as lower-case hex. */
function hex(bytes: ArrayBuffer | Uint8Array<ArrayBuffer>): string {
    return new Uint8Array(bytes).toHex();
}
