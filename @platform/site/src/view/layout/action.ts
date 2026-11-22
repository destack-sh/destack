import type { IconBodies } from "@destack/icon";
import arrowsLeftRight from "@destack/icon/phosphor/arrows-left-right";
import circleHalf from "@destack/icon/phosphor/circle-half";
import compass from "@destack/icon/phosphor/compass";
import copy from "@destack/icon/phosphor/copy";
import downloadSimple from "@destack/icon/phosphor/download-simple";
import fileCode from "@destack/icon/phosphor/file-code";
import fileText from "@destack/icon/phosphor/file-text";
import hash from "@destack/icon/phosphor/hash";
import monitor from "@destack/icon/phosphor/monitor";
import moon from "@destack/icon/phosphor/moon";
import robot from "@destack/icon/phosphor/robot";
import sun from "@destack/icon/phosphor/sun";
import terminalWindow from "@destack/icon/phosphor/terminal-window";
import type { Appearance } from "@destack/theme";
import type { Shortcut } from "@destack/locale";

import { collectionAt } from "../../../content";
import type { SearchEntry, SearchEntryKind } from "../content/search";
import type { PageFormats } from "../content/source";
import { agentPrompt, installCommand } from "../content/site";
import type { Download } from "../install/catalog";
import { destinations, isExternal } from "./navigation";

/** The global DOM events dispatched by site actions. */
export const actionEvents = {
    copyMarkdown: "destack:copy-md",
    copyText: "destack:copy-txt",
    open: "destack:search",
    switchStack: "destack:switch-stack",
    openMenu: "destack:open-menu",
} as const;

/** A list the palette shows: its root, or a submenu an action opens. */
export type Menu = "root" | "theme" | "documentation" | "sections";

/** The title each menu shows above its search, the root's leading back to it. */
export const menuTitles: Readonly<Record<Menu, string>> = {
    root: "Destack",
    theme: "Theme",
    documentation: "Documentation",
    sections: "Sections",
};

/** What an action does when chosen. */
export type ActionEffect =
    | { kind: "dispatch"; event: string }
    | { kind: "navigate"; href: string }
    | { kind: "copy"; text: string }
    | { kind: "appearance"; appearance: Appearance }
    | { kind: "menu"; menu: Menu };

/** One searchable action. */
export type Action = {
    /** What the action does when chosen. */
    effect: ActionEffect;

    /** The stable action identifier. */
    id: string;

    /** The action class used for search ranking. */
    kind: "action" | "navigation" | SearchEntryKind;

    /** The menu that lists the action. */
    menu: Menu;

    /** The heading of the group it shows under. */
    group: string;

    /** The primary visible label. */
    label: string;

    /** The quiet line under the label: a route, a version, or a figure. */
    detail: string;

    /** The icon beside the label. */
    icon: IconBodies;

    /** The key combination that runs the action, for actions that have one. */
    keybinding?: Shortcut;

    /** The searchable content or action description. */
    text: string;
};

/** An entry of the current page, which the sections menu jumps to. */
export type PageSection = {
    /** The entry's anchor. */
    id: string;
    /** The entry's part of speech, such as verb. */
    partOfSpeech: string;
    /** The entry's heading, its definition. */
    heading: string;
};

/** What the palette offers actions for: the site's pages, the current page, and the download. */
export type Site = {
    /** The generated pages and sections of the documentation and blog. */
    entries: readonly SearchEntry[];
    /** The current page's formats, absent on pages without a source. */
    source: PageFormats | undefined;
    /** The current page's sections, empty on pages without entries. */
    sections: readonly PageSection[];
    /** Whether the current page has figures to switch between stacked and destacked. */
    hasStack: boolean;
    /** The desktop download for this machine, absent until known. */
    download: Download | undefined;
};

/** One ranked action and its query context. */
export type ActionMatch = {
    /** The matched action. */
    action: Action;

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

/** Build every action the current page offers, across the root and its submenus. */
export function actionsFor(site: Site): readonly Action[] {
    // offer the copies and the download first
    const actions: Action[] = [
        rootAction("copy-install", "Copy install command", installCommand, terminalWindow, {
            kind: "copy",
            text: installCommand,
        }),
        rootAction("copy-prompt", "Copy agent prompt", agentPrompt, robot, {
            kind: "copy",
            text: agentPrompt,
        }),
        site.download === undefined
            ? rootAction("download", "Download Destack", "/docs/setup/", downloadSimple, {
                  kind: "navigate",
                  href: "/docs/setup/",
              })
            : rootAction(
                  "download",
                  `Download for ${site.download.label}`,
                  site.download.version,
                  downloadSimple,
                  { kind: "navigate", href: site.download.url },
              ),
    ];

    // switch the stack where the page has figures
    if (site.hasStack) {
        actions.push(
            rootAction("switch-stack", "Switch stacked and destacked", "", arrowsLeftRight, {
                kind: "dispatch",
                event: actionEvents.switchStack,
            }),
        );
    }

    // offer the page source views and copies when the page has sources
    if (site.source !== undefined) {
        actions.push(
            rootAction("source:md", "View page as Markdown", site.source.markdownRoute, fileCode, {
                kind: "navigate",
                href: site.source.markdownRoute,
            }),
            rootAction("source:txt", "View page as plain text", site.source.textRoute, fileCode, {
                kind: "navigate",
                href: site.source.textRoute,
            }),
            rootAction("copy-md", "Copy page as Markdown", "", copy, {
                kind: "dispatch",
                event: actionEvents.copyMarkdown,
            }),
            rootAction("copy-txt", "Copy page as plain text", "", copy, {
                kind: "dispatch",
                event: actionEvents.copyText,
            }),
        );
    }

    // open the theme menu, and list its appearances
    actions.push(
        menuAction("menu:theme", "Change theme…", circleHalf, "theme", "Actions"),
        ...themes.map(([appearance, label, icon]) =>
            submenuAction("theme", `theme:${appearance}`, label, "", icon, {
                kind: "appearance",
                appearance,
            }),
        ),
    );

    // offer the site destinations, the submenus, and the community off the site
    actions.push(
        ...destinations.map((destination): Action => {
            const isInternal = !isExternal(destination);

            return {
                effect: { kind: "navigate", href: destination.href },
                id: `navigate:${destination.href}`,
                kind: "navigation",
                menu: "root",
                group: isInternal ? "Go to" : "Community",
                label: destination.label,
                detail: isInternal ? "" : new URL(destination.href).host,
                icon: destination.icon,
                keybinding: destination.keybinding,
                text: `${destination.label} ${isInternal ? "site navigation" : "community social"}`,
            };
        }),
        menuAction("menu:documentation", "Documentation…", fileText, "documentation", "Go to"),
    );

    // list the current page's sections under their own menu
    if (site.sections.length > 0) {
        actions.push(
            menuAction("menu:sections", "Jump to section…", compass, "sections", "Go to"),
            ...site.sections.map(({ id, partOfSpeech, heading }) =>
                submenuAction("sections", `section:${id}`, heading, partOfSpeech, hash, {
                    kind: "navigate",
                    href: `#${id}`,
                }),
            ),
        );
    }

    // list each content entry a destination does not already open, the docs pages in their menu
    const opened = new Set(destinations.map((destination) => destination.href));

    return [
        ...actions,
        ...site.entries.filter((entry) => !opened.has(entry.route)).map(contentAction),
    ];
}

/** List what the palette shows: a menu's actions while empty, else the best matches of all. */
export function matchActions(
    actions: readonly Action[],
    menu: Menu,
    query: string,
    limit: number,
): readonly ActionMatch[] {
    // read the query's terms
    const terms = termsFor(query);
    const [first] = terms;

    // list the menu's own actions while nothing is typed, its sections and community left to search
    if (first === undefined) {
        return actions
            .filter(
                (action) =>
                    action.menu === menu &&
                    (menu !== "root" ||
                        (action.kind !== "section" && action.group !== "Community")),
            )
            .map((action) => ({ action, excerpt: "", terms }));
    }

    // rank every action the root reaches, or the submenu's own
    return actions
        .filter((action) => menu === "root" || action.menu === menu)
        .map((action) => ({ action, score: scoreAction(action, terms) }))
        .filter((result) => result.score >= 0)
        .toSorted((left, right) => right.score - left.score)
        .slice(0, limit)
        .map(({ action }) => ({ action, excerpt: excerptFor(action.text, first), terms }));
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

/** The appearances the theme menu offers: each value, its label and its icon. */
const themes: readonly (readonly [appearance: Appearance, label: string, icon: IconBodies])[] = [
    ["light", "Light", sun],
    ["dark", "Dark", moon],
    ["system", "System", monitor],
];

/** Construct one site action of the root menu. */
function rootAction(
    id: string,
    label: string,
    detail: string,
    icon: IconBodies,
    effect: ActionEffect,
): Action {
    return {
        effect,
        id,
        kind: "action",
        menu: "root",
        group: "Actions",
        label,
        detail,
        icon,
        text: label,
    };
}

/** Construct one action of the root menu that opens a submenu, under a group of the root. */
function menuAction(
    id: string,
    label: string,
    icon: IconBodies,
    menu: Menu,
    group: string,
): Action {
    return {
        effect: { kind: "menu", menu },
        id,
        kind: "navigation",
        menu: "root",
        group,
        label,
        detail: "",
        icon,
        text: label,
    };
}

/** Construct one action a submenu lists. */
function submenuAction(
    menu: Exclude<Menu, "root">,
    id: string,
    label: string,
    detail: string,
    icon: IconBodies,
    effect: ActionEffect,
): Action {
    return {
        effect,
        id,
        kind: "action",
        menu,
        group: menuTitles[menu],
        label,
        detail,
        icon,
        text: `${label} ${detail} ${menuTitles[menu]}`,
    };
}

/** Convert one generated content entry into a navigation action, docs pages in their menu. */
function contentAction(entry: SearchEntry): Action {
    const collection = collectionAt(entry.route)?.title ?? "Pages";

    return {
        effect: { kind: "navigate", href: entry.route },
        id: `navigate:${entry.route}`,
        kind: entry.kind,
        menu: collection === "Documentation" && entry.kind === "page" ? "documentation" : "root",
        group: collection,
        label: entry.title,
        detail: entry.kind === "section" ? entry.context : entry.route,
        icon: entry.kind === "section" ? hash : fileText,
        text: entry.text,
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

/** Score one action against all normalized query terms. */
function scoreAction(action: Action, terms: readonly string[]) {
    // lowercase the fields to match against
    const label = action.label.toLowerCase();
    const group = action.group.toLowerCase();
    const text = action.text.toLowerCase();
    // use category order only to break comparable text matches
    let score = rankFor(action.kind) / 100;

    // reject an action missing any term, and score the best field each term matches
    for (const term of terms) {
        if (!label.includes(term) && !group.includes(term) && !text.includes(term)) {
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
        } else if (group.includes(term)) {
            score += 100;
        } else if (isWordStart(text, term)) {
            score += 10;
        } else {
            score += 2;
        }
    }

    return score;
}

/** Return the base rank for one action class. */
function rankFor(kind: Action["kind"]) {
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
