import { type Accessor, createSignal, onCleanup, onMount } from "solid-js";
import * as stylex from "@stylexjs/stylex";

import { commandEvents } from "../command/command";
import type { PageSource } from "../content/source";
import { tokens } from "../style/tokens.stylex";

const narrow = "@media (width < 60rem)";
const tiny = "@media (max-width: 360px)";

/// Properties for the page source controls.
type SourceActionsProps = {
    /// The shared page source commands.
    commands: PageSourceCommands;
};

/// The latest source-copy outcome.
type SourceState = "" | "error" | "md" | "txt";

/// One portable source format.
type SourceKind = "md" | "txt";

/// The state and commands shared by responsive source controls.
export type PageSourceCommands = {
    /// Copy one source format.
    copy: (kind: SourceKind) => void;

    /// The current page source files.
    source: PageSource;

    /// The latest source-copy outcome.
    state: Accessor<SourceState>;

    /// The formatted token count or copy failure.
    tokenLabel: Accessor<string>;
};

/// Create source commands shared by responsive article controls.
export function createPageSourceCommands(source: PageSource): PageSourceCommands {
    const [state, setState] = createSignal<SourceState>("");
    let reset: ReturnType<typeof setTimeout> | undefined;

    // cancel a pending status reset when the reader is replaced
    onCleanup(() => clearTimeout(reset));

    const write = async (kind: SourceKind) => {
        // load the requested source format
        const route = kind === "md" ? source.markdownRoute : source.textRoute;
        const response = await fetch(route);

        // surface unavailable generated sources
        if (!response.ok) {
            throw new Error(`failed to load ${route}: ${response.status}`);
        }

        // publish the source and briefly report success
        await navigator.clipboard.writeText(await response.text());
        setState(kind);
        clearTimeout(reset);
        reset = setTimeout(() => setState(""), 1600);
    };

    const copy = (kind: SourceKind) => {
        // report clipboard failures through the visible source status
        void write(kind).catch((error: unknown) => {
            console.error(error);
            setState("error");
        });
    };

    onMount(() => {
        // bind command palette copy actions to this page
        const copyMarkdown = () => copy("md");
        const copyText = () => copy("txt");

        document.addEventListener(commandEvents.copyMarkdown, copyMarkdown);
        document.addEventListener(commandEvents.copyText, copyText);

        // unbind page actions when the reader is replaced
        onCleanup(() => {
            document.removeEventListener(commandEvents.copyMarkdown, copyMarkdown);
            document.removeEventListener(commandEvents.copyText, copyText);
        });
    });

    const tokenLabel = () =>
        state() === "error" ? "copy failed" : `~${formatTokens(source.tokens)} tokens`;

    return { copy, source, state, tokenLabel };
}

/// Render source links and copy controls shared by articles and documentation.
export function SourceActions(props: SourceActionsProps) {
    const commands = props.commands;

    return (
        <div {...stylex.attrs(styles.controls)}>
            <details {...stylex.attrs(styles.menu)} name="reader-tools">
                <summary {...stylex.attrs(styles.menuSummary)}>
                    <span>source</span>
                    <span {...stylex.attrs(styles.tokens)}>{commands.tokenLabel()}</span>
                </summary>
                <nav aria-label="Page formats" {...stylex.attrs(styles.menuBody)}>
                    <span {...stylex.attrs(styles.menuTokens)}>{commands.tokenLabel()}</span>
                    <SourceLinks commands={commands} />
                </nav>
            </details>
        </div>
    );
}

/// Properties for the reusable source links.
type SourceLinksProps = {
    /// The shared page source commands.
    commands: PageSourceCommands;
};

/// Render links and copy controls for the portable source formats.
function SourceLinks(props: SourceLinksProps) {
    const commands = props.commands;

    return (
        <>
            <span {...stylex.attrs(styles.group)}>
                <span {...stylex.attrs(styles.label)}>view</span>
                <a
                    {...stylex.attrs(styles.action)}
                    href={commands.source.markdownRoute}
                    rel="alternate noopener"
                    target="_blank"
                    type="text/markdown"
                >
                    .md
                </a>
                <a
                    {...stylex.attrs(styles.action)}
                    href={commands.source.textRoute}
                    rel="alternate noopener"
                    target="_blank"
                    type="text/plain"
                >
                    .txt
                </a>
            </span>

            <span {...stylex.attrs(styles.group)}>
                <span {...stylex.attrs(styles.label)}>copy</span>
                <button
                    {...stylex.attrs(styles.action)}
                    aria-label="Copy Markdown"
                    onClick={() => commands.copy("md")}
                    type="button"
                >
                    {commands.state() === "md" ? "copied" : ".md"}
                </button>
                <button
                    {...stylex.attrs(styles.action)}
                    aria-label="Copy text"
                    onClick={() => commands.copy("txt")}
                    type="button"
                >
                    {commands.state() === "txt" ? "copied" : ".txt"}
                </button>
            </span>
        </>
    );
}

/// Format an approximate token count compactly.
function formatTokens(tokens: number) {
    if (tokens < 1000) {
        return String(tokens);
    }

    const precision = tokens < 10_000 ? 1 : 0;

    return `${(tokens / 1000).toFixed(precision)}k`;
}

const styles = stylex.create({
    action: {
        backgroundColor: "transparent",
        borderWidth: 0,
        color: tokens.text,
        font: "inherit",
        padding: 0,
        textDecoration: "none",
        ":hover": {
            color: tokens.accent,
        },
    },
    controls: {
        minWidth: 0,
        [narrow]: {
            gridColumn: 2,
            gridRow: 1,
        },
    },
    group: {
        alignItems: "baseline",
        display: "inline-flex",
        gap: "0.5rem",
        whiteSpace: "nowrap",
    },
    label: {
        color: tokens.soft,
    },
    menu: {
        display: "block",
        position: "relative",
    },
    menuBody: {
        backgroundColor: tokens.page,
        borderColor: tokens.ink,
        borderRadius: tokens.panelRadius,
        borderStyle: "solid",
        borderWidth: "1px",
        display: "flex",
        flexWrap: "wrap",
        fontFamily: tokens.monoFont,
        fontSize: "0.78rem",
        gap: "0.25rem 0.75rem",
        justifyContent: "flex-end",
        padding: "0.5rem 0.75rem",
        position: "absolute",
        right: 0,
        top: "100%",
        width: `min(22rem, calc(100vw - ${tokens.gutterLeft} - ${tokens.gutterRight}))`,
        zIndex: 4,
    },
    menuSummary: {
        alignItems: "center",
        color: tokens.text,
        cursor: "pointer",
        display: "flex",
        fontFamily: tokens.monoFont,
        fontSize: "0.78rem",
        fontWeight: 600,
        gap: "0.75rem",
        listStyle: "none",
        minHeight: tokens.siteControlHeight,
    },
    menuTokens: {
        color: tokens.soft,
        display: "none",
        minHeight: tokens.siteControlHeight,
        width: "100%",
        [tiny]: {
            alignItems: "center",
            display: "inline-flex",
        },
    },
    tokens: {
        whiteSpace: "nowrap",
        [tiny]: {
            display: "none",
        },
    },
});
