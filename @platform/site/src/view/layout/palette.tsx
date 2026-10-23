import { Icon } from "@destack/icon";
import caretRight from "@destack/icon/phosphor/caret-right";
import check from "@destack/icon/phosphor/check";
import magnifyingGlass from "@destack/icon/phosphor/magnifying-glass";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, font, radius, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { Button } from "@destack/ui/button";
import {
    CommandDialog,
    CommandEmpty,
    CommandGroup,
    CommandInput,
    CommandItem,
    CommandList,
    CommandLoading,
} from "@destack/ui/command";
import { present, schema } from "@destack/schema";
import { Kbd } from "@destack/ui/kbd";
import { createMemo, createSignal, For, onSettled, Show } from "@destack/view";
import type { LocalAppearance } from "@destack/view/document";
import { Accelerator, isCommandPlatform } from "@destack/view/palette";
import { createShortcut } from "@destack/view/primitives/keyboard";
import { useNavigate } from "@destack/view/router";

import {
    type PaletteCommand,
    commandEvents,
    type CommandMatch,
    commandsFor,
    highlightSegments,
    matchCommands,
    type Menu,
    menuTitles,
    type PageSection,
    type Site,
} from "./command";
import { loadSearchEntries, type SearchEntry } from "../content/search";
import { siteSearchEntries } from "../content/site";
import type { PageFormats } from "../content/source";
import { createDownloads } from "../install/download";
import { createEventListener } from "@destack/view/primitives/event-listener";
import { typeScale, tracking } from "./type.stylex";

/** The most results a search shows at once. */
const RESULT_LIMIT = 10;

/** The most recently opened pages the empty palette lists. */
const RECENT_LIMIT = 4;

/** The browser storage key of the recently opened pages, a per-visitor convenience. */
const RECENT_KEY = "destack-palette-recent";

/** The milliseconds a copied command shows its check before the palette closes. */
const COPIED_TIME = 700;

/** Search the site's pages, sections and commands from anywhere, on ⌘K, Ctrl+K or a slash. */
export function CommandPalette(properties: { appearance: LocalAppearance }) {
    // hold whether the palette is open, its menu, the search and the loaded entries
    const navigate = useNavigate();
    const { selected: download } = createDownloads();
    const [isOpen, setIsOpen] = createSignal(false);
    const [menu, setMenu] = createSignal<Menu>("root");
    const [query, setQuery] = createSignal("");
    const [entries, setEntries] = createSignal<readonly SearchEntry[]>([]);
    const [isLoading, setIsLoading] = createSignal(false);
    const [failure, setFailure] = createSignal<string>();
    const [page, setPage] = createSignal<Pick<Site, "source" | "sections" | "hasStack">>({
        source: undefined,
        sections: [],
        hasStack: false,
    });
    const [recent, setRecent] = createSignal<readonly string[]>([]);
    const [copied, setCopied] = createSignal<string>();
    const [isMac, setIsMac] = createSignal(false);

    // build the page's commands, and list what the menu and search show
    const commands = createMemo(() =>
        commandsFor({
            entries: [...siteSearchEntries, ...entries()],
            download: download(),
            ...page(),
        }),
    );
    const results = createMemo(() => matchCommands(commands(), menu(), query(), RESULT_LIMIT));

    // group the results, the recent pages first on the empty root
    const groups = createMemo(() => {
        // collect the groups in the order they first show
        const grouped = new Map<string, CommandMatch[]>();

        // lead the empty root with the recently opened pages
        if (menu() === "root" && query().trim() === "") {
            const opened = recent().flatMap((id) =>
                commands().filter((command) => command.id === id),
            );
            grouped.set(
                "Recent",
                opened.map((command) => ({ command, excerpt: "", terms: [] })),
            );
        }

        // add each result to its group
        for (const match of results()) {
            grouped.set(match.command.group, [...(grouped.get(match.command.group) ?? []), match]);
        }

        return [...grouped].filter(([, matches]) => matches.length > 0);
    });

    // open on the root, reading the page, and load the search index once
    const open = async (): Promise<void> => {
        // clear the search and read the page under the palette
        setQuery("");
        setMenu("root");
        setCopied(undefined);
        setPage({
            source: currentPageSource(),
            sections: currentSections(),
            hasStack: document.querySelector("[data-figure-label]") !== null,
        });
        setRecent(readRecent());
        setIsOpen(true);

        // load the index once, leaving a load in flight or done alone
        if (entries().length > 0 || isLoading()) {
            return;
        }
        setIsLoading(true);
        setFailure(undefined);
        try {
            setEntries(await loadSearchEntries());
        } catch (error: unknown) {
            setFailure(error instanceof Error ? error.message : String(error));
        } finally {
            setIsLoading(false);
        }
    };

    // open on the key combinations and on the event other parts of the site dispatch
    const shortcut = { preventDefault: true, ignoreWithinInputs: true, anyOrder: true };
    createShortcut(Accelerator.keys("mod+k", isCommandPlatform()), () => void open(), shortcut);
    createShortcut(Accelerator.keys("/", isCommandPlatform()), () => void open(), shortcut);
    createEventListener(
        () => document,
        commandEvents.open,
        () => void open(),
    );

    // read the platform for shortcut labels
    onSettled(() => {
        setIsMac(isCommandPlatform());
    });

    // enter a submenu with a fresh search, or go back to the root
    const enter = (next: Menu): void => {
        setMenu(next);
        setQuery("");
    };

    // run the chosen command: open its menu, copy, set the theme, dispatch or follow it
    const choose = async (command: PaletteCommand): Promise<void> => {
        const { action } = command;
        if (action.kind === "menu") {
            enter(action.menu);
        } else if (action.kind === "copy") {
            await navigator.clipboard.writeText(action.text);
            setCopied(command.id);
            setTimeout(() => setIsOpen(false), COPIED_TIME);
        } else if (action.kind === "appearance") {
            properties.appearance.setAppearance(action.appearance);
            setIsOpen(false);
        } else if (action.kind === "dispatch") {
            setIsOpen(false);
            document.dispatchEvent(new CustomEvent(action.event));
        } else {
            setIsOpen(false);
            setRecent(writeRecent(command, recent()));
            follow(action.href);
        }
    };

    // follow a destination: a section in place, a site route, or another site in a new tab
    const follow = (href: string): void => {
        if (href.startsWith("#")) {
            present(document.getElementById(href.slice(1)), `section ${href}`).scrollIntoView({
                behavior: "smooth",
            });
        } else if (href.startsWith("/")) {
            navigate(href);
        } else {
            window.open(href, "_blank", "noopener,noreferrer");
        }
    };

    return (
        <>
            <Button variant="ghost" size="icon" aria-label="Search" onClick={() => void open()}>
                <Icon icon={magnifyingGlass} xstyle={styles.icon} />
            </Button>

            <CommandDialog
                open={isOpen()}
                onOpenChange={setIsOpen}
                title="Search Destack"
                description="Search pages, sections and commands"
                search={query()}
                onSearchChange={setQuery}
                shouldFilter={false}
                loop
                onKeyDown={(event) => {
                    // go back to the root on Backspace in an empty submenu search
                    if (event.key === "Backspace" && query() === "" && menu() !== "root") {
                        event.preventDefault();
                        enter("root");
                    }
                }}
            >
                {/* name the submenu above its search */}
                <Show when={menu() !== "root" ? menu() : undefined}>
                    {(submenu) => (
                        <span {...style.attrs(styles.crumbs)}>
                            <button
                                type="button"
                                onClick={() => enter("root")}
                                {...style.attrs(styles.crumb)}
                            >
                                {menuTitles.root}
                            </button>
                            <Icon icon={caretRight} xstyle={styles.crumbCaret} />
                            <span {...style.attrs(styles.crumb, styles.crumbOn)}>
                                {menuTitles[submenu()]}
                            </span>
                        </span>
                    )}
                </Show>
                <CommandInput
                    placeholder={
                        menu() === "root"
                            ? "What do you need?"
                            : `Search ${menuTitles[menu()].toLowerCase()}…`
                    }
                    xstyle={styles.input}
                />

                {/* list the results */}
                <CommandList xstyle={styles.list}>
                    {/* show the index loading */}
                    <Show when={isLoading() && query() !== ""}>
                        <CommandLoading>Loading search…</CommandLoading>
                    </Show>

                    {/* show the index failing to load */}
                    <Show when={failure()}>
                        {(message) => (
                            <p role="alert" {...style.attrs(styles.failure)}>
                                {message()}
                            </p>
                        )}
                    </Show>

                    {/* say what matched nothing, else list the groups */}
                    <CommandEmpty>No results for “{query()}”</CommandEmpty>
                    <For each={groups()}>
                        {([heading, matches]) => (
                            <CommandGroup
                                heading={<span {...style.attrs(styles.heading)}>{heading}</span>}
                            >
                                <For each={matches}>
                                    {(match) => (
                                        <Row
                                            match={match}
                                            isMac={isMac()}
                                            isCopied={copied() === match.command.id}
                                            isCurrent={
                                                match.command.action.kind === "appearance" &&
                                                match.command.action.appearance ===
                                                    properties.appearance.appearance()
                                            }
                                            onChoose={(command) => void choose(command)}
                                        />
                                    )}
                                </For>
                            </CommandGroup>
                        )}
                    </For>
                </CommandList>

                {/* show the keys the palette answers to */}
                <footer {...style.attrs(text.caption, styles.footer)}>
                    <span {...style.attrs(styles.hint)}>
                        <Kbd>↑</Kbd>
                        <Kbd>↓</Kbd>
                        Navigate
                    </span>
                    <span {...style.attrs(styles.hint)}>
                        <Kbd>↵</Kbd>
                        Open
                    </span>
                    <Show when={menu() !== "root"}>
                        <span {...style.attrs(styles.hint)}>
                            <Kbd>⌫</Kbd>
                            Back
                        </span>
                    </Show>
                    <span {...style.attrs(styles.hint, styles.hintEnd)}>
                        <Kbd>Esc</Kbd>
                        Close
                    </span>
                </footer>
            </CommandDialog>
        </>
    );
}

/** Draw one row: its icon, its label and quiet line, and its key, menu caret or check. */
function Row(properties: {
    match: CommandMatch;
    isMac: boolean;
    isCopied: boolean;
    isCurrent: boolean;
    onChoose: (command: PaletteCommand) => void;
}) {
    // read the command, and show a content match's excerpt in place of its route
    const command = () => properties.match.command;
    const isContent = () => command().kind === "page" || command().kind === "section";
    const detail = () =>
        isContent() && properties.match.excerpt !== ""
            ? properties.match.excerpt
            : command().detail;

    return (
        <CommandItem
            value={command().id}
            onSelect={() => properties.onChoose(command())}
            xstyle={styles.row}
        >
            <Icon icon={command().icon} xstyle={styles.kind} />
            <span {...style.attrs(styles.body)}>
                <span {...style.attrs(text.callout, styles.label)}>
                    <Highlight terms={properties.match.terms} text={command().label} />
                </span>
                <Show when={detail() !== ""}>
                    <Detail
                        terms={properties.match.terms}
                        text={detail()}
                        isCode={!(isContent() && properties.match.excerpt !== "")}
                    />
                </Show>
            </span>
            <span {...style.attrs(styles.end)}>
                {properties.isCopied ? (
                    <span {...style.attrs(text.caption, styles.copied)}>
                        <Icon icon={check} xstyle={styles.endIcon} />
                        Copied
                    </span>
                ) : properties.isCurrent ? (
                    <Icon icon={check} xstyle={styles.endIcon} />
                ) : command().action.kind === "menu" ? (
                    <Icon icon={caretRight} xstyle={styles.endIcon} />
                ) : command().keybinding === undefined ? undefined : (
                    <Kbd>{Accelerator.format(command().keybinding ?? "", properties.isMac)}</Kbd>
                )}
            </span>
        </CommandItem>
    );
}

/** Draw a row's quiet line: a route or version in code, or an excerpt in prose. */
function Detail(properties: { terms: readonly string[]; text: string; isCode: boolean }) {
    return (
        <span {...style.attrs(text.caption, styles.detail, properties.isCode && styles.detailCode)}>
            <Highlight terms={properties.terms} text={properties.text} />
        </span>
    );
}

/** Render a text with the searched terms marked, keeping its case. */
function Highlight(properties: { terms: readonly string[]; text: string }) {
    return (
        <For each={highlightSegments(properties.text, properties.terms)}>
            {(segment) =>
                segment.isMatch ? (
                    <mark {...style.attrs(styles.mark)}>{segment.text}</mark>
                ) : (
                    segment.text
                )
            }
        </For>
    );
}

/** Read the current page's formats, which the reader advertises on its article. */
function currentPageSource(): PageFormats | undefined {
    // find the advertised page source, none on pages without one
    const element = document.querySelector<HTMLElement>("[data-page-source]");
    if (element === null) {
        return undefined;
    }

    // fail when the advertised routes are incomplete
    const markdownRoute = element.dataset["markdownRoute"];
    const textRoute = element.dataset["textRoute"];
    if (markdownRoute === undefined || textRoute === undefined) {
        throw new Error("page source routes are missing");
    }

    return { markdownRoute, textRoute };
}

/** Read the current page's entries: each anchor, its part of speech, and its heading. */
function currentSections(): readonly PageSection[] {
    return [...document.querySelectorAll<HTMLElement>("header[id]:has(> h1, > h2)")].map(
        (header) => ({
            id: header.id,
            partOfSpeech: present(header.querySelector("p i")?.textContent, "entry part of speech"),
            heading: present(header.querySelector("h1, h2")?.textContent, "entry heading"),
        }),
    );
}

/** Read the recently opened commands, none where storage is empty or refused. */
function readRecent(): readonly string[] {
    try {
        const stored: unknown = JSON.parse(localStorage.getItem(RECENT_KEY) ?? "[]");

        return schema.array(schema.string()).parse(stored);
    } catch {
        return [];
    }
}

/** Put a followed page first among the recent ones, and store them where storage allows. */
function writeRecent(command: PaletteCommand, recent: readonly string[]): readonly string[] {
    // keep pages and sections only, sections of this page drop out on others
    const next =
        command.kind === "page" || command.kind === "section"
            ? [command.id, ...recent.filter((id) => id !== command.id)].slice(0, RECENT_LIMIT)
            : recent;
    try {
        localStorage.setItem(RECENT_KEY, JSON.stringify(next));
    } catch {
        // keep the list for this visit where storage refuses
    }

    return next;
}

/** The palette styles. */
const styles = style.create({
    icon: {
        width: "20px",
        height: "20px",
    },

    // search
    crumbs: {
        alignItems: "center",
        display: "flex",
        gap: space[1],
        paddingBlockStart: space[3],
        paddingInline: space[4],
    },
    crumb: {
        backgroundColor: color.muted,
        borderRadius: radius[1],
        borderWidth: 0,
        color: color.mutedForeground,
        cursor: "pointer",
        fontFamily: font.code,
        fontSize: typeScale.label,
        letterSpacing: tracking.label,
        paddingBlock: "0.125rem",
        paddingInline: space[2],
        textTransform: "uppercase",
    },
    crumbOn: {
        color: color.foreground,
        cursor: "default",
    },
    crumbCaret: {
        color: color.mutedForeground,
        height: "0.75rem",
        width: "0.75rem",
    },
    input: {
        fontSize: typeScale.lead,
    },

    // results
    list: {
        paddingBlock: space[2],
        paddingInline: space[2],
    },
    heading: {
        color: color.mutedForeground,
        display: "block",
        fontFamily: font.code,
        fontSize: typeScale.label,
        fontWeight: weight.regular,
        letterSpacing: tracking.label,
        paddingBlockStart: space[2],
        textTransform: "uppercase",
    },
    row: {
        backgroundColor: { default: "transparent", ":is([data-highlighted])": color.muted },
        color: color.foreground,
        columnGap: space[3],
        minHeight: "2.75rem",
        paddingBlock: space[2],
        paddingInline: space[3],
    },
    kind: {
        color: color.mutedForeground,
        flexShrink: 0,
        height: "1.125rem",
        width: "1.125rem",
    },
    body: {
        display: "grid",
        flexGrow: 1,
        minWidth: 0,
    },
    label: {
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    detail: {
        color: color.mutedForeground,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    detailCode: {
        fontFamily: font.code,
    },
    mark: {
        backgroundColor: "transparent",
        color: color.primary,
        fontWeight: weight.medium,
    },
    end: {
        alignItems: "center",
        color: color.mutedForeground,
        display: "flex",
        flexShrink: 0,
        marginInlineStart: "auto",
    },
    endIcon: {
        height: "1rem",
        width: "1rem",
    },
    copied: {
        alignItems: "center",
        color: color.primary,
        display: "flex",
        gap: space[1],
    },
    failure: {
        color: color.destructive,
        padding: space[3],
    },

    // keys
    footer: {
        borderTopColor: color.border,
        borderTopStyle: "solid",
        borderTopWidth: stroke.border,
        color: color.mutedForeground,
        display: "flex",
        gap: space[4],
        paddingBlock: space[2],
        paddingInline: space[4],
    },
    hint: {
        alignItems: "center",
        display: "flex",
        gap: space[1],
    },
    hintEnd: {
        marginInlineStart: "auto",
    },
});
