import {
    getRedirectHostname,
    getRedirectPathname,
    matchesRedirectUri,
    normalizeUri,
} from "./routing/redirect-uri.ts";
import { summarizeRedirectTarget } from "./routing/redirect-target.ts";

/** The redirect target state for the preserve modules entry. */
export const redirectTargetState = {
    redirectUri: normalizeUri("https://destack.dev/callback///?state=1"),
    redirectMatch: matchesRedirectUri(
        "https://destack.dev/callback/?code=123",
        ["https://destack.dev/callback", "https://destack.dev/docs"],
    ),
    redirectHostname: getRedirectHostname(
        "https://destack.dev/callback/?code=123",
    ),
    redirectPathname: getRedirectPathname(
        "https://destack.dev/callback/?code=123",
    ),
    redirectSummary: summarizeRedirectTarget(
        "https://destack.dev/callback/?code=123",
    ),
};
