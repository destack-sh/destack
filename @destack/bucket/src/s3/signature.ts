import { Digest } from "@destack/schema";
import { copyRequest } from "@destack/service/request";
import type { S3Credentials } from "./credentials.ts";
import { S3Error } from "./error.ts";
import { decodeUri, encodeUri, readQuery } from "./uri.ts";

/** The SigV4 signing algorithm. */
const ALGORITHM = "AWS4-HMAC-SHA256";
/** The payload hash of requests whose body stays unsigned. */
export const UNSIGNED_PAYLOAD = "UNSIGNED-PAYLOAD";
/** The hexadecimal SHA-256 of no bytes. */
export const EMPTY_HASH = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
/** The last element of every SigV4 credential scope. */
const TERMINATOR = "aws4_request";
/** The widest clock difference header authentication accepts, fifteen minutes as S3 allows. */
const MAX_CLOCK_SKEW = 15 * 60 * 1000;
/** The longest presigned URL lifetime in seconds, seven days as S3 allows. */
const MAX_EXPIRES = 7 * 24 * 60 * 60;
/** The query parameters of presigned authentication, which never count as request headers. */
export const PRESIGN_PARAMETERS = [
    "X-Amz-Algorithm",
    "X-Amz-Credential",
    "X-Amz-Date",
    "X-Amz-Expires",
    "X-Amz-SignedHeaders",
    "X-Amz-Signature",
    "X-Amz-Security-Token",
] as const;
/** Headers left unsigned because proxies and clients change them, as the AWS SDK leaves them. */
const UNSIGNABLE_HEADERS = new Set([
    "authorization",
    "cache-control",
    "connection",
    "expect",
    "from",
    "keep-alive",
    "max-forwards",
    "pragma",
    "referer",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
    "user-agent",
    "x-amzn-trace-id",
]);

/** A verified request signature and the key its streamed chunks are signed with. */
export interface S3Authorization {
    /** The verified access key. */
    readonly accessKeyId: string;
    /** The derived signing key of the request's date, region and service. */
    readonly signingKey: CryptoKey;
    /** The request time as SigV4 writes it. */
    readonly date: string;
    /** The credential scope. */
    readonly scope: string;
    /** The verified signature, which seeds streamed chunk signatures. */
    readonly signature: string;
    /** The payload hash the signature covers. */
    readonly payloadHash: string;
}

/** A derived signing key and the day and secret it signs for. */
interface SigningKey {
    /** The day as SigV4 writes it. */
    day: string;
    /** The secret the key derives from. */
    secretAccessKey: string;
    /** The derived HMAC key. */
    key: CryptoKey;
}

/** The request parts a signature covers. */
interface SignedRequest {
    /** The HTTP method. */
    method: string;
    /** The percent-encoded URL path. */
    path: string;
    /** The decoded query parameters, without the signature itself. */
    query: [string, string][];
    /** The request headers, including those the query carries. */
    headers: Headers;
    /** The signed header names, lowercase and sorted. */
    signedHeaders: string[];
    /** The payload hash. */
    payloadHash: string;
}

/** The AWS Signature Version 4 signer and verifier for one region and service. */
export class SignatureV4 {
    /** The signed region. */
    readonly region: string;
    /** The signed service. */
    readonly service: string;
    /** The signing key derived last for each access key, with the day and secret it signs for. */
    readonly #signingKeys = new Map<string, SigningKey>();

    /** Sign for a region and service, S3 by default. */
    constructor(options: { region: string; service?: string }) {
        this.region = options.region;
        this.service = options.service ?? "s3";
    }

    /** Sign a request with an Authorization header, leaving a body without a declared hash unsigned. */
    async sign(request: Request, credentials: S3Credentials, now: number): Promise<Request> {
        // stamp the time and payload hash the signature covers
        const date = formatDate(now);
        const headers = new Headers(request.headers);
        headers.set("x-amz-date", date);
        if (!headers.has("x-amz-content-sha256")) {
            headers.set(
                "x-amz-content-sha256",
                request.body === null ? EMPTY_HASH : UNSIGNED_PAYLOAD,
            );
        }

        // sign every header clients and proxies keep, with the host
        const url = new URL(request.url);
        const signed = {
            method: request.method,
            path: url.pathname,
            query: readQuery(url),
            headers: withHost(url, headers),
            signedHeaders: signedHeaderNames(headers),
            payloadHash: headers.get("x-amz-content-sha256")!,
        };
        const scope = this.#scope(date);
        const signingKey = await this.#signingKey(credentials, date);
        const signature = await signRequest(signed, date, scope, signingKey);

        // attach the authorization to a copy of the request
        headers.set(
            "authorization",
            `${ALGORITHM} Credential=${credentials.accessKeyId}/${scope}, SignedHeaders=${signed.signedHeaders.join(";")}, Signature=${signature}`,
        );

        return copyRequest(request, { headers });
    }

    /** Presign a request, hoisting its x-amz headers into the query and keeping the headers it sends. */
    async presign(
        request: Request,
        credentials: S3Credentials,
        expiresIn: number,
        now: number,
    ): Promise<Request> {
        // refuse lifetimes S3 refuses
        if (!Number.isInteger(expiresIn) || expiresIn < 1 || expiresIn > MAX_EXPIRES) {
            throw new RangeError("presigned URLs expire after 1 second to 7 days");
        }

        // move x-amz headers into the query, except those S3 reads only as headers
        const url = new URL(request.url);
        const query = readQuery(url);
        const headers = new Headers();
        for (const [name, value] of request.headers) {
            if (name.startsWith("x-amz-") && !isHeaderOnly(name)) {
                query.push([name, value]);
            } else {
                headers.set(name, value);
            }
        }

        // add the authentication parameters and sign the canonical query
        const date = formatDate(now);
        const scope = this.#scope(date);
        const signedHeaders = signedHeaderNames(headers);
        query.push(
            ["X-Amz-Algorithm", ALGORITHM],
            ["X-Amz-Credential", `${credentials.accessKeyId}/${scope}`],
            ["X-Amz-Date", date],
            ["X-Amz-Expires", String(expiresIn)],
            ["X-Amz-SignedHeaders", signedHeaders.join(";")],
        );
        const signed = {
            method: request.method,
            path: url.pathname,
            query,
            headers: withHost(url, headers),
            signedHeaders,
            payloadHash: headers.get("x-amz-content-sha256") ?? UNSIGNED_PAYLOAD,
        };
        const signingKey = await this.#signingKey(credentials, date);
        const signature = await signRequest(signed, date, scope, signingKey);
        url.search = `${canonicalQuery(query)}&X-Amz-Signature=${signature}`;

        return new Request(url.href, { method: request.method, headers });
    }

    /** Verify a request's header or query signature and return what later chunk signatures need. */
    async authenticate(
        request: Request,
        lookup: (accessKeyId: string) => Promise<S3Credentials | undefined>,
        now: number,
    ): Promise<S3Authorization> {
        // find the authentication mechanisms the request uses
        const url = new URL(request.url);
        const header = request.headers.get("authorization");
        const isPresigned = readQuery(url).some(([name]) => name === "X-Amz-Algorithm");

        // accept exactly one authentication mechanism
        if (header !== null && isPresigned) {
            throw new S3Error("InvalidArgument", "only one authentication mechanism is allowed");
        }
        // verify query authentication
        else if (isPresigned) {
            return await this.#verifyQuery(request, url, lookup, now);
        }
        // verify header authentication
        else if (header !== null) {
            return await this.#verifyHeader(request, url, header, lookup, now);
        }
        // refuse anonymous requests
        else {
            throw new S3Error("AccessDenied", "anonymous requests are not allowed");
        }
    }

    /** Verify an Authorization header signature. */
    async #verifyHeader(
        request: Request,
        url: URL,
        header: string,
        lookup: (accessKeyId: string) => Promise<S3Credentials | undefined>,
        now: number,
    ): Promise<S3Authorization> {
        // read the algorithm, credential, signed headers and signature
        const match =
            /^AWS4-HMAC-SHA256 +Credential=([^,\s]+), *SignedHeaders=([^,\s]+), *Signature=([0-9a-f]{64})$/.exec(
                header.trim(),
            );
        if (match === null) {
            const code = header.startsWith(ALGORITHM)
                ? "AuthorizationHeaderMalformed"
                : "InvalidRequest";
            throw new S3Error(code, "the authorization header must use AWS4-HMAC-SHA256");
        }
        const [, credential, signedHeaders, signature] = match as unknown as [
            string,
            string,
            string,
            string,
        ];

        // require a request time within the clock skew and the credential's day
        const date = request.headers.get("x-amz-date");
        const time = date === null ? undefined : parseDate(date);
        if (date === null || time === undefined) {
            throw new S3Error("AccessDenied", "authentication requires a valid x-amz-date header");
        }
        if (Math.abs(now - time) > MAX_CLOCK_SKEW) {
            throw new S3Error(
                "RequestTimeTooSkewed",
                "the request time differs too much from the server time",
            );
        }
        const accessKeyId = this.#checkCredential(credential, date, "AuthorizationHeaderMalformed");

        // require the payload hash and a signature over every x-amz header
        const payloadHash = request.headers.get("x-amz-content-sha256");
        if (payloadHash === null) {
            throw new S3Error(
                "InvalidRequest",
                "missing required header for this request: x-amz-content-sha256",
            );
        }
        const names = signedHeaders.split(";");
        checkSignedHeaders(request.headers, names);

        // compare the signature of the canonical request
        const signed = {
            method: request.method,
            path: url.pathname,
            query: readQuery(url),
            headers: withHost(url, request.headers),
            signedHeaders: names,
            payloadHash,
        };

        return await this.#check(signed, date, accessKeyId, signature, lookup);
    }

    /** Verify a presigned URL's query signature. */
    async #verifyQuery(
        request: Request,
        url: URL,
        lookup: (accessKeyId: string) => Promise<S3Credentials | undefined>,
        now: number,
    ): Promise<S3Authorization> {
        // require every authentication parameter
        const query = readQuery(url);
        const parameters = new Map(query);
        const [algorithm, credential, date, expires, signedHeaders, signature] =
            PRESIGN_PARAMETERS.map((name) => parameters.get(name));
        if (
            algorithm !== ALGORITHM ||
            credential === undefined ||
            date === undefined ||
            expires === undefined ||
            signedHeaders === undefined ||
            signature === undefined
        ) {
            throw new S3Error(
                "AuthorizationQueryParametersError",
                "query authentication requires the X-Amz-Algorithm, X-Amz-Credential, X-Amz-Signature, X-Amz-Date, X-Amz-SignedHeaders and X-Amz-Expires parameters",
            );
        }
        if (parameters.has("X-Amz-Security-Token")) {
            throw new S3Error("InvalidToken", "session tokens are not supported");
        }

        // require a valid lifetime of at most seven days
        const lifetime = /^\d+$/.test(expires) ? Number(expires) : Number.NaN;
        const time = parseDate(date);
        if (!(lifetime >= 1 && lifetime <= MAX_EXPIRES) || time === undefined) {
            throw new S3Error(
                "AuthorizationQueryParametersError",
                "X-Amz-Expires must be from 1 through 604800 seconds and X-Amz-Date a valid time",
            );
        }

        // refuse expired URLs and URLs signed for a later time
        if (now > time + lifetime * 1000) {
            throw new S3Error("AccessDenied", "request has expired");
        }
        if (now < time - MAX_CLOCK_SKEW) {
            throw new S3Error("AccessDenied", "request is not valid yet");
        }
        const accessKeyId = this.#checkCredential(
            credential,
            date,
            "AuthorizationQueryParametersError",
        );

        // compare the signature over the query without the signature itself
        const names = signedHeaders.split(";");
        checkSignedHeaders(request.headers, names);
        const signed = {
            method: request.method,
            path: url.pathname,
            query: query.filter(([name]) => name !== "X-Amz-Signature"),
            headers: withHost(url, request.headers),
            signedHeaders: names,
            payloadHash: request.headers.get("x-amz-content-sha256") ?? UNSIGNED_PAYLOAD,
        };

        return await this.#check(signed, date, accessKeyId, signature, lookup);
    }

    /** Require a credential scope of this region and service on the request's day. */
    #checkCredential(
        credential: string,
        date: string,
        code: "AuthorizationHeaderMalformed" | "AuthorizationQueryParametersError",
    ): string {
        // require the five scope parts with this service
        const [accessKeyId, day, region, service, terminator, ...rest] = credential.split("/");
        if (
            !accessKeyId ||
            rest.length !== 0 ||
            terminator !== TERMINATOR ||
            service !== this.service
        ) {
            throw new S3Error(
                code,
                "the credential must name an access key, date, region, service and aws4_request",
            );
        }
        if (region !== this.region) {
            throw new S3Error(code, `the region ${region} is wrong; expecting ${this.region}`);
        }
        if (day !== date.slice(0, 8)) {
            throw new S3Error(code, "the credential date differs from the request date");
        }

        return accessKeyId;
    }

    /** Compare a request's signature with the one its access key produces. */
    async #check(
        signed: SignedRequest,
        date: string,
        accessKeyId: string,
        signature: string,
        lookup: (accessKeyId: string) => Promise<S3Credentials | undefined>,
    ): Promise<S3Authorization> {
        // look up the access key
        const credentials = await lookup(accessKeyId);
        if (credentials === undefined) {
            throw new S3Error("InvalidAccessKeyId", "the access key does not exist");
        }

        // compute and compare the signature in constant time
        const scope = this.#scope(date);
        const signingKey = await this.#signingKey(credentials, date);
        const expected = await signRequest(signed, date, scope, signingKey);
        if (!isEqual(expected, signature)) {
            throw new S3Error(
                "SignatureDoesNotMatch",
                "the request signature does not match the signature calculated with the secret key",
            );
        }

        return { accessKeyId, signingKey, date, scope, signature, payloadHash: signed.payloadHash };
    }

    /** The credential scope of a request time. */
    #scope(date: string): string {
        return `${date.slice(0, 8)}/${this.region}/${this.service}/${TERMINATOR}`;
    }

    /** Derive the signing key of a secret for a day, reusing the last one of its access key. */
    async #signingKey(credentials: S3Credentials, date: string): Promise<CryptoKey> {
        // reuse the key of the same day and secret
        const day = date.slice(0, 8);
        const cached = this.#signingKeys.get(credentials.accessKeyId);
        if (cached?.day === day && cached.secretAccessKey === credentials.secretAccessKey) {
            return cached.key;
        }

        // chain HMACs from the secret through the day, region, service and terminator
        const encoder = new TextEncoder();
        let bytes: Uint8Array<ArrayBuffer> = encoder.encode(`AWS4${credentials.secretAccessKey}`);
        for (const part of [day, this.region, this.service, TERMINATOR]) {
            bytes = await hmac(await importKey(bytes), part);
        }
        const key = await importKey(bytes);
        this.#signingKeys.set(credentials.accessKeyId, {
            day,
            secretAccessKey: credentials.secretAccessKey,
            key,
        });

        return key;
    }
}

/** Sign a string with a derived key and return the hexadecimal signature. */
export async function signString(signingKey: CryptoKey, value: string): Promise<string> {
    const signature = await crypto.subtle.sign("HMAC", signingKey, new TextEncoder().encode(value));

    return new Uint8Array(signature).toHex();
}

/** Sign the canonical form of a request. */
async function signRequest(
    signed: SignedRequest,
    date: string,
    scope: string,
    signingKey: CryptoKey,
): Promise<string> {
    // build the canonical request from the signed parts
    const canonicalHeaders = signed.signedHeaders
        .map((name) => `${name}:${(signed.headers.get(name) ?? "").trim().replace(/\s+/g, " ")}\n`)
        .join("");
    const canonical = [
        signed.method,
        encodeUri(decodeUri(signed.path), true),
        canonicalQuery(signed.query),
        canonicalHeaders,
        signed.signedHeaders.join(";"),
        signed.payloadHash,
    ].join("\n");

    // sign the string naming the algorithm, time, scope and canonical request hash
    const stringToSign = [ALGORITHM, date, scope, await Digest.of(canonical)].join("\n");

    return await signString(signingKey, stringToSign);
}

/** List the host and every signable header, lowercase and sorted. */
function signedHeaderNames(headers: Headers): string[] {
    const names = new Set(["host"]);
    for (const name of headers.keys()) {
        if (!UNSIGNABLE_HEADERS.has(name)) {
            names.add(name);
        }
    }

    return [...names].sort();
}

/** Add the URL's host to headers that lack one, as HTTP sends it. */
function withHost(url: URL, headers: Headers): Headers {
    if (headers.has("host")) {
        return headers;
    }
    const hosted = new Headers(headers);
    hosted.set("host", url.host);

    return hosted;
}

/** Refuse a signature that leaves the host or an x-amz header of the request unsigned. */
function checkSignedHeaders(headers: Headers, signedHeaders: string[]): void {
    const signed = new Set(signedHeaders);
    if (!signed.has("host")) {
        throw new S3Error("AccessDenied", "the host header must be signed");
    }
    for (const name of headers.keys()) {
        if (name.startsWith("x-amz-") && !signed.has(name)) {
            throw new S3Error("AccessDenied", `the header ${name} is present but not signed`);
        }
    }
}

/** Encode, sort and join query parameters as SigV4 canonicalizes them. */
function canonicalQuery(query: [string, string][]): string {
    return query
        .map(([name, value]) => [encodeUri(name, false), encodeUri(value, false)] as const)
        .sort(([leftName, leftValue], [rightName, rightValue]) =>
            leftName === rightName ? compare(leftValue, rightValue) : compare(leftName, rightName),
        )
        .map(([name, value]) => `${name}=${value}`)
        .join("&");
}

/** Whether S3 reads a header only from headers, never from a presigned query. */
function isHeaderOnly(name: string): boolean {
    return name === "x-amz-content-sha256" || name.startsWith("x-amz-server-side-encryption");
}

/** Format a time as SigV4's basic ISO 8601 form. */
function formatDate(now: number): string {
    return new Date(now)
        .toISOString()
        .replace(/[-:]/g, "")
        .replace(/\.\d{3}/, "");
}

/** Parse SigV4's basic ISO 8601 form, or return undefined when malformed. */
function parseDate(value: string): number | undefined {
    // match the fixed-width fields
    const match = /^(\d{4})(\d{2})(\d{2})T(\d{2})(\d{2})(\d{2})Z$/.exec(value);
    if (match === null) {
        return undefined;
    }
    const time = Date.parse(
        `${match[1]}-${match[2]}-${match[3]}T${match[4]}:${match[5]}:${match[6]}Z`,
    );

    return Number.isNaN(time) ? undefined : time;
}

/** Import raw bytes as an HMAC SHA-256 signing key. */
function importKey(bytes: Uint8Array<ArrayBuffer>): Promise<CryptoKey> {
    return crypto.subtle.importKey("raw", bytes, { name: "HMAC", hash: "SHA-256" }, false, [
        "sign",
    ]);
}

/** Sign text with a key and return the raw signature. */
async function hmac(key: CryptoKey, value: string): Promise<Uint8Array<ArrayBuffer>> {
    return new Uint8Array(await crypto.subtle.sign("HMAC", key, new TextEncoder().encode(value)));
}

/** Compare two strings by UTF-16 code units, which match bytes for encoded ASCII. */
function compare(left: string, right: string): number {
    return left < right ? -1 : left > right ? 1 : 0;
}

/** Compare two signatures without an early exit. */
function isEqual(left: string, right: string): boolean {
    let difference = left.length ^ right.length;
    for (let index = 0; index < left.length && index < right.length; index++) {
        difference |= left.charCodeAt(index) ^ right.charCodeAt(index);
    }

    return difference === 0;
}
