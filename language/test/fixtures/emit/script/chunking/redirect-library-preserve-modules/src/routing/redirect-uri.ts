/** Normalize one redirect URI for redirect matching. */
export function normalizeUri(uri: string): string {
    const trimmedUri = uri.split("?")[0];

    return trimmedUri.replace(/\/+$/, "");
}

/** Check whether an incoming redirect matches a registered redirect URI. */
export function matchesRedirectUri(
    incoming: string,
    registered: string[],
): boolean {
    const normalizedIncoming = normalizeUri(incoming);
    let index = 0;

    // scan the registered redirects
    while (index < registered.length) {
        const registeredUri = registered[index];

        if (normalizeUri(registeredUri) === normalizedIncoming) {
            return true;
        }

        index = index + 1;
    }

    return false;
}

/** Read the hostname from one redirect URI. */
export function getRedirectHostname(uri: string): string {
    const normalizedUri = normalizeUri(uri);
    const segments = normalizedUri.split("/");

    return segments[2];
}

/** Read the pathname from one redirect URI. */
export function getRedirectPathname(uri: string): string {
    const normalizedUri = normalizeUri(uri);
    const segments = normalizedUri.split("/");
    const pathname = segments.slice(3).join("/");

    return `/${pathname}`;
}
