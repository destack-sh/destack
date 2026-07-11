import { createSignal, onCleanup, onMount } from "solid-js";

import { commandEvents } from "../command/command";
import type { PageSource } from "../content/source";

type SourceActionsProps = {
    /// The current page source files.
    source: PageSource;
};

/// The latest source-copy outcome.
type SourceState = "" | "error" | "md" | "txt";

/// Render source links and copy controls shared by articles and documentation.
export function SourceActions(props: SourceActionsProps) {
    const [state, setState] = createSignal<SourceState>("");
    let reset: ReturnType<typeof setTimeout> | undefined;

    onCleanup(() => clearTimeout(reset));

    const copy = async (kind: "md" | "txt") => {
        const route = kind === "md" ? props.source.markdownRoute : props.source.textRoute;
        const response = await fetch(route);
        if (!response.ok) {
            throw new Error(`failed to load ${route}: ${response.status}`);
        }

        await navigator.clipboard.writeText(await response.text());
        setState(kind);
        clearTimeout(reset);
        reset = setTimeout(() => setState(""), 1600);
    };

    const copyMaybe = (kind: "md" | "txt") => {
        void copy(kind).catch((error: unknown) => {
            console.error(error);
            setState("error");
        });
    };

    onMount(() => {
        const copyMarkdown = () => copyMaybe("md");
        const copyText = () => copyMaybe("txt");

        document.addEventListener(commandEvents.copyMarkdown, copyMarkdown);
        document.addEventListener(commandEvents.copyText, copyText);
        onCleanup(() => {
            document.removeEventListener(commandEvents.copyMarkdown, copyMarkdown);
            document.removeEventListener(commandEvents.copyText, copyText);
        });
    });

    return (
        <nav
            aria-label="Page formats"
            class="source-actions"
            data-markdown-route={props.source.markdownRoute}
            data-page-source
            data-text-route={props.source.textRoute}
        >
            <a
                href={props.source.markdownRoute}
                rel="alternate noopener"
                target="_blank"
                type="text/markdown"
            >
                [source]
            </a>
            <a
                href={props.source.markdownRoute}
                rel="alternate noopener"
                target="_blank"
                type="text/markdown"
            >
                [.md]
            </a>
            <a
                href={props.source.textRoute}
                rel="alternate noopener"
                target="_blank"
                type="text/plain"
            >
                [.txt]
            </a>
            <button onClick={() => copyMaybe("md")} type="button">
                [{state() === "md" ? "copied" : "copy .md"}]
            </button>
            <button onClick={() => copyMaybe("txt")} type="button">
                [{state() === "txt" ? "copied" : "copy .txt"}]
            </button>
            <span class="source-actions__tokens">
                {state() === "error" ? "copy failed" : `~${formatTokens(props.source.tokens)} tokens`}
            </span>
        </nav>
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
