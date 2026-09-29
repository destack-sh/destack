/** The shape of an authorization header carrying one bearer token, per RFC 6750 2.1. */
const BEARER = /^Bearer (\S+)$/;

/** Read the one bearer token of a request without cookies, absent when it carries none or an invalid one. */
export function readBearer(headers: Headers): string | undefined {
    // refuse a malformed header, and a bearer token next to cookies
    const authorization = headers.get("authorization");
    const matched = authorization === null ? null : BEARER.exec(authorization);
    if (matched === null || headers.has("cookie")) {
        return undefined;
    }

    return matched[1];
}
