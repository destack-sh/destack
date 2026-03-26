import { hasCookie, parseCookies } from "./cookies";
import { matchesRedirectUri, normalizeUri } from "./redirect-uri";
import { readRequestBody } from "./request-body";

export {
    hasCookie,
    matchesRedirectUri,
    normalizeUri,
    parseCookies,
    readRequestBody,
};

/** Read a named cookie value from a request header. */
export function readRequestCookie(header: string, name: string) {
    // parse the incoming cookie header
    const cookies = parseCookies(header);

    return cookies[name] ?? "";
}

/** Read a normalized OAuth request snapshot from request inputs. */
export function readOauthRequest(
    header: string,
    body: string | string[],
    redirectUri: string,
) {
    // parse the request parts
    const cookies = parseCookies(header);
    const requestBody = readRequestBody(body);
    const normalizedRedirectUri = normalizeUri(redirectUri);

    // return the normalized request state
    return {
        cookies,
        requestBody,
        normalizedRedirectUri,
    };
}

/** Resolve whether an OAuth redirect is registered. */
export function resolveRedirectMatch(incoming: string, registered: string[]) {
    // normalize the incoming redirect
    const normalizedIncoming = normalizeUri(incoming);

    return {
        normalizedIncoming,
        isRegisteredRedirect: matchesRedirectUri(incoming, registered),
    };
}
