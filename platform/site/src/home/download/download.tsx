import { createSignal, For, onSettled, Show } from "@destack/view";
import * as stylex from "@destack/style";
import { fontFamily } from "@destack/theme/tokens.stylex";

import { tokens } from "../../style/tokens.stylex";
import { type Download, readDownloads, selectDownload } from "./catalog.ts";

/** Render the desktop download cell and its platform choices. */
export function DownloadCell(properties: { style?: stylex.Styles }) {
    // hold the loaded downloads and the choice
    const [downloads, setDownloads] = createSignal<Download[]>([]);
    const [selected, setSelected] = createSignal<Download>();
    const [error, setError] = createSignal<string>();
    let choices!: HTMLDetailsElement;

    // load platform choices without delaying the rest of the landing page
    onSettled(() => {
        readDownloads()
            .then((downloads) => {
                setDownloads(downloads);
                setSelected(selectDownload(downloads, navigator.userAgent));
            })
            .catch((error) => {
                console.error(error);
                setError("Downloads unavailable. Please try again.");
            });
    });

    return (
        <div {...stylex.attrs(styles.root, properties.style)}>
            <Show
                when={selected()}
                fallback={
                    <button
                        type="button"
                        {...stylex.attrs(styles.action)}
                        onClick={() => {
                            choices.open = !choices.open;
                            choices.querySelector("summary")?.focus();
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
                        Download for {download().label.split(" · ")[0]}
                    </a>
                )}
            </Show>
            <details
                ref={choices}
                {...stylex.attrs(styles.choices)}
                onKeyDown={(event) => {
                    if (event.key === "Escape") {
                        choices.open = false;
                        choices.querySelector("summary")?.focus();
                    }
                }}
                onFocusOut={(event) => {
                    if (!choices.contains(event.relatedTarget as Node | null)) {
                        choices.open = false;
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
                                    choices.open = false;
                                }}
                            >
                                <SystemIcon target={download.target} />
                                {download.label}
                                <span {...stylex.attrs(styles.version)}>{download.version}</span>
                            </a>
                        )}
                    </For>
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
        if (properties.target?.includes("apple")) {
            return "M12.15 6.9c-.95 0-2.42-1.08-3.96-1.04-2.04.03-3.91 1.18-4.96 3.01-2.12 3.68-.55 9.1 1.52 12.09 1.01 1.45 2.21 3.09 3.79 3.04 1.52-.07 2.09-.99 3.94-.99 1.83 0 2.35.99 3.96.95 1.64-.03 2.68-1.48 3.68-2.95 1.16-1.69 1.64-3.33 1.66-3.42-.04-.01-3.18-1.22-3.22-4.86-.03-3.04 2.48-4.49 2.6-4.56-1.43-2.09-3.62-2.32-4.39-2.38-2-.16-3.68 1.09-4.61 1.09zM15.53 3.83c.84-1.01 1.4-2.43 1.25-3.83-1.21.05-2.66.8-3.53 1.82-.78.9-1.46 2.34-1.27 3.71 1.34.1 2.72-.69 3.56-1.7";
        }
        // windows
        else if (properties.target?.includes("windows")) {
            return "M0 3.45 9.75 2.1v9.45H0m10.95-9.6L24 0v11.4H10.95M0 12.6h9.75v9.45L0 20.7m10.95-8.1H24V24l-13.05-1.8";
        }
        // linux
        else if (properties.target?.includes("linux")) {
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
    },

    action: {
        alignItems: "center",
        backgroundColor: "transparent",
        borderWidth: 0,
        color: "inherit",
        cursor: "pointer",
        display: "flex",
        flexGrow: 1,
        fontFamily: fontFamily.default,
        fontSize: "0.9375rem",
        fontWeight: 600,
        gap: "0.5rem",
        justifyContent: "center",
        paddingInlineEnd: 0,
        paddingInlineStart: "1.75rem",
        position: "relative",
        zIndex: 1,
    },
    system: {
        flexShrink: 0,
        height: "1.125rem",
        width: "1.125rem",
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
        backgroundColor: tokens.cream,
        borderColor: tokens.signalInk,
        borderStyle: "solid",
        borderWidth: "2px",
        boxShadow: `4px 4px 0 ${tokens.signal}`,
        color: tokens.signalInk,
        left: 0,
        position: "absolute",
        right: 0,
        top: "calc(100% + 0.75rem)",
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
        borderBottomWidth: 0,
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
