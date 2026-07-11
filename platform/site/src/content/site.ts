import type { SearchEntry } from "./search";

/// The public installation command.
export const installCommand = "curl -fsSL https://destack.sh/install | sh";

/// One public product action.
export type HomePoint = {
    /// The action identifier and visible verb.
    action: string;

    /// The short action description.
    description: string;
};

/// The ordered Destack product outline.
export const homePoints = [
    { action: "install", description: installCommand },
    { action: "write", description: "familiar TS / TSX, Node, and Web code" },
    { action: "use", description: "standardized libraries" },
    { action: "compile", description: "to sandboxed VM and true AOT native targets" },
    { action: "control", description: "precise access over every host binding" },
    { action: "check", description: "strong typing and userland lints" },
    { action: "test", description: "every byte and cycle of your systems" },
    { action: "simulate", description: "the entire application end-to-end" },
    { action: "debug", description: "backward, in parallel or slow motion" },
    { action: "ship", description: "... web and native (really)" },
] as const satisfies readonly HomePoint[];

/// Searchable content outside the generated documentation and blog collections.
export const siteSearchEntries: readonly SearchEntry[] = [
    {
        context: "destack.sh",
        route: "/",
        text: `Destack the absurdly integrated computing stack ${homePoints
            .map((point) => `${point.action} ${point.description}`)
            .join(" ")}`,
        title: "Home",
    },
    ...homePoints.map((point) => ({
        context: "destack.sh / actions",
        route: `/#${point.action}`,
        text: point.description,
        title: point.action,
    })),
    {
        context: "footer",
        route: "/",
        text: "copyright symbol industries",
        title: "Symbol Industries",
    },
];
