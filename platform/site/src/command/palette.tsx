import { createMemo, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { Portal } from "solid-js/web";
import * as stylex from "@stylexjs/stylex";

import {
    type Command,
    type CommandMatch,
    commandEvents,
    commandsFor,
    highlightParts,
    matchCommands,
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
    const [source, setSource] = createSignal<PageFormats>();
    const [query, setQuery] = createSignal("");
    const [selected, setSelected] = createSignal(0);
    const commands = createMemo(() => commandsFor([...siteSearchEntries, ...entries()], source()));
    const results = createMemo(() => matchCommands(commands(), query(), resultLimit));

    // open the palette from anywhere outside an editable control
    onMount(() => {
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
        onCleanup(() => {
            document.removeEventListener("keydown", handleKey);
            document.removeEventListener(commandEvents.open, handleOpen);
        });
    });

    // load search content only when somebody asks for it
    const openPalette = async () => {
        if (entries().length === 0) {
            setEntries(await loadSearchEntries());
        }

        setQuery("");
        setSelected(0);
        setSource(currentPageSource());
        dialog?.showModal();
        queueMicrotask(() => input?.focus());
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
        if (event.key === "ArrowDown") {
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
                        <label {...stylex.attrs(styles.inputLabel)}>
                            <span {...stylex.attrs(styles.inputPrompt)}>&gt;</span>
                            <input
                                {...stylex.attrs(styles.input)}
                                aria-activedescendant={
                                    results().length === 0 ? undefined : `search-result-${selected()}`
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
                                placeholder="search everything"
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
                                close
                            </button>
                        </label>

                        <ol {...stylex.attrs(styles.resultList)} id="search-results" role="listbox">
                            <For each={results()}>
                                {(match, index) => (
                                    <li {...stylex.attrs(styles.resultRow, index() === 0 && styles.resultRowFirst)} role="none">
                                        <button
                                            {...stylex.attrs(
                                                styles.resultButton,
                                                selected() === index() && styles.selected,
                                            )}
                                            aria-selected={selected() === index()}
                                            id={`search-result-${index()}`}
                                            onClick={(event) => {
                                                event.preventDefault();
                                                choose(match.command);
                                            }}
                                            onMouseMove={() => setSelected(index())}
                                            role="option"
                                            type="button"
                                        >
                                            <span aria-hidden="true" {...stylex.attrs(styles.indicator)}>
                                                {selected() === index() ? ">" : ""}
                                            </span>
                                            <span {...stylex.attrs(styles.result)}>
                                                <strong {...stylex.attrs(styles.resultLabel)}>
                                                    <Highlight match={match} text={match.command.label} />
                                                </strong>
                                                <span {...stylex.attrs(styles.context)}>
                                                    <Highlight match={match} text={match.command.context} />
                                                </span>
                                                <Show when={match.excerpt !== ""}>
                                                    <span {...stylex.attrs(styles.excerpt)}>
                                                        <Highlight match={match} text={match.excerpt} />
                                                    </span>
                                                </Show>
                                            </span>
                                            <Show when={match.command.shortcut}>
                                                {(shortcut) => (
                                                    <kbd {...stylex.attrs(styles.shortcut)}>
                                                        alt+{shortcut()}
                                                    </kbd>
                                                )}
                                            </Show>
                                        </button>
                                    </li>
                                )}
                            </For>
                        </ol>

                        <Show when={results().length === 0}>
                            <p {...stylex.attrs(styles.empty)}>no matches</p>
                        </Show>
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
            {(part) => part.isMatch ? <mark {...stylex.attrs(styles.mark)}>{part.text}</mark> : part.text}
        </For>
    );
}

/// Return whether the keyboard event originated in editable content.
function isEditable(target: EventTarget | null) {
    return target instanceof HTMLInputElement
        || target instanceof HTMLTextAreaElement
        || (target instanceof HTMLElement && target.isContentEditable);
}

const styles = stylex.create({
    close: {
        backgroundColor: "transparent",
        borderWidth: 0,
        color: tokens.soft,
        cursor: "pointer",
        font: "inherit",
        padding: 0,
        ":hover": {
            color: tokens.accent,
        },
    },
    context: {
        color: tokens.soft,
        fontSize: "var(--size-label)",
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    empty: {
        color: tokens.soft,
        margin: 0,
        padding: "1rem",
    },
    excerpt: {
        color: tokens.soft,
        fontSize: "0.8rem",
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
            color: tokens.soft,
            opacity: 1,
        },
    },
    inputLabel: {
        alignItems: "center",
        borderBottomColor: tokens.line,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        display: "grid",
        gap: "0.5rem",
        gridTemplateColumns: "1rem minmax(0, 1fr) auto",
        padding: "0.75rem 1rem",
    },
    inputPrompt: {
        color: tokens.accent,
    },
    mark: {
        backgroundColor: "transparent",
        color: tokens.accent,
        fontWeight: 600,
    },
    palette: {
        backgroundColor: tokens.cream,
        borderColor: tokens.ink,
        borderStyle: "solid",
        borderWidth: tokens.hairline,
        color: tokens.ink,
        fontFamily: tokens.monoFont,
        fontSize: tokens.siteFontSize,
        margin: "10svh auto auto",
        maxHeight: "min(42rem, calc(100svh - 2rem))",
        maxWidth: "none",
        padding: 0,
        width: "min(42rem, calc(100vw - 2rem))",
        "::backdrop": {
            backgroundColor: "rgb(5 46 64 / 72%)",
        },
    },
    result: {
        display: "grid",
        gap: "0.25rem",
        minWidth: 0,
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
        padding: "0.625rem 1rem",
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
        maxHeight: "min(34rem, calc(100svh - 8rem))",
        overflowY: "auto",
        padding: 0,
    },
    resultRow: {
        borderTopColor: tokens.line,
        borderTopStyle: "solid",
        borderTopWidth: tokens.hairline,
    },
    resultRowFirst: {
        borderTopWidth: 0,
    },
    selected: {
        backgroundColor: tokens.creamDeep,
        color: tokens.ink,
    },
    shortcut: {
        color: tokens.soft,
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
