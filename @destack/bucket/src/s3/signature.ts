import {
    AWS_ALGORITHM,
    AwsSigner,
    EMPTY_HASH,
    type SignableRequest,
    UNSIGNED_PAYLOAD,
} from "@destack/identity/aws";
import { aligned, present } from "@destack/schema";
import { copyRequest } from "@destack/service/request";
import type { S3Credentials } from "./credentials.ts";
import { S3Error } from "./error.ts";
import { decodeUri, encodeUri, readQuery } from "./uri.ts";

/** The service S3 requests are signed for. */
const SERVICE = "s3";
/** The headers a signer leaves unsigned, which proxies and clients may change in flight. */
const UNSIGNED_HEADERS: ReadonlySet<string> = new Set([
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
/** The header declaring the payload hash a signature covers. */
const PAYLOAD_HASH_HEADER = "x-amz-content-sha256";
/** The query parameter carrying a presigned URL's signature. */
const SIGNATURE_PARAMETER = "X-Amz-Signature";
/** The query parameter listing the headers a presigned URL signs. */
const SIGNED_HEADERS_PARAMETER = "X-Amz-SignedHeaders";
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
    SIGNED_HEADERS_PARAMETER,
    SIGNATURE_PARAMETER,
    "X-Amz-Security-Token",
] as const;

/** A verified request signature, and how its streamed chunks chain their signatures to it. */
export interface S3Authorization {
    /** The verified access key. */
    readonly accessKeyId: string;
    /** Sign a string with the key of the request's day, region and service, as streamed chunks are signed. */
    readonly sign: (value: string) => Promise<string>;
    /** The request time as SigV4 writes it. */
    readonly date: string;
    /** The credential scope. */
    readonly scope: string;
    /** The verified signature, which seeds streamed chunk signatures. */
    readonly signature: string;
    /** The payload hash the signature covers. */
    readonly payloadHash: string;
}

/** What a client's signature covers, beside the request's path and query. */
interface SignedPart {
    /** The signed header names, lowercase and sorted. */
    readonly names: string[];
    /** The request time as SigV4 writes it. */
    readonly date: string;
    /** The payload hash the signature covers. */
    readonly payloadHash: string;
}

/** The authentication parameters of a presigned URL. */
interface Presigned {
    /** The credential scope with its access key. */
    readonly credential: string;
    /** The signing time as SigV4 writes it. */
    readonly date: string;
    /** The signing time. */
    readonly time: number;
    /** How long the URL lasts, in seconds. */
    readonly lifetime: number;
    /** The signed header names. */
    readonly names: string[];
    /** The signature. */
    readonly signature: string;
}

/** AWS Signature Version 4 as S3 applies it in one region: signing, presigning and verifying requests, keeping a signer per access key. */
export class S3Signature {
    /** The signed region. */
    readonly region: string;
    /** The signer of each access key, which keeps its daily signing keys. */
    readonly #signers = new Map<string, AwsSigner>();

    /** Sign for a region. */
    constructor(options: { region: string }) {
        this.region = options.region;
    }

    /** Sign a request with an Authorization header, leaving a body without a declared hash unsigned. */
    async sign(request: Request, credentials: S3Credentials, now: number): Promise<Request> {
        // stamp the payload hash the signature covers
        const headers = new Headers(request.headers);
        const payloadHash =
            headers.get(PAYLOAD_HASH_HEADER) ??
            (request.body === null ? EMPTY_HASH : UNSIGNED_PAYLOAD);
        headers.set(PAYLOAD_HASH_HEADER, payloadHash);

        // sign every header clients and proxies keep, with the host and the time
        const url = new URL(request.url);
        const kept = new Headers();
        for (const [name, value] of withHost(url, headers)) {
            if (!UNSIGNED_HEADERS.has(name)) {
                kept.set(name, value);
            }
        }
        const signed = await this.#signer(credentials).authorize(
            signableOf(request.method, url, readQuery(url), kept),
            new Date(now),
        );

        // attach the time and the authorization to a copy of the request
        headers.set("x-amz-date", present(signed["x-amz-date"], "the signed time"));
        headers.set("authorization", present(signed["authorization"], "the signature"));

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

        // hoist the x-amz headers S3 also reads from a query, and leave an undeclared payload unsigned
        const url = new URL(request.url);
        const headers = withHost(url, new Headers(request.headers));
        const payloadHash = headers.get(PAYLOAD_HASH_HEADER) ?? UNSIGNED_PAYLOAD;
        const query = readQuery(url);
        const signed = new Headers();
        for (const [name, value] of headers) {
            if (name.startsWith("x-amz-") && !isHeaderOnly(name)) {
                query.push([name, value]);
            } else if (!UNSIGNED_HEADERS.has(name)) {
                signed.set(name, value);
            }
        }
        const authentication = await this.#signer(credentials).presign(
            signableOf(request.method, url, query, signed, payloadHash),
            new Date(now),
            expiresIn,
        );

        // write the signed query, the signature last, and send the headers left in place
        const signature = present(authentication.pop(), "the presigned signature");
        url.search = `${writeQuery([...query, ...authentication])}&${SIGNATURE_PARAMETER}=${signature[1]}`;
        const sent = new Headers();
        for (const [name, value] of request.headers) {
            if (!name.startsWith("x-amz-") || isHeaderOnly(name)) {
                sent.set(name, value);
            }
        }

        return new Request(url.href, { method: request.method, headers: sent });
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

    /** Verify an Authorization header signature by signing the same parts again. */
    async #verifyHeader(
        request: Request,
        url: URL,
        header: string,
        lookup: (accessKeyId: string) => Promise<S3Credentials | undefined>,
        now: number,
    ): Promise<S3Authorization> {
        // read the algorithm, credential, signed headers and signature
        const match =
            /^AWS4-HMAC-SHA256 +Credential=([^,\s]+), *SignedHeaders=([^,\s]+), *Signature=([0-9a-f]{64})$/u.exec(
                header.trim(),
            );
        if (match === null) {
            const code = header.startsWith(AWS_ALGORITHM)
                ? "AuthorizationHeaderMalformed"
                : "InvalidRequest";
            throw new S3Error(code, "the authorization header must use AWS4-HMAC-SHA256");
        }
        const credential = aligned(match, 1);
        const names = aligned(match, 2).split(";");

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
        const payloadHash = request.headers.get(PAYLOAD_HASH_HEADER);
        if (payloadHash === null) {
            throw new S3Error(
                "InvalidRequest",
                `missing required header for this request: ${PAYLOAD_HASH_HEADER}`,
            );
        }
        checkSignedHeaders(request.headers, names);

        // sign the signed parts again
        const headers = signedHeaders(url, request.headers, names);
        const signingDate = new Date(time);
        const parts = { names, date, payloadHash };
        const credentials = await requireCredentials(lookup, accessKeyId);
        const signed = await this.#signer(credentials).sign(
            signableOf(request.method, url, readQuery(url), headers, payloadHash),
            signingDate,
        );

        return this.#match(
            parts,
            credentials,
            aligned(match, 3),
            signed.signedHeaders,
            signed.signature,
        );
    }

    /** Verify a presigned URL's query signature by presigning the same parts again. */
    async #verifyQuery(
        request: Request,
        url: URL,
        lookup: (accessKeyId: string) => Promise<S3Credentials | undefined>,
        now: number,
    ): Promise<S3Authorization> {
        // refuse expired URLs and URLs signed for a later time
        const query = readQuery(url);
        const presigned = readPresigned(query);
        const { date, time, lifetime, names } = presigned;
        if (now > time + lifetime * 1000) {
            throw new S3Error("AccessDenied", "request has expired");
        }
        if (now < time - MAX_CLOCK_SKEW) {
            throw new S3Error("AccessDenied", "request is not valid yet");
        }
        const credential = presigned.credential;
        const code = "AuthorizationQueryParametersError";
        const accessKeyId = this.#checkCredential(credential, date, code);
        checkSignedHeaders(request.headers, names);

        // presign the signed parts again, an undeclared payload unsigned
        const headers = signedHeaders(url, request.headers, names);
        const payloadHash = headers.get(PAYLOAD_HASH_HEADER) ?? UNSIGNED_PAYLOAD;
        const unsigned = query.filter(
            ([name]) => !PRESIGN_PARAMETERS.some((each) => each === name),
        );
        const signingDate = new Date(time);
        const parts = { names, date, payloadHash };
        const credentials = await requireCredentials(lookup, accessKeyId);
        const resigned = new Map(
            await this.#signer(credentials).presign(
                signableOf(request.method, url, unsigned, headers, payloadHash),
                signingDate,
                lifetime,
            ),
        );

        // compare the signature over the same headers
        return this.#match(
            parts,
            credentials,
            presigned.signature,
            resigned.get(SIGNED_HEADERS_PARAMETER),
            resigned.get(SIGNATURE_PARAMETER),
        );
    }

    /** Keep the signer of an access key, replaced once its secret changes. */
    #signer(credentials: S3Credentials): AwsSigner {
        // reuse the signer of an unchanged key
        const kept = this.#signers.get(credentials.accessKeyId);
        if (kept?.credentials.secretAccessKey === credentials.secretAccessKey) {
            return kept;
        }

        // sign with the new secret from now on
        const signer = new AwsSigner({ region: this.region, service: SERVICE, credentials });
        this.#signers.set(credentials.accessKeyId, signer);

        return signer;
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
            accessKeyId === undefined ||
            accessKeyId === "" ||
            rest.length !== 0 ||
            terminator !== TERMINATOR ||
            service !== SERVICE
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

    /** Accept a client's signature once it equals, in constant time, the one signed again over the same headers. */
    #match(
        parts: SignedPart,
        credentials: S3Credentials,
        signature: string,
        listed: unknown,
        expected: unknown,
    ): S3Authorization {
        // refuse a signature over other headers, or another signature
        if (
            listed !== parts.names.join(";") ||
            typeof expected !== "string" ||
            !isEqual(expected, signature)
        ) {
            throw new S3Error(
                "SignatureDoesNotMatch",
                "the request signature does not match the signature calculated with the secret key",
            );
        }

        // sign chunks with the same key and time
        const chunks = this.#signer(credentials);
        const { date, payloadHash } = parts;

        return {
            accessKeyId: credentials.accessKeyId,
            sign: (value) => chunks.signString(value, date),
            date,
            scope: `${date.slice(0, 8)}/${this.region}/${SERVICE}/${TERMINATOR}`,
            signature,
            payloadHash,
        };
    }
}

/** Describe a request as S3 signs it: its path decoded and encoded once, its query, its headers and its payload hash. */
function signableOf(
    method: string,
    url: URL,
    query: readonly (readonly [string, string])[],
    headers: Headers,
    payloadHash = headers.get(PAYLOAD_HASH_HEADER) ?? EMPTY_HASH,
): SignableRequest {
    return {
        method,
        path: encodeUri(decodeUri(url.pathname), true),
        query,
        headers: Object.fromEntries(headers),
        payloadHash,
    };
}

/** Look up the credentials of an access key, refusing an unknown one. */
async function requireCredentials(
    lookup: (accessKeyId: string) => Promise<S3Credentials | undefined>,
    accessKeyId: string,
): Promise<S3Credentials> {
    const credentials = await lookup(accessKeyId);
    if (credentials === undefined) {
        throw new S3Error("InvalidAccessKeyId", "the access key does not exist");
    }

    return credentials;
}

/** Read and check the authentication parameters of a presigned URL's query. */
function readPresigned(query: [string, string][]): Presigned {
    // require every authentication parameter but the session token, which is unsupported
    const parameters = new Map(query);
    const [algorithm, credential, date, expires, listed, signature] = PRESIGN_PARAMETERS.map(
        (name) => parameters.get(name),
    );
    if (
        algorithm !== AWS_ALGORITHM ||
        credential === undefined ||
        date === undefined ||
        expires === undefined ||
        listed === undefined ||
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
    const lifetime = /^\d+$/u.test(expires) ? Number(expires) : Number.NaN;
    const time = parseDate(date);
    if (!(lifetime >= 1 && lifetime <= MAX_EXPIRES) || time === undefined) {
        throw new S3Error(
            "AuthorizationQueryParametersError",
            "X-Amz-Expires must be from 1 through 604800 seconds and X-Amz-Date a valid time",
        );
    }

    return { credential, date, time, lifetime, names: listed.split(";"), signature };
}

/** Keep the headers a signature lists, with the host HTTP sends. */
function signedHeaders(url: URL, headers: Headers, names: string[]): Headers {
    const signed = new Headers();
    for (const name of names) {
        const value = headers.get(name);
        if (value !== null) {
            signed.set(name, value);
        }
    }

    return withHost(url, signed);
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
function checkSignedHeaders(headers: Headers, names: string[]): void {
    const signed = new Set(names);
    if (!signed.has("host")) {
        throw new S3Error("AccessDenied", "the host header must be signed");
    }
    for (const name of headers.keys()) {
        if (name.startsWith("x-amz-") && !signed.has(name)) {
            throw new S3Error("AccessDenied", `the header ${name} is present but not signed`);
        }
    }
}

/** Write query parameters encoded and sorted by name, then by value, as SigV4 orders them. */
function writeQuery(query: readonly (readonly [string, string])[]): string {
    return query
        .map(([name, value]) => [encodeUri(name, false), encodeUri(value, false)] as const)
        .toSorted(([leftName, leftValue], [rightName, rightValue]) =>
            leftName === rightName ? compare(leftValue, rightValue) : compare(leftName, rightName),
        )
        .map(([name, value]) => `${name}=${value}`)
        .join("&");
}

/** Whether S3 reads a header only from headers, never from a presigned query. */
function isHeaderOnly(name: string): boolean {
    return name === PAYLOAD_HASH_HEADER || name.startsWith("x-amz-server-side-encryption");
}

/** Parse SigV4's basic ISO 8601 form, or return undefined when malformed. */
function parseDate(value: string): number | undefined {
    // match the fixed-width fields
    const match = /^(\d{4})(\d{2})(\d{2})T(\d{2})(\d{2})(\d{2})Z$/u.exec(value);
    if (match === null) {
        return undefined;
    }
    const time = Date.parse(
        `${match[1]}-${match[2]}-${match[3]}T${match[4]}:${match[5]}:${match[6]}Z`,
    );

    return Number.isNaN(time) ? undefined : time;
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
