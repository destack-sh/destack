import type { Shim } from "./shim.ts";

/** The `URL.parse` member. */
export const URL_SHIMS: readonly Shim[] = [
    {
        feature: "api.URL.parse_static",
        owner: URL,
        name: "parse",
        implementation: parse,
        native: { chrome: 126, edge: 126, firefox: 126, safari: 18 },
    },
];

/** Parse a URL against an optional base, or null where invalid, as `URL.parse` does. */
function parse(url: string | URL, base?: string | URL): URL | null {
    return URL.canParse(url, base) ? new URL(url, base) : null;
}
