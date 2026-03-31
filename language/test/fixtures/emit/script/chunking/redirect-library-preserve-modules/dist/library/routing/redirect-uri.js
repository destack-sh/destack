export function normalizeUri(uri) {
    const trimmedUri = uri.split("?")[0];
    return trimmedUri.replace(/\/+$/, "");
}
export function matchesRedirectUri(incoming, registered) {
    const normalizedIncoming = normalizeUri(incoming);
    let index = 0;
    while(index < registered.length) {
        const registeredUri = registered[index];
        if(normalizeUri(registeredUri) === normalizedIncoming) {
            return true;
        }
        index=index + 1;
    }
    return false;
}
export function getRedirectHostname(uri) {
    const normalizedUri = normalizeUri(uri);
    const segments = normalizedUri.split("/");
    return segments[2];
}
export function getRedirectPathname(uri) {
    const normalizedUri = normalizeUri(uri);
    const segments = normalizedUri.split("/");
    const pathname = segments.slice(3).join("/");
    return `/${pathname}`;
}
//# sourceMappingURL=./redirect-uri.map
