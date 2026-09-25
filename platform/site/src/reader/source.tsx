import { color, fontFamily } from "@destack/theme/tokens.stylex";
import { onSettled } from "@destack/view";
import * as stylex from "@destack/style";

import { commandEvents } from "../command/command";
import type { PageSource } from "../content/source";
import { sound } from "../effect/sound";

/** Properties for the page source controls. */
type SourceActionsProperties = {
    /** The shared page source commands. */
    commands: PageSourceCommands;
};

/** One portable source format. */
type SourceKind = "md" | "txt";

/** The source commands shared by responsive reader controls. */
export type PageSourceCommands = {
    /** Copy one source format. */
    copy: (kind: SourceKind) => void;

    /** The current page source files. */
    source: PageSource;
};

/** Create source commands shared by responsive article controls. */
export function createPageSourceCommands(source: PageSource): PageSourceCommands {
    // copy one source format to the clipboard
    const write = async (kind: SourceKind) => {
        // load the requested source format
        const route = kind === "md" ? source.markdownRoute : source.textRoute;
        const response = await fetch(route);

        // fail on unavailable generated sources
        if (!response.ok) {
            throw new Error(`failed to load ${route}: ${response.status}`);
        }

        // publish the requested source
        await navigator.clipboard.writeText(await response.text());
        sound.play("copy");
    };

    // copy in the background, logging failures
    const copy = (kind: SourceKind) => {
        // report clipboard failures through the browser console
        void write(kind).catch((error: unknown) => {
            console.error(error);
        });
    };

    // bind the palette copy commands while the page shows
    onSettled(() => {
        // bind command palette copy actions to this page
        const copyMarkdown = () => copy("md");
        const copyText = () => copy("txt");

        // listen for the palette commands
        document.addEventListener(commandEvents.copyMarkdown, copyMarkdown);
        document.addEventListener(commandEvents.copyText, copyText);

        return () => {
            // unbind page actions when the reader is replaced
            document.removeEventListener(commandEvents.copyMarkdown, copyMarkdown);
            document.removeEventListener(commandEvents.copyText, copyText);
        };
    });

    return { copy, source };
}

/** Render source links shared by articles and documentation. */
export function SourceActions(properties: SourceActionsProperties) {
    const commands = properties.commands;

    return (
        <nav aria-label="Page formats" {...stylex.attrs(styles.controls)}>
            <a
                {...stylex.attrs(styles.action)}
                href={commands.source.markdownRoute}
                rel="alternate noopener"
                target="_blank"
                title="Open as Markdown"
                aria-label="Open as Markdown"
                type="text/markdown"
            >
                <svg aria-hidden="true" viewBox="0 0 22 16" {...stylex.attrs(styles.icon)}>
                    <rect x="0.75" y="0.75" width="20.5" height="14.5" rx="2" />
                    <path d="M4 11.5v-7l2.75 3.5 2.75-3.5v7M15.5 4.5v6.5M13 8.75l2.5 2.5 2.5-2.5" />
                </svg>
            </a>
            <a
                {...stylex.attrs(styles.action)}
                href={commands.source.textRoute}
                rel="alternate noopener"
                target="_blank"
                title="Open as plain text"
                aria-label="Open as plain text"
                type="text/plain"
            >
                <svg aria-hidden="true" viewBox="0 0 16 16" {...stylex.attrs(styles.icon)}>
                    <path d="M3.5 1.75h6.25l3 3v9.5H3.5ZM9.5 1.75v3.25h3.25M5.75 8h4.5M5.75 10.75h4.5" />
                </svg>
            </a>
        </nav>
    );
}

/** The source action styles. */
const styles = stylex.create({
    action: {
        color: color.foreground,
        display: "flex",
        textDecoration: "none",
        ":hover": {
            color: color.primary,
        },
    },
    icon: {
        fill: "none",
        height: "1rem",
        stroke: "currentColor",
        strokeLinecap: "round",
        strokeLinejoin: "round",
        strokeWidth: 1.5,
        width: "auto",
    },
    controls: {
        alignItems: "center",
        display: "flex",
        fontFamily: fontFamily.default,
        fontSize: "var(--size-navigation)",
        fontWeight: 600,
        gap: "1rem",
        letterSpacing: "0.02em",
        minWidth: 0,
    },
});
