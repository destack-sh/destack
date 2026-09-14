import {
    createMemo,
    createSignal,
    For,
    onCleanup,
    onMount,
    Show,
} from "solid-js";
import { Portal } from "solid-js/web";
import * as stylex from "@stylexjs/stylex";

import {
    type Command,
    type CommandMatch,
    commandEvents,
    commandsFor,
    highlightParts,
    matchCommands,
    searchScopes,
    type SearchScope,
} from "./command";
import { loadSearchEntries, type SearchEntry } from "../content/search";
import { siteSearchEntries } from "../content/site";
import type { PageFormats } from "../content/source";
import { isExternalLink } from "../navigation/link";
import { tokens } from "../style/tokens.stylex";

/// The maximum number of visible command results.
const resultLimit = 8;

/// Render site-wide search and keyboard navigation.
export function CommandPalette() {
    let dialog: HTMLDialogElement | undefined;
    let input: HTMLInputElement | undefined;
    const [entries, setEntries] = createSignal<readonly SearchEntry[]>([]);
    const [loading, setLoading] = createSignal(false);
    const [error, setError] = createSignal<string>();
    const [source, setSource] = createSignal<PageFormats>();
    const [query, setQuery] = createSignal("");
    const [selected, setSelected] = createSignal(0);
    const [scope, setScope] = createSignal<SearchScope>("All");
    const commands = createMemo(() =>
        commandsFor([...siteSearchEntries, ...entries()], source()),
    );
    const results = createMemo(() =>
        matchCommands(commands(), query(), resultLimit, scope()),
    );

    // open the palette from anywhere outside an editable control
    onMount(() => {
        const handleKey = (event: KeyboardEvent) => {
            const isCommand =
                (event.metaKey || event.ctrlKey) &&
                event.key.toLowerCase() === "k";
            const isSlash = event.key === "/" && !isEditable(event.target);
            if (!isCommand && !isSlash) {
                return;
            }

            event.preventDefault();
            void openPalette();
        };
        const handleOpen = () => void openPalette();

        document.addEventListener("keydown", handleKey);
        document.addEventListener(commandEvents.open, handleOpen);
        onCleanup(() => {
            document.removeEventListener("keydown", handleKey);
            document.removeEventListener(commandEvents.open, handleOpen);
        });
    });

    // load search content only when somebody asks for it
    const openPalette = async () => {
        // show local commands while the content index loads
        setQuery("");
        setSelected(0);
        setSource(currentPageSource());
        dialog?.showModal();
        queueMicrotask(() => input?.focus());

        // report index failures in the open dialog and retry on the next open
        if (entries().length === 0 && !loading()) {
            setLoading(true);
            setError(undefined);
            try {
                setEntries(await loadSearchEntries());
                setSelected(0);
            } catch (error: unknown) {
                setError(
                    error instanceof Error ? error.message : String(error),
                );
            } finally {
                setLoading(false);
            }
        }
    };

    const choose = (command: Command) => {
        dialog?.close();

        if (command.action.kind === "navigate") {
            if (isExternalLink(command.action.href)) {
                window.open(
                    command.action.href,
                    "_blank",
                    "noopener,noreferrer",
                );
            } else {
                window.location.assign(command.action.href);
            }
        } else {
            document.dispatchEvent(new CustomEvent(command.action.event));
        }
    };

    const handleInputKey = (event: KeyboardEvent) => {
        if (event.key === "Escape") {
            event.preventDefault();
            dialog?.close();
        } else if (event.key === "ArrowDown") {
            event.preventDefault();
            moveSelection((selected() + 1) % Math.max(results().length, 1));
        } else if (event.key === "ArrowUp") {
            event.preventDefault();
            moveSelection(
                (selected() - 1 + Math.max(results().length, 1)) %
                    Math.max(results().length, 1),
            );
        } else if (event.key === "Enter") {
            const match = results()[selected()];
            if (match != undefined) {
                event.preventDefault();
                choose(match.command);
            }
        }
    };

    const moveSelection = (index: number) => {
        setSelected(index);
        queueMicrotask(() => {
            document
                .getElementById(`search-result-${index}`)
                ?.scrollIntoView({ block: "nearest" });
        });
    };

    return (
        <>
            <button
                aria-label="Search"
                {...stylex.attrs(styles.toggle)}
                onClick={() => void openPalette()}
                title="Search"
                type="button"
            >
                search
            </button>

            <Portal>
                <dialog
                    aria-label="Search Destack"
                    {...stylex.attrs(styles.palette)}
                    onClick={(event) => {
                        if (event.target === dialog) {
                            dialog.close();
                        }
                    }}
                    ref={dialog}
                >
                    <div {...stylex.attrs(styles.frame)}>
                        <div {...stylex.attrs(styles.inputLabel)}>
                            <svg
                                aria-hidden="true"
                                width="18"
                                height="18"
                                viewBox="0 0 20 20"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="1.5"
                            >
                                <circle cx="8" cy="8" r="5.5" />
                                <path d="m12 12 5 5" />
                            </svg>
                            <input
                                {...stylex.attrs(styles.input)}
                                aria-activedescendant={
                                    results().length === 0
                                        ? undefined
                                        : `search-result-${selected()}`
                                }
                                aria-autocomplete="list"
                                aria-controls="search-results"
                                aria-expanded="true"
                                aria-label="Search query"
                                autocomplete="off"
                                onInput={(event) => {
                                    setQuery(event.currentTarget.value);
                                    setSelected(0);
                                }}
                                onKeyDown={handleInputKey}
                                placeholder={
                                    scope() === "All"
                                        ? "Search Destack…"
                                        : `Search ${scope().toLowerCase()}…`
                                }
                                ref={input}
                                role="combobox"
                                type="search"
                                value={query()}
                            />
                            <button
                                {...stylex.attrs(styles.close)}
                                aria-label="Close search"
                                onClick={() => dialog?.close()}
                                type="button"
                            >
                                Close
                            </button>
                        </div>

                        {/* keep the selected collection visible while searching */}
                        <label {...stylex.attrs(styles.scopes)}>
                            Search in
                            <select
                                aria-label="Search scope"
                                {...stylex.attrs(styles.scope)}
                                value={scope()}
                                onChange={(event) => {
                                    setScope(
                                        event.currentTarget
                                            .value as SearchScope,
                                    );
                                    setSelected(0);
                                    input?.focus();
                                }}
                            >
                                <For each={searchScopes}>
                                    {(name) => (
                                        <option value={name}>
                                            {name === "All"
                                                ? "Everything"
                                                : name}
                                        </option>
                                    )}
                                </For>
                            </select>
                        </label>

                        <ol
                            {...stylex.attrs(styles.resultList)}
                            id="search-results"
                            role="listbox"
                        >
                            <For each={results()}>
                                {(match, index) => (
                                    <li
                                        {...stylex.attrs(
                                            styles.resultRow,
                                            index() === 0 &&
                                                styles.resultRowFirst,
                                        )}
                                        role="none"
                                    >
                                        <button
                                            {...stylex.attrs(
                                                styles.resultButton,
                                                selected() === index() &&
                                                    styles.selected,
                                            )}
                                            aria-selected={
                                                selected() === index()
                                            }
                                            id={`search-result-${index()}`}
                                            onClick={(event) => {
                                                event.preventDefault();
                                                choose(match.command);
                                            }}
                                            onMouseMove={() =>
                                                setSelected(index())
                                            }
                                            role="option"
                                            type="button"
                                        >
                                            <span
                                                aria-hidden="true"
                                                {...stylex.attrs(
                                                    styles.indicator,
                                                )}
                                            >
                                                {selected() === index()
                                                    ? ">"
                                                    : ""}
                                            </span>
                                            <span
                                                {...stylex.attrs(styles.result)}
                                            >
                                                <strong
                                                    {...stylex.attrs(
                                                        styles.resultLabel,
                                                    )}
                                                >
                                                    <Highlight
                                                        match={match}
                                                        text={
                                                            match.command.label
                                                        }
                                                    />
                                                </strong>
                                                <Show
                                                    when={query().trim() !== ""}
                                                >
                                                    <span
                                                        {...stylex.attrs(
                                                            styles.context,
                                                        )}
                                                    >
                                                        <Highlight
                                                            match={match}
                                                            text={
                                                                match.command
                                                                    .context
                                                            }
                                                        />
                                                    </span>
                                                </Show>
                                                <Show
                                                    when={match.excerpt !== ""}
                                                >
                                                    <span
                                                        {...stylex.attrs(
                                                            styles.excerpt,
                                                        )}
                                                    >
                                                        <Highlight
                                                            match={match}
                                                            text={match.excerpt}
                                                        />
                                                    </span>
                                                </Show>
                                            </span>
                                            <Show when={match.command.shortcut}>
                                                {(shortcut) => (
                                                    <kbd
                                                        {...stylex.attrs(
                                                            styles.shortcut,
                                                        )}
                                                    >
                                                        alt+{shortcut()}
                                                    </kbd>
                                                )}
                                            </Show>
                                        </button>
                                    </li>
                                )}
                            </For>
                        </ol>

                        <Show when={loading()}>
                            <p {...stylex.attrs(styles.empty)} role="status">
                                Loading search…
                            </p>
                        </Show>
                        <Show when={error()}>
                            {(message) => (
                                <p {...stylex.attrs(styles.empty)} role="alert">
                                    {message()}
                                </p>
                            )}
                        </Show>
                        <Show
                            when={
                                !loading() && !error() && results().length === 0
                            }
                        >
                            <p {...stylex.attrs(styles.empty)}>no matches</p>
                        </Show>
                        <footer {...stylex.attrs(styles.help)}>
                            <span>↑ ↓ Navigate</span>
                            <span>↵ Open</span>
                            <span>Esc Close</span>
                        </footer>
                    </div>
                </dialog>
            </Portal>
        </>
    );
}

/// Read the current page formats advertised by the reader toolbar.
function currentPageSource(): PageFormats | undefined {
    const element = document.querySelector<HTMLElement>("[data-page-source]");
    if (element == undefined) {
        return undefined;
    }

    const markdownRoute = element.dataset.markdownRoute;
    const textRoute = element.dataset.textRoute;
    if (markdownRoute == undefined || textRoute == undefined) {
        throw new Error("page source routes are missing");
    }

    return { markdownRoute, textRoute };
}

type HighlightProps = {
    /// The search match defining highlighted terms.
    match: CommandMatch;

    /// The visible text to segment.
    text: string;
};

/// Render case-preserving query highlights.
function Highlight(props: HighlightProps) {
    return (
        <For each={highlightParts(props.text, props.match.terms)}>
            {(part) =>
                part.isMatch ? (
                    <mark {...stylex.attrs(styles.mark)}>{part.text}</mark>
                ) : (
                    part.text
                )
            }
        </For>
    );
}

/// Return whether the keyboard event originated in editable content.
function isEditable(target: EventTarget | null) {
    return (
        target instanceof HTMLInputElement ||
        target instanceof HTMLTextAreaElement ||
        (target instanceof HTMLElement && target.isContentEditable)
    );
}

const styles = stylex.create({
    close: {
        backgroundColor: "transparent",
        borderWidth: 0,
        color: tokens.ink,
        cursor: "pointer",
        font: "inherit",
        padding: 0,
        ":hover": {
            color: tokens.accent,
        },
    },
    context: {
        color: tokens.ink,
        fontSize: "var(--size-label)",
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
        maxWidth: "15rem",
        textAlign: "right",
        "@media (max-width: 640px)": {
            gridColumn: "1 / -1",
            maxWidth: "none",
            textAlign: "left",
        },
    },
    empty: {
        color: tokens.ink,
        margin: 0,
        padding: "1rem",
    },
    excerpt: {
        gridColumn: "1 / -1",
        color: tokens.ink,
        fontSize: "var(--size-label)",
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    frame: {
        display: "grid",
        minHeight: 0,
        minWidth: 0,
    },
    indicator: {
        color: tokens.accent,
    },
    input: {
        backgroundColor: "transparent",
        borderWidth: 0,
        color: "inherit",
        font: "inherit",
        minWidth: 0,
        outlineWidth: 0,
        "::placeholder": {
            color: tokens.ink,
            opacity: 1,
        },
    },
    inputLabel: {
        alignItems: "center",
        backgroundColor: tokens.creamDeep,
        borderBottomColor: tokens.line,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        display: "grid",
        gap: "0.5rem",
        gridTemplateColumns: "1rem minmax(0, 1fr) auto",
        padding: "1rem",
    },
    mark: {
        backgroundColor: "transparent",
        color: tokens.accent,
        fontWeight: 600,
    },
    palette: {
        backgroundColor: tokens.cream,
        borderColor: tokens.line,
        borderStyle: "solid",
        borderWidth: tokens.hairline,
        boxShadow: "0 16px 64px rgb(18 49 60 / 18%)",
        color: tokens.ink,
        fontFamily: tokens.textFont,
        fontSize: "var(--size-body)",
        margin: "min(12svh, 6rem) auto auto",
        maxHeight: "calc(100dvh - 2rem)",
        maxWidth: "none",
        padding: 0,
        width: "min(42rem, calc(100vw - 2rem))",
        "::backdrop": {
            backgroundColor: "rgb(23 26 27 / 35%)",
        },
        "@media (max-width: 640px)": {
            marginTop: "1rem",
        },
    },
    result: {
        display: "grid",
        gridTemplateColumns: "minmax(0, 1fr) minmax(0, auto)",
        gap: "0.2rem 1rem",
        minWidth: 0,
        "@media (max-width: 640px)": {
            gridTemplateColumns: "minmax(0, 1fr)",
        },
    },
    resultButton: {
        alignItems: "center",
        backgroundColor: "transparent",
        borderWidth: 0,
        color: tokens.ink,
        display: "grid",
        font: "inherit",
        gap: "0.75rem",
        gridTemplateColumns: "1rem minmax(0, 1fr) auto",
        minHeight: "2.75rem",
        padding: "0.5rem 0.75rem",
        textAlign: "left",
        width: "100%",
        ":hover": {
            backgroundColor: tokens.creamDeep,
            color: tokens.ink,
        },
        "@media (max-width: 640px)": {
            gridTemplateColumns: "1rem minmax(0, 1fr) auto",
        },
    },
    resultLabel: {
        color: tokens.ink,
        fontWeight: 600,
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    resultList: {
        listStyle: "none",
        margin: 0,
        maxHeight: "min(28rem, calc(100dvh - 13rem))",
        overflowY: "auto",
        padding: "0.5rem",
    },
    resultRow: {
        borderTopColor: tokens.line,
        borderTopStyle: "solid",
        borderTopWidth: 0,
    },
    resultRowFirst: {
        borderTopWidth: 0,
    },
    selected: {
        backgroundColor: tokens.creamDeep,
        color: tokens.ink,
    },
    scopes: {
        display: "flex",
        alignItems: "center",
        gap: "0.75rem",
        fontSize: "var(--size-navigation)",
        paddingInline: "1rem",
        borderBottomColor: tokens.line,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
    },
    scope: {
        backgroundColor: "transparent",
        borderWidth: 0,
        borderBottomColor: "transparent",
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        color: tokens.ink,
        cursor: "pointer",
        font: "inherit",
        fontSize: "var(--size-navigation)",
        fontWeight: 600,
        padding: "0.7rem 0.5rem",
        whiteSpace: "nowrap",
        ":hover": { color: tokens.accent },
    },
    help: {
        display: "flex",
        gap: "1.25rem",
        borderTopColor: tokens.line,
        borderTopStyle: "solid",
        borderTopWidth: tokens.hairline,
        padding: "0.65rem 1rem",
        fontSize: "var(--size-navigation)",
    },
    shortcut: {
        color: tokens.ink,
        font: "inherit",
        whiteSpace: "nowrap",
    },
    toggle: {
        alignItems: "center",
        backgroundColor: "transparent",
        borderWidth: 0,
        color: "inherit",
        display: "inline-flex",
        font: "inherit",
        letterSpacing: "inherit",
        padding: 0,
        textTransform: "inherit",
        ":hover": {
            color: tokens.accent,
        },
    },
});
