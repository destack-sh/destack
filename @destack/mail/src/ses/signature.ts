/** The algorithm name Signature Version 4 writes into its string to sign and its header. */
const ALGORITHM = "AWS4-HMAC-SHA256";

/** The characters RFC 3986 leaves unreserved, which SigV4 never escapes. */
const UNRESERVED = /[A-Za-z0-9\-._~]/u;

/** An AWS identity's credentials: an access key, its secret, and a session token for temporary ones. */
export interface AwsCredentials {
    /** The access key id. */
    readonly accessKeyId: string;
    /** The secret access key. */
    readonly secretAccessKey: string;
    /** The session token of temporary credentials. */
    readonly sessionToken?: string;
}

/** The parts of an HTTP request Signature Version 4 covers. */
export interface SignableRequest {
    /** The method, such as POST. */
    readonly method: string;
    /** The URL, whose host, path and query the signature covers. */
    readonly url: URL;
    /** The headers to sign, beside the host and the date the signer adds. */
    readonly headers: Readonly<Record<string, string>>;
    /** The body. */
    readonly body: string | Uint8Array<ArrayBuffer>;
}

/** Who signs, for which service in which region, and when. */
export interface Signing {
    /** The signing credentials. */
    readonly credentials: AwsCredentials;
    /** The region, such as eu-central-1. */
    readonly region: string;
    /** The service's signing name, such as ses. */
    readonly service: string;
    /** The signing time. */
    readonly date: Date;
}

/** A signed request's headers and the intermediate texts its signature derives from. */
export interface SignedRequest {
    /** The headers to send: the signed ones, the date, the session token and the authorization. */
    readonly headers: Readonly<Record<string, string>>;
    /** The canonical request. */
    readonly canonicalRequest: string;
    /** The string to sign. */
    readonly stringToSign: string;
    /** The Authorization header. */
    readonly authorization: string;
}

/** Sign a request with AWS Signature Version 4 over WebCrypto, as Bun, workerd and browsers run it. */
export async function signRequest(
    request: SignableRequest,
    signing: Signing,
): Promise<SignedRequest> {
    // add the host, the date and a session token to the signed headers
    const timestamp = writeTimestamp(signing.date);
    const day = timestamp.slice(0, 8);
    const token = signing.credentials.sessionToken;
    const headers: Record<string, string> = {
        ...request.headers,
        host: request.url.host,
        "x-amz-date": timestamp,
        ...(token === undefined || token === "" ? {} : { "x-amz-security-token": token }),
    };

    // write the canonical headers in lower case, sorted, values trimmed with inner runs of spaces collapsed
    const canonical = new Map<string, string>();
    for (const [name, value] of Object.entries(headers)) {
        canonical.set(name.toLowerCase(), value.trim().replaceAll(/\s+/gu, " "));
    }
    const names = [...canonical.keys()].toSorted();
    const signedHeaders = names.join(";");

    // write the canonical request: method, path, query, headers, signed names and the body's digest
    const body =
        typeof request.body === "string" ? new TextEncoder().encode(request.body) : request.body;
    const canonicalRequest = [
        request.method,
        writePath(request.url.pathname),
        writeQuery(request.url.searchParams),
        ...names.map((name) => `${name}:${canonical.get(name) ?? ""}`),
        "",
        signedHeaders,
        hex(await crypto.subtle.digest("SHA-256", body)),
    ].join("\n");

    // write the string to sign over the request's digest and the credential scope
    const scope = `${day}/${signing.region}/${signing.service}/aws4_request`;
    const stringToSign = [
        ALGORITHM,
        timestamp,
        scope,
        hex(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(canonicalRequest))),
    ].join("\n");

    // derive the day's signing key and sign
    let key = await hmac(
        new TextEncoder().encode(`AWS4${signing.credentials.secretAccessKey}`),
        day,
    );
    for (const part of [signing.region, signing.service, "aws4_request"]) {
        key = await hmac(key, part);
    }
    const signature = hex(await hmac(key, stringToSign));
    const authorization = `${ALGORITHM} Credential=${signing.credentials.accessKeyId}/${scope}, SignedHeaders=${signedHeaders}, Signature=${signature}`;

    return {
        headers: { ...headers, authorization },
        canonicalRequest,
        stringToSign,
        authorization,
    };
}

/** Write a time as SigV4's basic ISO 8601 form, such as 20150830T123600Z. */
function writeTimestamp(date: Date): string {
    return date
        .toISOString()
        .replaceAll(/[-:]/gu, "")
        .replace(/\.\d{3}/u, "");
}

/** Write the canonical path: each segment of the once-encoded path encoded again. */
function writePath(pathname: string): string {
    return pathname === "" ? "/" : pathname.split("/").map(encode).join("/");
}

/** Write the canonical query: each pair encoded, sorted by name, then by value. */
function writeQuery(parameters: URLSearchParams): string {
    return [...parameters]
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
    return Array.from(new TextEncoder().encode(text), (byte) => {
        const character = String.fromCharCode(byte);

        return byte < 0x80 && UNRESERVED.test(character)
            ? character
            : `%${byte.toString(16).toUpperCase().padStart(2, "0")}`;
    }).join("");
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
    return Array.from(new Uint8Array(bytes), (byte) => byte.toString(16).padStart(2, "0")).join("");
}
