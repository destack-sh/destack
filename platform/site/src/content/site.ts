import type { SearchEntry } from "./search.ts";

/// The public installation command.
export const installCommand = "curl -fsSL https://destack.sh/install | sh";

/// Searchable content outside the generated documentation and blog collections.
export const siteSearchEntries: readonly SearchEntry[] = [
    {
        context: "destack.sh",
        kind: "page",
        route: "/",
        text:
            "Destack, the absurdly integrated, fully hackable, open-source computing stack. TypeScript++, Web, native, and desktop.",
        title: "Home",
    },
];
