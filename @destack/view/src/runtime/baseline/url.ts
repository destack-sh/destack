/** Parse a URL against an optional base, or null where invalid, as `URL.parse` does. */
export function parse(url: string | URL, base?: string | URL): URL | null {
    return URL.canParse(url, base) ? new URL(url, base) : null;
}
