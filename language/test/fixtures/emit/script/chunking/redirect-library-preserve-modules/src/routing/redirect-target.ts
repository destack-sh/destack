import { getRedirectHostname, getRedirectPathname } from "./redirect-uri.ts";

/** Build the redirect target summary for one redirect URI. */
export function summarizeRedirectTarget(uri: string): string {
    const hostname = getRedirectHostname(uri);
    const pathname = getRedirectPathname(uri);

    return `${hostname}${pathname}`;
}
