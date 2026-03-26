/** Parse a cookie header into a record of cookie values. */
export function parseCookies(header: string) {
    const cookies = {};

    // split the cookie header into parts
    for (const part of header.split(";")) {
        const segments = part.split("=");
        const key = segments[0];

        // skip malformed cookie parts
        if (key === undefined) {
            continue;
        }

        const value = segments.slice(1).join("=").trim();
        cookies[key.trim()] = value;
    }

    return cookies;
}

/** Check whether a parsed cookie record contains a named cookie. */
export function hasCookie(
    cookies: Record<string, string>,
    name: string,
): boolean {
    return cookies[name] !== undefined;
}
