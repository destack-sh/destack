import { color, fontFamily } from "@destack/theme/tokens.stylex";
import { createMemo, createSignal, For, onSettled, Show } from "@destack/view";
import { Portal } from "@destack/view";
import * as stylex from "@destack/style";

import {
    type Command,
    commandEvents,
    type CommandMatch,
    commandsFor,
    highlightParts,
    matchCommands,
    type SearchScope,
    searchScopes,
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
    const commands = createMemo(() => commandsFor([...siteSearchEntries, ...entries()], source()));
    const results = createMemo(() => matchCommands(commands(), query(), resultLimit, scope()));

    // open the palette from anywhere outside an editable control
    onSettled(() => {
        const handleKey = (event: KeyboardEvent) => {
            const isCommand = (event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k";
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
        return () => {
            document.removeEventListener("keydown", handleKey);
            document.removeEventListener(commandEvents.open, handleOpen);
        };
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
                setError(error instanceof Error ? error.message : String(error));
            } finally {
                setLoading(false);
            }
        }
    };

    const choose = (command: Command) => {
        dialog?.close();

        if (command.action.kind === "navigate") {
            if (isExternalLink(command.action.href)) {
                window.open(command.action.href, "_blank", "noopener,noreferrer");
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
                (selected() - 1 + Math.max(results().length, 1)) % Math.max(results().length, 1),
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
            document.getElementById(`search-result-${index}`)?.scrollIntoView({ block: "nearest" });
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
                <svg
                    aria-hidden="true"
                    width="20"
                    height="20"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.5"
                >
                    <circle cx="10" cy="10" r="6" />
                    <path d="m15 15 5 5" />
                </svg>
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
                                <svg
                                    aria-hidden="true"
                                    width="20"
                                    height="20"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="1.5"
                                >
                                    <path d="m6 6 12 12M6 18 18 6" />
                                </svg>
                            </button>
                        </div>

                        {/* keep the selected collection visible while searching */}
                        <div
                            {...stylex.attrs(styles.scopes)}
                            role="group"
                            aria-label="Search scope"
                        >
                            <For each={searchScopes}>
                                {(name) => (
                                    <button
                                        {...stylex.attrs(
                                            styles.scope,
                                            scope() === name && styles.scopeSelected,
                                        )}
                                        aria-pressed={scope() === name ? "true" : "false"}
                                        onClick={() => {
                                            setScope(name);
                                            setSelected(0);
                                            input?.focus();
                                        }}
                                        type="button"
                                    >
                                        {name}
                                    </button>
                                )}
                            </For>
                        </div>

                        <ol {...stylex.attrs(styles.resultList)} id="search-results" role="listbox">
                            <For each={results()}>
                                {(match, index) => (
                                    <li role="none">
                                        <button
                                            {...stylex.attrs(
                                                styles.resultButton,
                                                selected() === index() && styles.selected,
                                            )}
                                            aria-selected={
                                                selected() === index() ? "true" : "false"
                                            }
                                            id={`search-result-${index()}`}
                                            onClick={(event) => {
                                                event.preventDefault();
                                                choose(match.command);
                                            }}
                                            onMouseMove={() => setSelected(index())}
                                            role="option"
                                            tabindex={-1}
                                            type="button"
                                        >
                                            <svg
                                                aria-hidden="true"
                                                {...stylex.attrs(styles.resultIcon)}
                                                width="18"
                                                height="18"
                                                viewBox="0 0 24 24"
                                                fill="none"
                                                stroke="currentColor"
                                                stroke-width="1.5"
                                                stroke-linecap="round"
                                                stroke-linejoin="round"
                                            >
                                                <Show
                                                    when={match.command.kind === "action"}
                                                    fallback={
                                                        <>
                                                            <path d="M14 3H6a1 1 0 0 0-1 1v16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1V8Z" />
                                                            <path d="M14 3v5h5M9 12h6M9 16h4" />
                                                        </>
                                                    }
                                                >
                                                    <path d="m5 7 5 5-5 5m8 0h6" />
                                                </Show>
                                            </svg>
                                            <span {...stylex.attrs(styles.result)}>
                                                <strong {...stylex.attrs(styles.resultLabel)}>
                                                    <Highlight
                                                        match={match}
                                                        text={match.command.label}
                                                    />
                                                </strong>
                                                <Show when={match.command.context !== ""}>
                                                    <span {...stylex.attrs(styles.context)}>
                                                        <Highlight
                                                            match={match}
                                                            text={match.command.context}
                                                        />
                                                    </span>
                                                </Show>
                                                <Show when={match.excerpt !== ""}>
                                                    <span {...stylex.attrs(styles.excerpt)}>
                                                        <Highlight
                                                            match={match}
                                                            text={match.excerpt}
                                                        />
                                                    </span>
                                                </Show>
                                            </span>
                                            <span
                                                {...stylex.attrs(styles.resultAction)}
                                                aria-hidden="true"
                                            >
                                                <Show
                                                    when={selected() === index()}
                                                    fallback={
                                                        match.command.shortcut && (
                                                            <kbd {...stylex.attrs(styles.shortcut)}>
                                                                Alt{" "}
                                                                {match.command.shortcut?.toUpperCase()}
                                                            </kbd>
                                                        )
                                                    }
                                                >
                                                    <kbd {...stylex.attrs(styles.shortcut)}>↵</kbd>
                                                </Show>
                                            </span>
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
                        <Show when={!loading() && !error() && results().length === 0}>
                            <p {...stylex.attrs(styles.empty)} role="status">
                                No results found.
                            </p>
                        </Show>
                        <footer {...stylex.attrs(styles.help)}>
                            <span {...stylex.attrs(styles.helpKeys)}>
                                <kbd {...stylex.attrs(styles.shortcut)}>↑</kbd>
                                <kbd {...stylex.attrs(styles.shortcut)}>↓</kbd> Navigate
                            </span>
                            <span {...stylex.attrs(styles.helpKeys)}>
                                <kbd {...stylex.attrs(styles.shortcut)}>↵</kbd> Open
                            </span>
                            <span {...stylex.attrs(styles.helpKeys)}>
                                <kbd {...stylex.attrs(styles.shortcut)}>Esc</kbd> Close
                            </span>
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
                part.isMatch ? <mark {...stylex.attrs(styles.mark)}>{part.text}</mark> : part.text
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
        color: color.mutedForeground,
        cursor: "pointer",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        padding: 0,
        width: "2.75rem",
        height: "2.75rem",
        marginRight: "-0.75rem",
        borderRadius: "0.25rem",
        ":hover": { backgroundColor: tokens.creamDeep, color: color.foreground },
        ":focus-visible": { outline: "2px solid", outlineColor: color.primary },
    },
    context: {
        color: color.mutedForeground,
        fontSize: "var(--size-label)",
        lineHeight: "1.5",
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    empty: {
        color: color.mutedForeground,
        fontSize: "1rem",
        margin: 0,
        padding: "2rem 1rem",
        textAlign: "center",
    },
    excerpt: {
        color: color.mutedForeground,
        fontSize: "var(--size-label)",
        lineHeight: "1.5",
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    frame: {
        display: "flex",
        flexDirection: "column",
        maxHeight: "inherit",
        minHeight: 0,
        minWidth: 0,
    },
    input: {
        backgroundColor: "transparent",
        borderWidth: 0,
        color: color.foreground,
        font: "inherit",
        height: "2rem",
        minWidth: 0,
        outlineWidth: 0,
        padding: 0,
        "::placeholder": { color: color.mutedForeground, opacity: 1 },
        "@media (max-width: 640px)": { fontSize: "1rem" },
    },
    inputLabel: {
        alignItems: "center",
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        color: color.mutedForeground,
        display: "grid",
        flexShrink: 0,
        gap: "0.75rem",
        gridTemplateColumns: "1.125rem minmax(0, 1fr) auto",
        padding: "0.375rem 1rem",
    },
    mark: {
        backgroundColor: "transparent",
        color: color.primary,
        fontWeight: 500,
    },
    palette: {
        backgroundColor: color.background,
        borderColor: color.border,
        borderRadius: "0.25rem",
        borderStyle: "solid",
        borderWidth: tokens.hairline,
        boxShadow: "0 24px 80px rgb(0 0 0 / 20%)",
        color: color.foreground,
        fontFamily: fontFamily.default,
        fontSize: "1rem",
        margin: "min(12svh, 6rem) auto auto",
        maxHeight: "min(40rem, 80dvh)",
        maxWidth: "none",
        overflow: "hidden",
        padding: 0,
        width: "min(40rem, calc(100vw - 2rem))",
        "::backdrop": { backgroundColor: "rgb(0 0 0 / 40%)" },
        "@media (max-width: 640px)": {
            marginTop: "1rem",
            maxHeight: "calc(100dvh - 2rem)",
        },
    },
    result: {
        display: "grid",
        gap: "0.125rem",
        minWidth: 0,
    },
    resultIcon: {
        color: color.mutedForeground,
        alignSelf: "start",
        marginTop: "0.125rem",
    },
    resultAction: {
        display: "flex",
        justifyContent: "end",
        minWidth: "1.25rem",
    },
    resultButton: {
        alignItems: "center",
        backgroundColor: "transparent",
        borderWidth: 0,
        borderRadius: "0.25rem",
        color: color.foreground,
        cursor: "pointer",
        display: "grid",
        font: "inherit",
        gap: "0.75rem",
        gridTemplateColumns: "1.125rem minmax(0, 1fr) auto",
        minHeight: "2.75rem",
        padding: "0.625rem 0.5rem",
        textAlign: "left",
        width: "100%",
    },
    resultLabel: {
        fontWeight: 500,
        lineHeight: "1.25rem",
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    resultList: {
        listStyle: "none",
        margin: 0,
        minHeight: 0,
        overflowY: "auto",
        overscrollBehavior: "contain",
        padding: "0.5rem",
        scrollbarWidth: "thin",
    },
    selected: { backgroundColor: tokens.creamDeep },
    scopes: {
        display: "flex",
        alignItems: "center",
        flexShrink: 0,
        gap: "0.25rem",
        overflowX: "auto",
        scrollbarWidth: "thin",
        padding: "0.5rem",
        "@media (max-width: 640px)": { flexWrap: "wrap" },
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
    },
    scope: {
        backgroundColor: "transparent",
        borderWidth: 0,
        borderRadius: "0.25rem",
        color: color.mutedForeground,
        cursor: "pointer",
        flexShrink: 0,
        font: "inherit",
        fontSize: "var(--size-label)",
        padding: "0.5rem",
        whiteSpace: "nowrap",
        ":hover": { backgroundColor: tokens.creamDeep, color: color.foreground },
        ":focus-visible": {
            outlineColor: color.primary,
            outlineStyle: "solid",
            outlineWidth: "2px",
            outlineOffset: "2px",
        },
    },
    scopeSelected: {
        backgroundColor: tokens.creamDeep,
        color: color.foreground,
        textDecorationLine: "underline",
        textDecorationColor: color.primary,
        textDecorationThickness: "1px",
        textUnderlineOffset: "0.3em",
    },
    help: {
        alignItems: "center",
        color: color.mutedForeground,
        display: "flex",
        flexShrink: 0,
        gap: "1rem",
        borderTopColor: color.border,
        borderTopStyle: "solid",
        borderTopWidth: tokens.hairline,
        padding: "0.75rem 1rem",
        fontSize: "var(--size-label)",
        "@media (pointer: coarse)": { display: "none" },
    },
    helpKeys: { display: "inline-flex", alignItems: "center", gap: "0.25rem" },
    shortcut: {
        alignItems: "center",
        backgroundColor: color.background,
        borderColor: color.border,
        borderRadius: "0.25rem",
        borderStyle: "solid",
        borderWidth: tokens.hairline,
        color: color.mutedForeground,
        display: "inline-flex",
        fontFamily: fontFamily.code,
        fontSize: "0.75rem",
        fontWeight: 400,
        height: "1.25rem",
        justifyContent: "center",
        minWidth: "1.25rem",
        paddingInline: "0.25rem",
        whiteSpace: "nowrap",
    },
    toggle: {
        justifyContent: "center",
        width: "2.75rem",
        height: "2.75rem",
        cursor: "pointer",
        alignItems: "center",
        backgroundColor: "transparent",
        borderWidth: 0,
        color: "inherit",
        display: "inline-flex",
        font: "inherit",
        letterSpacing: "inherit",
        padding: 0,
        textTransform: "inherit",
        ":hover": { color: color.primary },
    },
});
