import { media } from "@destack/style/media.stylex";
import { createSignal, onSettled, Show } from "@destack/view";
import * as style from "@destack/style";
import { color, font } from "@destack/theme/tokens.stylex";

import { type Download, readDownloads, selectDownload } from "./catalog.ts";
import { captureException } from "../../../telemetry.ts";

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
                captureException(failure, { tags: { feature: "download" } });
                setError("Downloads unavailable. Please try again.");
            });
    });

    return { downloads, selected, setSelected, error };
}

/** Render the desktop download link. */
export function DownloadCell(properties: { xstyle?: style.Styles }) {
    const { selected } = createDownloads();

    return (
        <div {...style.attrs(styles.root, properties.xstyle)}>
            <Show
                when={selected()}
                fallback={
                    <a href="/docs/setup/" {...style.attrs(styles.action)}>
                        <SystemIcon target={undefined} />
                        Download
                    </a>
                }
            >
                {(download) => (
                    <a
                        href={download().url}
                        title={`Download for ${download().label} (${download().version})`}
                        {...style.attrs(styles.action)}
                    >
                        <SystemIcon target={download().target} />
                        Download
                    </a>
                )}
            </Show>
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
        <svg aria-hidden="true" viewBox="0 0 24 24" {...style.attrs(styles.system)}>
            <path d={path()} fill="currentColor" />
        </svg>
    );
}

/** Download cell and platform menu styles. */
const styles = style.create({
    root: {
        backgroundColor: color.primary,
        color: color.primaryForeground,
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
        fontFamily: font.text,
        fontSize: "0.9375rem",
        fontWeight: 600,
        gap: "0.5rem",
        justifyContent: "center",
        paddingInline: 0,
        position: "relative",
        zIndex: 1,
        ":hover > svg": { translate: "0 2px" },
        ":active > svg": { translate: "0 5px" },
    },
    system: {
        flexShrink: 0,
        height: "1.125rem",
        transition: {
            default: "translate 160ms cubic-bezier(0.3, 1.6, 0.5, 1)",
            [media.motionReduce]: "none",
        },
        width: "1.125rem",
    },
});
