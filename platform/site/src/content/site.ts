import type { SearchEntry } from "./search.ts";

/** The one sentence that says what Destack is, shared by the hero, search, and link previews. */
export const tagline = "Destack unifies all your apps and agents with one open stack.";

/** The public installation command. */
export const installCommand = "curl -fsSL https://destack.sh/install | sh";

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
