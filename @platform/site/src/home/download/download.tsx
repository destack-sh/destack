import { createSignal, For, onSettled, Show } from "@destack/view";
import * as stylex from "@destack/style";
import { color, text } from "@destack/theme/tokens.stylex";

import { tokens } from "../../style/tokens.stylex";
import { type Download, readDownloads, selectDownload } from "./catalog.ts";
import { telemetry } from "@destack/telemetry";
import { log } from "../../site/telemetry.ts";

/** The one request for the published downloads that every download control on the page shares. */
let published: Promise<Download[]> | undefined;

/** Load the published downloads after the page settles, and choose the one for this machine. */
export function createDownloads() {
    // hold the loaded downloads, the choice and a failure to load them
    const [downloads, setDownloads] = createSignal<Download[]>([]);
    const [selected, setSelected] = createSignal<Download>();
    const [error, setError] = createSignal<string>();

    // load platform choices without delaying the rest of the landing page
    onSettled(() => {
        published ??= readDownloads();
        published
            .then((catalog) => {
                setDownloads(catalog);
                setSelected(selectDownload(catalog, navigator.userAgent));
            })
            .catch((failure: unknown) => {
                log.error("download.list.failed", telemetry.exceptionAttributes(failure));
                setError("Downloads unavailable. Please try again.");
            });
    });

    return { downloads, selected, setSelected, error };
}

/** Render the desktop download cell and its platform choices. */
export function DownloadCell(properties: { style?: stylex.Styles }) {
    // hold the loaded downloads and the choice
    const { downloads, selected, setSelected, error } = createDownloads();
    let choices: HTMLDetailsElement | undefined;
    const renderedChoices = () => {
        if (!choices) {
            throw new TypeError("the download rendered without its choices");
        }

        return choices;
    };

    return (
        <div {...stylex.attrs(styles.root, properties.style)}>
            <Show
                when={selected()}
                fallback={
                    <button
                        type="button"
                        {...stylex.attrs(styles.action)}
                        onClick={() => {
                            const details = renderedChoices();
                            details.open = !details.open;
                            details.querySelector("summary")?.focus();
                        }}
                    >
                        <SystemIcon target={undefined} />
                        Download
                    </button>
                }
            >
                {(download) => (
                    <a
                        href={download().url}
                        title={`Download for ${download().label} (${download().version})`}
                        {...stylex.attrs(styles.action)}
                    >
                        <SystemIcon target={download().target} />
                        Download
                    </a>
                )}
            </Show>
            <details
                ref={choices}
                {...stylex.attrs(styles.choices)}
                onKeyDown={(event) => {
                    if (event.key === "Escape") {
                        event.currentTarget.open = false;
                        event.currentTarget.querySelector("summary")?.focus();
                    }
                }}
                onFocusOut={(event) => {
                    const next = event.relatedTarget;
                    if (!(next instanceof Node) || !event.currentTarget.contains(next)) {
                        event.currentTarget.open = false;
                    }
                }}
            >
                <summary aria-label="Choose a platform" {...stylex.attrs(styles.toggle)}>
                    <svg
                        width="12"
                        height="12"
                        viewBox="0 0 16 16"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="1.5"
                        aria-hidden="true"
                    >
                        <path d="m4 6 4 4 4-4" />
                    </svg>
                </summary>
                <div {...stylex.attrs(styles.menu)}>
                    <For each={downloads()}>
                        {(download) => (
                            <a
                                href={download.url}
                                {...stylex.attrs(styles.option)}
                                onClick={() => {
                                    setSelected(download);
                                    renderedChoices().open = false;
                                }}
                            >
                                <SystemIcon target={download.target} />
                                {download.label}
                                <span {...stylex.attrs(styles.version)}>{download.version}</span>
                            </a>
                        )}
                    </For>
                    <a
                        href="/docs/setup/#ask-your-agent"
                        {...stylex.attrs(styles.option, styles.guide)}
                    >
                        Ask your agent
                        <span aria-hidden="true" {...stylex.attrs(styles.version)}>
                            ↗
                        </span>
                    </a>
                    <a href="/docs/setup/" {...stylex.attrs(styles.option, styles.guide)}>
                        Installation guide
                        <span aria-hidden="true" {...stylex.attrs(styles.version)}>
                            ↗
                        </span>
                    </a>
                    <Show when={!downloads().length}>
                        <span role="status" {...stylex.attrs(styles.message)}>
                            <Show when={error()} fallback="Loading downloads…">
                                {(message) => <>{message()}</>}
                            </Show>
                        </span>
                    </Show>
                </div>
            </details>
        </div>
    );
}

/** Draw the logo of the operating system a download targets. */
function SystemIcon(properties: { target: Download["target"] | undefined }) {
    // pick the vendor mark, or a plain download arrow when no platform is chosen
    const path = () => {
        // apple
        if (properties.target?.includes("apple") === true) {
            return "M12.15 6.9c-.95 0-2.42-1.08-3.96-1.04-2.04.03-3.91 1.18-4.96 3.01-2.12 3.68-.55 9.1 1.52 12.09 1.01 1.45 2.21 3.09 3.79 3.04 1.52-.07 2.09-.99 3.94-.99 1.83 0 2.35.99 3.96.95 1.64-.03 2.68-1.48 3.68-2.95 1.16-1.69 1.64-3.33 1.66-3.42-.04-.01-3.18-1.22-3.22-4.86-.03-3.04 2.48-4.49 2.6-4.56-1.43-2.09-3.62-2.32-4.39-2.38-2-.16-3.68 1.09-4.61 1.09zM15.53 3.83c.84-1.01 1.4-2.43 1.25-3.83-1.21.05-2.66.8-3.53 1.82-.78.9-1.46 2.34-1.27 3.71 1.34.1 2.72-.69 3.56-1.7";
        }
        // linux
        else if (properties.target?.includes("linux") === true) {
            return "M4 4.5h16v11H4Zm-2 14h20v1.5H2Z";
        }
        // no platform chosen yet: a plain download arrow
        else {
            return "M11 3h2v10.2l3.6-3.6 1.4 1.4-6 6-6-6 1.4-1.4 3.6 3.6ZM4 19h16v2H4Z";
        }
    };

    return (
        <svg aria-hidden="true" viewBox="0 0 24 24" {...stylex.attrs(styles.system)}>
            <path d={path()} fill="currentColor" />
        </svg>
    );
}

/** Download cell and platform menu styles. */
const styles = stylex.create({
    root: {
        backgroundColor: tokens.signal,
        color: tokens.signalInk,
        display: "flex",
        position: "relative",
    },

    action: {
        alignItems: "center",
        backgroundColor: "transparent",
        borderWidth: 0,
        color: "inherit",
        cursor: "pointer",
        display: "flex",
        flexGrow: 1,
        fontFamily: text.family,
        fontSize: "0.9375rem",
        fontWeight: 600,
        gap: "0.5rem",
        justifyContent: "center",
        paddingInlineEnd: 0,
        paddingInlineStart: "1.75rem",
        position: "relative",
        zIndex: 1,
        ":hover > svg": { translate: "0 2px" },
        ":active > svg": { translate: "0 5px" },
    },
    system: {
        flexShrink: 0,
        height: "1.125rem",
        transition: "translate 160ms cubic-bezier(0.3, 1.6, 0.5, 1)",
        width: "1.125rem",
        "@media (prefers-reduced-motion: reduce)": { transition: "none" },
    },
    choices: {
        display: "flex",
    },
    toggle: {
        alignItems: "center",
        cursor: "pointer",
        position: "relative",
        zIndex: 1,
        display: "flex",
        justifyContent: "center",
        listStyle: "none",
        width: "1.75rem",
        "::marker": { content: "''" },
        ":hover": { backgroundColor: "#ffffff26" },
    },
    menu: {
        backgroundColor: color.popover,
        borderColor: tokens.rule,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: tokens.hairline,
        boxShadow: "0 12px 32px rgb(8 23 35 / 14%)",
        color: color.popoverForeground,
        minWidth: "18rem",
        position: "absolute",
        right: 0,
        top: "calc(100% + 0.5rem)",
        zIndex: 30,
    },
    option: {
        alignItems: "center",
        borderBottomColor: tokens.signalInk,
        borderBottomStyle: "solid",
        borderBottomWidth: "1.5px",
        color: "inherit",
        display: "flex",
        fontSize: "1rem",
        fontWeight: 600,
        gap: "0.75rem",
        paddingBlock: "0.875rem",
        paddingInline: "1rem",
        ":hover": { backgroundColor: tokens.signal },
    },
    guide: {
        ":last-of-type": { borderBottomWidth: 0 },
        color: `color-mix(in srgb, ${tokens.signalInk} 65%, transparent)`,
    },
    version: {
        color: `color-mix(in srgb, ${tokens.signalInk} 65%, transparent)`,
        fontFamily: tokens.monoFont,
        fontSize: "0.75rem",
        marginLeft: "auto",
    },
    message: {
        color: `color-mix(in srgb, ${tokens.signalInk} 65%, transparent)`,
        display: "block",
        fontSize: "0.875rem",
        paddingBlock: "0.75rem",
        paddingInline: tokens.inset,
    },
});
