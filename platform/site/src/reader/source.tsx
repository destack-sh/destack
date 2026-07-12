import { type Accessor, createSignal, onCleanup, onMount } from "solid-js";

import { commandEvents } from "../command/command";
import type { PageSource } from "../content/source";

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
        <div class="source-controls">
            <nav aria-label="Page formats" class="source-actions">
                <a
                    href={commands.source.markdownRoute}
                    rel="alternate noopener"
                    target="_blank"
                    type="text/markdown"
                >
                    [source]
                </a>
                <SourceLinks commands={commands} />
                <span class="source-actions__tokens">{commands.tokenLabel()}</span>
            </nav>

            <details class="source-menu" name="reader-tools">
                <summary>
                    <span>[source]</span>
                    <span class="source-actions__tokens">{commands.tokenLabel()}</span>
                </summary>
                <nav aria-label="Page formats" class="source-menu__body">
                    <span class="source-menu__tokens">{commands.tokenLabel()}</span>
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
            <a
                href={commands.source.markdownRoute}
                rel="alternate noopener"
                target="_blank"
                type="text/markdown"
            >
                [.md]
            </a>
            <a
                href={commands.source.textRoute}
                rel="alternate noopener"
                target="_blank"
                type="text/plain"
            >
                [.txt]
            </a>
            <button onClick={() => commands.copy("md")} type="button">
                [{commands.state() === "md" ? "copied" : "copy .md"}]
            </button>
            <button onClick={() => commands.copy("txt")} type="button">
                [{commands.state() === "txt" ? "copied" : "copy .txt"}]
            </button>
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
