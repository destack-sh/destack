import { color, fontFamily } from "@destack/theme/tokens.stylex";
import { onSettled } from "@destack/view";
import * as stylex from "@destack/style";

import { commandEvents } from "../command/command";
import type { PageSource } from "../content/source";

/// Properties for the page source controls.
type SourceActionsProps = {
    /// The shared page source commands.
    commands: PageSourceCommands;
};

/// One portable source format.
type SourceKind = "md" | "txt";

/// The source commands shared by responsive reader controls.
export type PageSourceCommands = {
    /// Copy one source format.
    copy: (kind: SourceKind) => void;

    /// The current page source files.
    source: PageSource;
};

/// Create source commands shared by responsive article controls.
export function createPageSourceCommands(source: PageSource): PageSourceCommands {
    const write = async (kind: SourceKind) => {
        // load the requested source format
        const route = kind === "md" ? source.markdownRoute : source.textRoute;
        const response = await fetch(route);

        // surface unavailable generated sources
        if (!response.ok) {
            throw new Error(`failed to load ${route}: ${response.status}`);
        }

        // publish the requested source
        await navigator.clipboard.writeText(await response.text());
    };

    const copy = (kind: SourceKind) => {
        // report clipboard failures through the browser console
        void write(kind).catch((error: unknown) => {
            console.error(error);
        });
    };

    onSettled(() => {
        // bind command palette copy actions to this page
        const copyMarkdown = () => copy("md");
        const copyText = () => copy("txt");

        document.addEventListener(commandEvents.copyMarkdown, copyMarkdown);
        document.addEventListener(commandEvents.copyText, copyText);

        // unbind page actions when the reader is replaced
        return () => {
            document.removeEventListener(commandEvents.copyMarkdown, copyMarkdown);
            document.removeEventListener(commandEvents.copyText, copyText);
        };
    });

    return { copy, source };
}

/// Render source links shared by articles and documentation.
export function SourceActions(props: SourceActionsProps) {
    const commands = props.commands;

    return (
        <nav aria-label="Page formats" {...stylex.attrs(styles.controls)}>
            <a
                {...stylex.attrs(styles.action)}
                href={commands.source.markdownRoute}
                rel="alternate noopener"
                target="_blank"
                type="text/markdown"
            >
                md
            </a>
            <a
                {...stylex.attrs(styles.action)}
                href={commands.source.textRoute}
                rel="alternate noopener"
                target="_blank"
                type="text/plain"
            >
                txt
            </a>
        </nav>
    );
}

const styles = stylex.create({
    action: {
        color: color.foreground,
        textDecoration: "none",
        ":hover": {
            color: color.primary,
        },
    },
    controls: {
        alignItems: "baseline",
        display: "flex",
        fontFamily: fontFamily.default,
        fontSize: "var(--size-navigation)",
        fontWeight: 600,
        gap: "1rem",
        letterSpacing: "0.02em",
        minWidth: 0,
    },
});
