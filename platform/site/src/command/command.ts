import { collections, collectionAt } from "../../content";
import type { SearchEntry, SearchEntryKind } from "../content/search";
import type { PageFormats } from "../content/source";
import { navigationLinks } from "../navigation/navigation";

/// The collections available in site search.
export const searchScopes = [
    "All",
    ...collections
        .filter((collection) => collection.isListed !== false)
        .map((collection) => collection.title),
    "Commands",
];

/// One selected search collection.
export type SearchScope = (typeof searchScopes)[number];

/// The global DOM events dispatched by site commands.
export const commandEvents = {
    copyMarkdown: "destack:copy-md",
    copyText: "destack:copy-txt",
    open: "destack:search",
} as const;

/// One action selectable from the command palette.
export type CommandAction =
    | { event: string; kind: "dispatch" }
    | { href: string; kind: "navigate" };

/// One searchable command.
export type Command = {
    /// The action performed when selected.
    action: CommandAction;

    /// The collection or command category.
    context: string;

    /// The stable command identifier.
    id: string;

    /// The command class used for search ranking.
    kind: "action" | "navigation" | SearchEntryKind;

    /// The primary visible label.
    label: string;

    /// The optional global keyboard mnemonic.
    shortcut?: string;

    /// The searchable content or command description.
    text: string;
};

/// One ranked command and its query context.
export type CommandMatch = {
    /// The matched command.
    command: Command;

    /// A compact passage around the first content match.
    excerpt: string;

    /// The normalized query terms.
    terms: readonly string[];
};

/// One highlighted or plain search-result segment.
export type HighlightPart = {
    /// Whether this text matched the query.
    isMatch: boolean;

    /// One contiguous text segment.
    text: string;
};

/// Build the commands available to the current page.
export function commandsFor(
    entries: readonly SearchEntry[],
    source?: PageFormats,
): readonly Command[] {
    const commands: Command[] = [];

    if (source != undefined) {
        commands.push(
            linkCommand("source:md", "source", "View page as Markdown", source.markdownRoute),
            linkCommand("source:txt", "source", "View page as plain text", source.textRoute),
            eventCommand(
                "copy-md",
                "source",
                "Copy page as Markdown",
                "copy markdown md",
                commandEvents.copyMarkdown,
            ),
            eventCommand(
                "copy-txt",
                "source",
                "Copy page as plain text",
                "copy text txt",
                commandEvents.copyText,
            ),
        );
    }

    commands.push(
        ...navigationLinks.map((link) => {
            // distinguish local collections from off-site community destinations
            const isInternal = link.href.startsWith("/");

            return {
                action: { href: link.href, kind: "navigate" as const },
                context: isInternal ? "navigation" : "social",
                id: `navigate:${link.href}`,
                kind: "navigation" as const,
                label: link.label,
                shortcut: link.shortcut,
                text: isInternal
                    ? `${link.label} site navigation`
                    : `${link.label} community social`,
            };
        }),
    );
    commands.push(...entries.map(contentCommand));

    return commands;
}

/// Construct one link-backed site command.
function linkCommand(id: string, context: string, label: string, href: string): Command {
    return {
        action: { href, kind: "navigate" },
        context,
        id,
        kind: "action",
        label,
        text: `${label} ${href}`,
    };
}

/// Rank commands that contain every query term.
export function matchCommands(
    commands: readonly Command[],
    query: string,
    limit: number,
    scope: SearchScope = "All",
): readonly CommandMatch[] {
    const terms = termsFor(query);

    return commands
        .filter(
            (command) =>
                scope === "All" ||
                commandScope(command) === scope ||
                (scope === "Docs" &&
                    command.action.kind === "navigate" &&
                    command.action.href.startsWith("/docs/") &&
                    command.kind !== "navigation" &&
                    command.kind !== "action"),
        )
        .filter(
            (command) =>
                terms.length > 0 ||
                (scope === "Commands" && commandScope(command) === scope) ||
                command.kind === "page",
        )
        .map((command) => ({ command, score: scoreCommand(command, terms) }))
        .filter((result) => result.score >= 0)
        .sort((left, right) => right.score - left.score)
        .slice(0, limit)
        .map(({ command }) => ({
            command,
            excerpt: terms.length === 0 ? "" : excerptFor(command.text, terms[0]),
            terms,
        }));
}

/// Classify a command by its destination or action.
function commandScope(command: Command): SearchScope {
    if (command.kind === "action" || command.kind === "navigation") {
        return "Commands";
    }
    if (command.action.kind !== "navigate") {
        return "Commands";
    }

    return collectionAt(command.action.href)?.title ?? "All";
}

/// Split text into matched and unmatched segments without changing its case.
export function highlightParts(text: string, terms: readonly string[]): readonly HighlightPart[] {
    if (text === "" || terms.length === 0) {
        return [{ isMatch: false, text }];
    }

    const normalizedTerms = [...new Set(terms.map((term) => term.toLowerCase()))]
        .filter(Boolean)
        .sort((left, right) => right.length - left.length);
    const lower = text.toLowerCase();
    const parts: HighlightPart[] = [];
    let start = 0;
    let index = 0;

    while (index < text.length) {
        const term = normalizedTerms.find((candidate) => lower.startsWith(candidate, index));
        if (term == undefined) {
            index += 1;
            continue;
        }

        if (start < index) {
            parts.push({ isMatch: false, text: text.slice(start, index) });
        }
        parts.push({
            isMatch: true,
            text: text.slice(index, index + term.length),
        });
        index += term.length;
        start = index;
    }

    if (start < text.length) {
        parts.push({ isMatch: false, text: text.slice(start) });
    }

    return parts;
}

/// Convert one generated content entry into a navigation command.
function contentCommand(entry: SearchEntry): Command {
    return {
        action: { href: entry.route, kind: "navigate" },
        context: entry.context,
        id: `navigate:${entry.route}`,
        kind: entry.kind,
        label: entry.title,
        text: entry.text,
    };
}

/// Construct one event-backed site command.
function eventCommand(
    id: string,
    context: string,
    label: string,
    text: string,
    event: string,
): Command {
    return {
        action: { event, kind: "dispatch" },
        context,
        id,
        kind: "action",
        label,
        text,
    };
}

/// Normalize a free-form query into unique terms.
function termsFor(query: string) {
    return [...new Set(query.toLowerCase().trim().split(/\s+/).filter(Boolean))];
}

/// Score one command against all normalized query terms.
function scoreCommand(command: Command, terms: readonly string[]) {
    const label = command.label.toLowerCase();
    const context = command.context.toLowerCase();
    const text = command.text.toLowerCase();
    // use category order only to break comparable text matches
    let score = terms.length === 0 ? rankFor(command.kind) : rankFor(command.kind) / 100;

    for (const term of terms) {
        if (!label.includes(term) && !context.includes(term) && !text.includes(term)) {
            return -1;
        }

        // treat the final module segment as its local name
        const isExact =
            label === term ||
            (command.kind === "module" &&
                (label.endsWith(`:${term}`) || label.endsWith(`/${term}`)));

        if (isExact) {
            score += 1000;
        } else if (label.startsWith(term)) {
            score += 500;
        } else if (label.includes(term)) {
            score += 250;
        } else if (context.includes(term)) {
            score += 100;
        } else {
            score += 10;
        }
    }

    return score;
}

/// Return the base rank for one command class.
function rankFor(kind: Command["kind"]) {
    switch (kind) {
        case "action":
        case "navigation":
            return 1600;
        case "page":
            return 1200;
        case "rule":
            return 1100;
        case "section":
            return 800;
        case "module":
            return 900;
        case "symbol":
            return 0;
    }
}

/// Extract a compact passage around one matching term.
function excerptFor(text: string, term: string) {
    const normalized = text.replace(/\s+/g, " ").trim();
    const match = normalized.toLowerCase().indexOf(term);
    const start = Math.max(0, match - 42);
    const end = Math.min(normalized.length, start + 132);
    const prefix = start === 0 ? "" : "...";
    const suffix = end === normalized.length ? "" : "...";

    return `${prefix}${normalized.slice(start, end)}${suffix}`;
}
