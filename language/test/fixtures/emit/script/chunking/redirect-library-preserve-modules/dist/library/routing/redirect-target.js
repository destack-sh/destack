import { getRedirectHostname, getRedirectPathname } from "./redirect-uri.js";
export function summarizeRedirectTarget(uri) {
    const hostname = getRedirectHostname(uri);
    const pathname = getRedirectPathname(uri);
    return `${hostname}${pathname}`;
}
//# sourceMappingURL=./redirect-target.map
