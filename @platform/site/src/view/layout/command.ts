import type { IconBodies } from "@destack/icon";
import arrowsLeftRight from "@destack/icon/phosphor/arrows-left-right";
import circleHalf from "@destack/icon/phosphor/circle-half";
import copy from "@destack/icon/phosphor/copy";
import fileCode from "@destack/icon/phosphor/file-code";
import fileText from "@destack/icon/phosphor/file-text";
import hash from "@destack/icon/phosphor/hash";
import { collections, collectionAt } from "../../../content";
import type { SearchEntry, SearchEntryKind } from "../content/search";
import type { PageFormats } from "../content/source";
import type { Accelerator } from "@destack/view/palette";
import { destinations, isExternal } from "./navigation";

/** The collections available in site search. */
export const searchScopes = [
    "All",
    ...collections.map((collection) => collection.title),
    "Commands",
];

/** One selected search collection. */
export type SearchScope = (typeof searchScopes)[number];

/** The global DOM events dispatched by site commands. */
export const commandEvents = {
    copyMarkdown: "destack:copy-md",
    copyText: "destack:copy-txt",
    open: "destack:search",
    switchStack: "destack:switch-stack",
    toggleAppearance: "destack:toggle-appearance",
    openMenu: "destack:open-menu",
} as const;

/** One action selectable from the command palette. */
export type CommandAction =
    | { event: string; kind: "dispatch" }
    | { href: string; kind: "navigate" };

/** One searchable command. */
export type PaletteCommand = {
    /** The action performed when selected. */
    action: CommandAction;

    /** The collection or command category. */
    context: string;

    /** The stable command identifier. */
    id: string;

    /** The command class used for search ranking. */
    kind: "action" | "navigation" | SearchEntryKind;

    /** The primary visible label. */
    label: string;

    /** The icon beside the label. */
    icon: IconBodies;

    /** The key combination that runs the command, for commands that have one. */
    keybinding?: Accelerator;

    /** The searchable content or command description. */
    text: string;
};

/** One ranked command and its query context. */
export type CommandMatch = {
    /** The matched command. */
    command: PaletteCommand;

    /** A compact passage around the first content match. */
    excerpt: string;

    /** The normalized query terms. */
    terms: readonly string[];
};

/** One highlighted or plain search-result segment. */
export type HighlightSegment = {
    /** Whether this text matched the query. */
    isMatch: boolean;

    /** One contiguous text segment. */
    text: string;
};

/** Build the commands available to the current page, the stack's switch on pages with figures. */
export function commandsFor(
    entries: readonly SearchEntry[],
    source: PageFormats | undefined,
    hasStack: boolean,
): readonly PaletteCommand[] {
    // offer the site's own actions, switching the stack where the page has one
    const commands: PaletteCommand[] = [
        eventCommand(
            "toggle-appearance",
            "appearance",
            "Toggle light and dark",
            "theme appearance dark light mode",
            commandEvents.toggleAppearance,
            circleHalf,
        ),
    ];
    if (hasStack) {
        commands.unshift(
            eventCommand(
                "switch-stack",
                "figures",
                "Switch stacked and destacked",
                "switch stack destack toggle figures",
                commandEvents.switchStack,
                arrowsLeftRight,
            ),
        );
    }

    // offer the page source views and copies when the page has sources
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
                copy,
            ),
            eventCommand(
                "copy-txt",
                "source",
                "Copy page as plain text",
                "copy text txt",
                commandEvents.copyText,
                copy,
            ),
        );
    }

    // offer the site navigation and every content entry
    commands.push(
        ...destinations.map((destination) => {
            // distinguish site destinations from community ones off the site
            const isInternal = !isExternal(destination);

            return {
                action: { href: destination.href, kind: "navigate" as const },
                context: isInternal ? "navigation" : "social",
                id: `navigate:${destination.href}`,
                kind: "navigation" as const,
                label: destination.label,
                icon: destination.icon,
                keybinding: destination.keybinding,
                text: isInternal
                    ? `${destination.label} site navigation`
                    : `${destination.label} community social`,
            };
        }),
    );
    // offer each content entry a destination does not already open
    const opened = new Set(destinations.map((destination) => destination.href));
    commands.push(...entries.filter((entry) => !opened.has(entry.route)).map(contentCommand));

    return commands;
}

/** Construct one link-backed site command. */
function linkCommand(id: string, context: string, label: string, href: string): PaletteCommand {
    return {
        action: { href, kind: "navigate" },
        context,
        id,
        kind: "action",
        label,
        icon: fileCode,
        text: `${label} ${href}`,
    };
}

/** Rank commands that contain every query term. */
export function matchCommands(
    commands: readonly PaletteCommand[],
    query: string,
    limit: number,
    scope: SearchScope = "All",
): readonly CommandMatch[] {
    const terms = termsFor(query);
    const [first] = terms;

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
                (command.kind !== "section" && command.context !== "social"),
        )
        .map((command) => ({ command, score: scoreCommand(command, terms) }))
        .filter((result) => result.score >= 0)
        .toSorted((left, right) => right.score - left.score)
        .slice(0, limit)
        .map(({ command }) => ({
            command,
            excerpt: first === undefined ? "" : excerptFor(command.text, first),
            terms,
        }));
}

/** Classify a command by its destination or action. */
function commandScope(command: PaletteCommand): SearchScope {
    if (command.kind === "action" || command.kind === "navigation") {
        return "Commands";
    }
    if (command.action.kind !== "navigate") {
        return "Commands";
    }

    return collectionAt(command.action.href)?.title ?? "All";
}

/** Split text into matched and unmatched segments without changing its case. */
export function highlightSegments(
    text: string,
    terms: readonly string[],
): readonly HighlightSegment[] {
    if (text === "" || terms.length === 0) {
        return [{ isMatch: false, text }];
    }

    // match the longest terms first
    const normalizedTerms = [...new Set(terms.map((term) => term.toLowerCase()))]
        .filter(Boolean)
        .toSorted((left, right) => right.length - left.length);
    const lower = text.toLowerCase();
    const segments: HighlightSegment[] = [];
    let start = 0;
    let index = 0;

    // split the text at each term match
    while (index < text.length) {
        const term = termAt(normalizedTerms, lower, index);
        if (term == undefined) {
            index += 1;
            continue;
        }

        if (start < index) {
            segments.push({ isMatch: false, text: text.slice(start, index) });
        }
        segments.push({
            isMatch: true,
            text: text.slice(index, index + term.length),
        });
        index += term.length;
        start = index;
    }

    // keep the unmatched tail
    if (start < text.length) {
        segments.push({ isMatch: false, text: text.slice(start) });
    }

    return segments;
}

/** Convert one generated content entry into a navigation command. */
function contentCommand(entry: SearchEntry): PaletteCommand {
    return {
        action: { href: entry.route, kind: "navigate" },
        context: entry.context,
        id: `navigate:${entry.route}`,
        kind: entry.kind,
        label: entry.title,
        icon: entry.kind === "section" ? hash : fileText,
        text: entry.text,
    };
}

/** Construct one event-backed site command. */
function eventCommand(
    id: string,
    context: string,
    label: string,
    text: string,
    event: string,
    icon: IconBodies,
): PaletteCommand {
    return {
        action: { event, kind: "dispatch" },
        context,
        id,
        kind: "action",
        label,
        icon,
        text,
    };
}

/** Find the first term starting at one position of lowercase text. */
function termAt(terms: readonly string[], lower: string, index: number): string | undefined {
    return terms.find((term) => lower.startsWith(term, index));
}

/** Normalize a free-form query into unique terms. */
function termsFor(query: string) {
    return [...new Set(query.toLowerCase().trim().split(/\s+/u).filter(Boolean))];
}

/** Score one command against all normalized query terms. */
function scoreCommand(command: PaletteCommand, terms: readonly string[]) {
    // lowercase the fields to match against
    const label = command.label.toLowerCase();
    const context = command.context.toLowerCase();
    const text = command.text.toLowerCase();
    // use category order only to break comparable text matches
    let score = terms.length === 0 ? rankFor(command.kind) : rankFor(command.kind) / 100;

    // reject a command missing any term, and score the best field each term matches
    for (const term of terms) {
        if (!label.includes(term) && !context.includes(term) && !text.includes(term)) {
            return -1;
        }

        if (label === term) {
            score += 1000;
        } else if (label.startsWith(term)) {
            score += 500;
        } else if (isWordStart(label, term)) {
            score += 400;
        } else if (label.includes(term)) {
            score += 250;
        } else if (context.includes(term)) {
            score += 100;
        } else if (isWordStart(text, term)) {
            score += 10;
        } else {
            score += 2;
        }
    }

    return score;
}

/** Return the base rank for one command class. */
function rankFor(kind: PaletteCommand["kind"]) {
    switch (kind) {
        case "action":
        case "navigation":
            return 1600;
        case "page":
            return 1200;
        case "section":
            return 800;
    }
}

/** Report whether a term starts a word of the text. */
function isWordStart(text: string, term: string) {
    return text.startsWith(term) || text.includes(` ${term}`);
}

/** Return the group a command shows under: commands, destinations, community, or its collection. */
export function groupOf(command: PaletteCommand): string {
    if (command.kind === "action") {
        return "Commands";
    } else if (command.context === "navigation") {
        return "Go to";
    } else if (command.context === "social") {
        return "Community";
    }

    return command.action.kind === "navigate"
        ? (collectionAt(command.action.href)?.title ?? "Pages")
        : "Pages";
}

/** Extract a compact passage around one matching term, cut at word boundaries. */
function excerptFor(text: string, term: string) {
    // find the first match that starts a word, or else the first match
    const normalized = text.replace(/\s+/gu, " ").trim();
    const lower = normalized.toLowerCase();
    const wordMatch = lower.indexOf(` ${term}`);
    const match = lower.startsWith(term) ? 0 : wordMatch >= 0 ? wordMatch + 1 : lower.indexOf(term);

    // widen the cut to the nearest spaces, so no word is broken
    const from = Math.max(0, match - 42);
    const start = from === 0 ? 0 : lower.indexOf(" ", from) + 1;
    const to = Math.min(normalized.length, start + 132);
    const end = to === normalized.length ? to : Math.max(start, normalized.lastIndexOf(" ", to));
    const prefix = start === 0 ? "" : "…";
    const suffix = end === normalized.length ? "" : "…";

    return `${prefix}${normalized.slice(start, end)}${suffix}`;
}
