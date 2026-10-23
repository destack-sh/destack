import type { SearchEntry } from "./search.ts";

/** The one sentence that says what Destack is, shared by search and link previews. */
export const tagline =
    "Reconquer your software: build and run every app on one open, standardised stack, on your machines or ours.";

/** The site's origin, which absolute links and metadata start from. */
export const origin = "https://destack.sh";

/** The address that serves a coding agent the setup guide as Markdown. */
export const setupGuide = "https://destack.me";

/** The public installation command. */
export const installCommand = `curl -fsSL ${origin}/install | sh`;

/** The prompt that has a coding agent install Destack. */
export const agentPrompt = `Set up my Destack: ${setupGuide}`;

/** Searchable content outside the generated documentation and blog collections. */
export const siteSearchEntries: readonly SearchEntry[] = [
    {
        context: new URL(origin).host,
        kind: "page",
        route: "/",
        text: tagline,
        title: "Home",
    },
];
