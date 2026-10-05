import type { SearchEntry } from "./search.ts";

/** The one sentence that says what Destack is, shared by search and link previews. */
export const tagline =
    "Take back your software: run every app on one open, standardised stack, on your machines or ours.";

/** The public installation command. */
export const installCommand = "curl -fsSL https://destack.sh/install | sh";

/** The prompt that has a coding agent install Destack. */
export const agentPrompt = "Set up Destack for me by following https://destack.sh/docs/setup.md";

/** Searchable content outside the generated documentation and blog collections. */
export const siteSearchEntries: readonly SearchEntry[] = [
    {
        context: "destack.sh",
        kind: "page",
        route: "/",
        text: tagline,
        title: "Home",
    },
];
