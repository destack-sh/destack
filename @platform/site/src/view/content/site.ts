import type { SearchEntry } from "./search.ts";

/** The one sentence that says what Destack is, shared by search and link previews. */
export const tagline =
    "Reconquer your software: the apps you use, the apps you make and the agents you run, on one open stack, on your machines or ours.";

/** The site's origin, which absolute links and metadata start from. */
export const origin = "https://destack.sh";

/** The address that serves a coding agent the setup guide as Markdown. */
export const setupGuide = "https://destack.me";

/** The public installation command. */
export const installCommand = `curl -fsSL ${origin}/install | sh`;

/** Write the prompt that has a coding agent set up Destack, with any further asks. */
export function agentPromptOf(asks: readonly string[]) {
    const request =
        asks.length === 0 ? "Set up my Destack" : `Set up my Destack, ${asks.join(" and ")}`;

    return `${request}: ${setupGuide}`;
}

/** The prompt that has a coding agent set up Destack. */
export const agentPrompt = agentPromptOf([]);

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
