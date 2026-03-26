export function normalizeUri(uri) {
    const trimmedUri = uri.split("?")[0]
    return trimmedUri.replace(/\/+$/, "")
}
export function parseCookies(header) {
    const cookies = { }
    for (const part of header.split(";")) {
        const segments = part.split("=")
        const key = segments[0]
        if (key === undefined) {
            continue
        }
        const value = segments.slice(1).join("=").trim()
        cookies[key.trim()] = value
    }
    return cookies
}
export function matchesRedirectUri(incoming, registered) {
    const normalizedIncoming = normalizeUri(incoming)
    let index = 0
    while (index < registered.length) {
        const registeredUri = registered[index]
        if (normalizeUri(registeredUri) === normalizedIncoming) {
            return true
        }
        index = index + 1
    }
    return false
}
export function readRequestBody(value) {
    if (typeof value === "string") {
        return value
    }
    const bodyValue = value[0]
    if (bodyValue === undefined) {
        return ""
    }
    return bodyValue
}
export function readRequestCookie(header, name) {
    const cookies = parseCookies(header)
    return cookies[name] ?? ""
}
export function readOauthRequest(header, body, redirectUri) {
    const cookies = parseCookies(header)
    const requestBody = readRequestBody(body)
    const normalizedRedirectUri = normalizeUri(redirectUri)
    return {
        cookies,
        requestBody,
        normalizedRedirectUri,
    }
}
export function resolveRedirectMatch(incoming, registered) {
    const normalizedIncoming = normalizeUri(incoming)
    return {
        normalizedIncoming,
        isRegisteredRedirect: matchesRedirectUri(incoming, registered),
    }
}
//# sourceMappingURL=./oauth-helpers.js.map
