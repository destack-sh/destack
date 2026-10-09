import { Icon } from "@destack/icon";
import arrowSquareOut from "@destack/icon/phosphor/arrow-square-out";
import fileText from "@destack/icon/phosphor/file-text";
import magnifyingGlass from "@destack/icon/phosphor/magnifying-glass";
import terminal from "@destack/icon/phosphor/terminal";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { Button } from "@destack/ui/button";
import {
    CommandDialog,
    CommandEmpty,
    CommandInput,
    CommandItem,
    CommandList,
    CommandLoading,
    CommandShortcut,
} from "@destack/ui/command";
import { Kbd, KbdGroup } from "@destack/ui/kbd";
import { ToggleGroup, ToggleGroupItem } from "@destack/ui/toggle-group";
import { createMemo, createSignal, For, onSettled, Show } from "@destack/view";
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
    type SearchScope,
    searchScopes,
} from "./command";
import { loadSearchEntries, type SearchEntry } from "../content/search";
import { siteSearchEntries } from "../content/site";
import type { PageFormats } from "../content/source";
import { createEventListener } from "@destack/view/primitives/event-listener";

/** The most results the palette shows at once. */
const RESULT_LIMIT = 8;

/** Search the site's pages, sections and commands from anywhere, on ⌘K, Ctrl+K or a slash. */
export function CommandPalette() {
    // hold whether the palette is open, the loaded entries and the search
    const navigate = useNavigate();
    const [isOpen, setIsOpen] = createSignal(false);
    const [entries, setEntries] = createSignal<readonly SearchEntry[]>([]);
    const [isLoading, setIsLoading] = createSignal(false);
    const [failure, setFailure] = createSignal<string>();
    const [source, setSource] = createSignal<PageFormats>();
    const [query, setQuery] = createSignal("");
    const [scope, setScope] = createSignal<SearchScope>("All");
    const [isMac, setIsMac] = createSignal(false);
    const commands = createMemo(() => commandsFor([...siteSearchEntries, ...entries()], source()));
    const results = createMemo(() => matchCommands(commands(), query(), RESULT_LIMIT, scope()));

    // load the search index the first time somebody opens the palette, retrying after a failure
    const open = async (): Promise<void> => {
        // clear the search and show the palette over the current page
        setQuery("");
        setSource(currentPageSource());
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

    // close the palette and run the chosen command
    const choose = (command: PaletteCommand): void => {
        setIsOpen(false);
        if (command.action.kind === "dispatch") {
            document.dispatchEvent(new CustomEvent(command.action.event));
        } else if (command.action.href.startsWith("/")) {
            navigate(command.action.href);
        } else {
            window.open(command.action.href, "_blank", "noopener,noreferrer");
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
            >
                <CommandInput
                    placeholder={
                        scope() === "All" ? "Search Destack…" : `Search ${scope().toLowerCase()}…`
                    }
                />

                {/* keep the searched collection visible while typing */}
                <ToggleGroup
                    aria-label="Search scope"
                    value={scope()}
                    onValueChange={(value) => {
                        const chosen = searchScopes.find((name) => name === value);
                        setScope(chosen ?? "All");
                    }}
                    size="sm"
                    xstyle={styles.scopes}
                >
                    <For each={searchScopes}>
                        {(name) => <ToggleGroupItem value={name}>{name}</ToggleGroupItem>}
                    </For>
                </ToggleGroup>

                <CommandList>
                    <Show when={isLoading()}>
                        <CommandLoading>Loading search…</CommandLoading>
                    </Show>
                    <Show when={failure()}>
                        {(message) => (
                            <p role="alert" {...style.attrs(styles.failure)}>
                                {message()}
                            </p>
                        )}
                    </Show>
                    <CommandEmpty>No results found.</CommandEmpty>
                    <For each={results()}>
                        {(match) => (
                            <CommandItem
                                value={match.command.id}
                                onSelect={() => choose(match.command)}
                                xstyle={styles.item}
                            >
                                <Icon icon={iconOf(match.command)} xstyle={styles.kind} />
                                <span {...style.attrs(styles.result)}>
                                    <strong {...style.attrs(text.body, styles.label)}>
                                        <Highlight match={match} text={match.command.label} />
                                    </strong>
                                    <span {...style.attrs(text.footnote, styles.context)}>
                                        {match.command.context}
                                    </span>
                                    <Show when={match.excerpt !== ""}>
                                        <span {...style.attrs(text.subheadline, styles.excerpt)}>
                                            <Highlight match={match} text={match.excerpt} />
                                        </span>
                                    </Show>
                                </span>
                                <Show when={match.command.keybinding}>
                                    {(keybinding) => (
                                        <CommandShortcut>
                                            {Accelerator.format(keybinding(), isMac())}
                                        </CommandShortcut>
                                    )}
                                </Show>
                            </CommandItem>
                        )}
                    </For>
                </CommandList>

                {/* show the keys the palette answers to */}
                <footer {...style.attrs(text.footnote, styles.footer)}>
                    <span {...style.attrs(styles.hint)}>
                        <KbdGroup>
                            <Kbd>↑</Kbd>
                            <Kbd>↓</Kbd>
                        </KbdGroup>
                        Navigate
                    </span>
                    <span {...style.attrs(styles.hint)}>
                        <Kbd>↵</Kbd>
                        Open
                    </span>
                    <span {...style.attrs(styles.hint)}>
                        <Kbd>Esc</Kbd>
                        Close
                    </span>
                </footer>
            </CommandDialog>
        </>
    );
}

/** Pick a command's icon: a page or section, an action, or a destination off the site. */
function iconOf(command: PaletteCommand) {
    if (command.kind === "action") {
        return terminal;
    } else if (command.action.kind === "navigate" && !command.action.href.startsWith("/")) {
        return arrowSquareOut;
    }

    return fileText;
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

/** Render a result text with the searched terms marked, keeping its case. */
function Highlight(properties: { match: CommandMatch; text: string }) {
    return (
        <For each={highlightSegments(properties.text, properties.match.terms)}>
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

/** The palette styles. */
const styles = style.create({
    icon: {
        width: "20px",
        height: "20px",
    },
    scopes: {
        paddingInline: space[3],
        paddingBlock: space[2],
        borderBottomWidth: stroke.border,
        borderBottomStyle: "solid",
        borderBottomColor: color.border,
        flexWrap: "wrap",
    },
    item: {
        alignItems: "start",
    },
    kind: {
        width: "1rem",
        height: "1rem",
        flexShrink: 0,
        marginBlockStart: "0.25rem",
        color: color.mutedForeground,
    },
    result: {
        display: "grid",
        flexGrow: 1,
        minWidth: 0,
    },
    label: {
        fontWeight: weight.semibold,
    },
    context: {
        color: color.mutedForeground,
    },
    excerpt: {
        color: color.mutedForeground,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    mark: {
        backgroundColor: "transparent",
        color: color.primary,
    },
    footer: {
        display: "flex",
        gap: space[4],
        paddingBlock: space[2],
        paddingInline: space[3],
        borderTopWidth: stroke.border,
        borderTopStyle: "solid",
        borderTopColor: color.border,
        color: color.mutedForeground,
    },
    hint: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
    },
    failure: {
        color: color.destructive,
        padding: space[3],
    },
});
