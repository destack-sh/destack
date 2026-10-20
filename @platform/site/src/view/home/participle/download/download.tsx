import { createSignal, onSettled } from "@destack/view";

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
