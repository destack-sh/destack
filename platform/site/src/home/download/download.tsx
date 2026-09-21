import { createSignal, For, type JSX, onSettled, Show } from "@destack/view";
import * as style from "@destack/style";
import { fontFamily } from "@destack/theme/tokens.stylex";
import { tokens } from "../../style/tokens.stylex";
import { type Download as Distribution, readDownloads, selectDownload } from "./catalog.ts";

/** Render the desktop download and platform choices. */
export function Download(props: { children: JSX.Element }) {
    const [downloads, setDownloads] = createSignal<Distribution[]>([]);
    const [selected, setSelected] = createSignal<Distribution>();
    const [error, setError] = createSignal("");
    let choices: HTMLDetailsElement | undefined;

    /** Show the selected version or the common release across all platforms. */
    const version = () => {
        if (selected()) {
            return selected()!.version;
        }
        const first = downloads()[0]?.version;

        return downloads().every((download) => download.version === first) ? first : undefined;
    };

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
        <div {...style.attrs(styles.container)}>
            <div {...style.attrs(styles.bar)}>
                {props.children}
                <div {...style.attrs(styles.button)}>
                    <Show
                        when={selected()}
                        fallback={
                            <button
                                type="button"
                                {...style.attrs(styles.action)}
                                onClick={() => {
                                    if (choices) {
                                        choices.open = !choices.open;
                                        choices.querySelector("summary")?.focus();
                                    }
                                }}
                            >
                                Download
                            </button>
                        }
                    >
                        {(download) => (
                            <a
                                href={download().url}
                                title={`Download for ${download().label}`}
                                {...style.attrs(styles.action)}
                            >
                                Download
                            </a>
                        )}
                    </Show>
                    <details
                        ref={choices}
                        {...style.attrs(styles.choices)}
                        onKeyDown={(event) => {
                            if (event.key === "Escape" && choices) {
                                choices.open = false;
                                choices.querySelector("summary")?.focus();
                            }
                        }}
                        onFocusOut={(event) => {
                            if (choices && !choices.contains(event.relatedTarget as Node | null)) {
                                choices.open = false;
                            }
                        }}
                    >
                        <summary aria-label="Choose a platform" {...style.attrs(styles.toggle)}>
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
                        <div {...style.attrs(styles.menu)}>
                            <For each={downloads()}>
                                {(download) => (
                                    <a
                                        href={download.url}
                                        {...style.attrs(styles.option)}
                                        onClick={() => {
                                            setSelected(download);
                                            if (choices) {
                                                choices.open = false;
                                            }
                                        }}
                                    >
                                        {download.label}
                                    </a>
                                )}
                            </For>
                            <Show when={!downloads().length}>
                                <span role="status" {...style.attrs(styles.message)}>
                                    {error() || "Loading downloads…"}
                                </span>
                            </Show>
                        </div>
                    </details>
                </div>
            </div>
            <div {...style.attrs(styles.version)}>{version()}</div>
        </div>
    );
}

/** Download button and platform menu styles. */
const styles = style.create({
    container: { position: "relative", zIndex: 2, width: "fit-content", maxWidth: "100%" },
    bar: {
        display: "flex",
        alignItems: "stretch",
        borderWidth: "1px",
        borderStyle: "solid",
        borderColor: "#45606a",
        borderRadius: "0.2rem",
        "@media (max-width: 767px)": { flexDirection: "column-reverse" },
    },
    version: {
        minHeight: "1.2em",
        marginTop: "0.65rem",
        fontSize: "0.75rem",
        fontFamily: fontFamily.code,
        color: "#b5c6ca",
        fontVariantNumeric: "tabular-nums",
    },
    button: {
        display: "flex",
        backgroundColor: "#ff792e",
        color: tokens.night,
        borderTopRightRadius: "0.15rem",
        borderBottomRightRadius: "0.15rem",
        flexShrink: 0,
        "@media (max-width: 767px)": { borderTopLeftRadius: "0.15rem", borderBottomRightRadius: 0 },
    },
    action: {
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        flexGrow: 1,
        padding: "0.95rem 1.25rem",
        color: "inherit",
        backgroundColor: "transparent",
        borderWidth: 0,
        font: "inherit",
        fontFamily: fontFamily.default,
        fontSize: "1rem",
        fontWeight: 700,
        whiteSpace: "nowrap",
        textDecoration: "none",
        cursor: "pointer",
        ":hover": { backgroundColor: "#ffffff18" },
    },
    choices: { position: "relative", flexShrink: 0 },
    toggle: {
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        height: "100%",
        width: "2.25rem",
        borderLeftWidth: "1px",
        borderLeftStyle: "solid",
        borderLeftColor: "#12313c35",
        cursor: "pointer",
        listStyle: "none",
        "::marker": { content: "''" },
        "::-webkit-details-marker": { display: "none" },
        ":hover": { backgroundColor: "#ffffff25" },
    },
    menu: {
        position: "absolute",
        bottom: "calc(100% + 0.65rem)",
        right: 0,
        width: "15rem",
        padding: "0.35rem",
        backgroundColor: "#f1eadb",
        color: tokens.night,
        borderRadius: "0.4rem",
        boxShadow: "0 6px 24px #0004",
        textAlign: "left",
    },
    option: {
        display: "block",
        padding: "0.65rem 0.8rem",
        color: "inherit",
        textDecoration: "none",
        borderRadius: "0.2rem",
        ":hover": { backgroundColor: "#12313c12" },
    },
    message: { display: "block", padding: "0.65rem 0.8rem", fontSize: "0.85rem" },
});
