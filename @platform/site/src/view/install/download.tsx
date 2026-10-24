import { createSignal, onSettled, useTelemetry } from "@destack/view";

import {
    type Download,
    readDownloads,
    selectDownload,
    type System,
    systemOfAgent,
} from "./catalog.ts";

/** The one request for the published downloads that every download control on the page shares. */
let published: Promise<Download[]> | undefined;

/** Read this machine's system and load the published downloads once the page settles. */
export function createDownloads() {
    // hold this machine's system, the loaded downloads, the choice and a failure to load them
    const [system, setSystem] = createSignal<System>();
    const [downloads, setDownloads] = createSignal<Download[]>([]);
    const [selected, setSelected] = createSignal<Download>();
    const [error, setError] = createSignal<string>();
    const { captureException } = useTelemetry();

    // load platform choices without delaying the rest of the landing page
    onSettled(() => {
        setSystem(systemOfAgent(navigator.userAgent));
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

    return { system, downloads, selected, setSelected, error };
}
