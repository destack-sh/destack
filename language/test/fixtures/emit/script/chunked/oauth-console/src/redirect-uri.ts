/** Normalize a redirect URI for redirect matching. */
export function normalizeUri(uri: string): string {
    const trimmedUri = uri.split("?")[0];

    return trimmedUri.replace(/\/+$/, "");
}

/** Check whether an incoming redirect matches any registered redirect URI. */
export function matchesRedirectUri(incoming: string, registered: string[]) {
    // normalize the incoming redirect
    const normalizedIncoming = normalizeUri(incoming);
    let index = 0;

    // scan the registered redirect list
    while (index < registered.length) {
        const registeredUri = registered[index];

        // return once a redirect matches
        if (normalizeUri(registeredUri) === normalizedIncoming) {
            return true;
        }

        index = index + 1;
    }

    return false;
}
