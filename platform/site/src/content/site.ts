import { homeExamples } from "./home";
import type { SearchEntry } from "./search";

/// Searchable content outside the generated documentation and blog collections.
export const siteSearchEntries: readonly SearchEntry[] = [
    {
        context: "destack.sh",
        route: "/",
        text: `Destack the absurdly integrated open computing stack ${homeExamples
            .map((example) => `${example.action} ${example.description}`)
            .join(" ")}`,
        title: "Home",
    },
    ...homeExamples.map((example) => ({
        context: "destack.sh / actions",
        route: `/#${example.action}`,
        text: [example.claim, example.description, ...example.like].join(" "),
        title: example.action,
    })),
];
