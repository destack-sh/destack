import type { SearchEntry } from "./search.ts";

/** The one sentence that says what Destack is, shared by search and link previews. */
export const tagline =
    "Take back your software: build and run every app on one open, standardised stack, on your machines or ours.";

/** The site's origin, which absolute links and metadata start from. */
export const origin = "https://destack.sh";

/** The setup guide a coding agent follows, as Markdown. */
export const setupGuide = `${origin}/docs/setup.md`;

/** The public installation command. */
export const installCommand = `curl -fsSL ${origin}/install | sh`;

/** The prompt that has a coding agent install Destack. */
export const agentPrompt = `Set up Destack for me by following ${setupGuide}`;

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
