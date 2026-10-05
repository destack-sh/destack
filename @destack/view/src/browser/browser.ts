import "@destack/theme/theme.css";
import { Catalog } from "@destack/locale";
import type { ObjectType } from "@destack/object";
import { BrowserTab } from "@destack/object/browser";
import { ViewScope } from "@destack/package/manifest";
import { reportToDevtools, startTelemetry } from "@destack/telemetry/browser";
import { OtlpExporter } from "@destack/telemetry/otlp";
import {
    type CatalogReference,
    DISPLAY_PATH,
    type ViewContext,
    ViewDisplay,
    ViewLaunch,
} from "../declare/context.ts";
import type { View } from "../declare/view.ts";
import { renderView } from "../page/mount.ts";
import { mountServices } from "../page/view.ts";

/** How long to wait before following the display again after its stream drops. */
const RECONNECT_MILLISECONDS = 1000;

/** The identifier of the element with the page's launch. */
const LAUNCH_ELEMENT = "destack-view";

/** Mount a view into the page serving it once its scopes' objects are open, returning how to unmount it. */
export async function mount(view: View): Promise<() => Promise<void>> {
    // read the page's launch
    const element = document.getElementById(LAUNCH_ELEMENT);
    if (element === null) {
        throw new TypeError(`the page has no ${LAUNCH_ELEMENT} launch`);
    }
    const { release, endpoint, manifest, catalogs, ...context } = ViewLaunch.parse(
        JSON.parse(element.textContent),
    );

    // export the page's telemetry and uncaught failures to its origin as the release serving it
    const exporter = OtlpExporter.origin(reportToDevtools);
    const telemetry = await startTelemetry(
        exporter.options(
            { name: view.package.name, version: release },
            manifest === undefined ? {} : { manifest },
        ),
    );

    // open the view's scopes over the launch's endpoint while fetching its catalogs
    const [tabs, translations] = await Promise.all([
        openTabs(context, endpoint, view),
        fetchCatalogs(catalogs),
    ]);
    const clients = Object.fromEntries([...tabs].map(([scope, tab]) => [scope, tab.client]));
    const services = mountServices(view.services, location.origin);

    // render the root component under the view, its clients and the launch's catalogs
    const { default: Component } = await view.component();
    let dispose = renderView(Component, document.body, context, clients, translations, services);

    // restyle the page on each display change and render again in a new language
    let locale = context.locale;
    let switches = 0;
    const stopping = new AbortController();
    void followDisplay(stopping.signal, (next) => {
        // restyle in place and keep the language when it stays
        restyle(next);
        if (next.locale === locale) {
            return;
        }

        // render in the latest language once its catalogs arrive, reporting a failed fetch
        locale = next.locale;
        switches += 1;
        const switched = switches;
        fetchCatalogs(next.catalogs).then(
            (fetched) => {
                // drop a switch a later one replaced
                if (switched !== switches || stopping.signal.aborted) {
                    return;
                }
                dispose();
                dispose = renderView(
                    Component,
                    document.body,
                    { ...context, locale: next.locale },
                    clients,
                    fetched,
                    services,
                );
            },
            (error: unknown) => reportError(error),
        );
    });

    // release the page's display, render, tabs and telemetry on unmount
    const unmount = async () => {
        stopping.abort();
        dispose();
        try {
            await Promise.all([...tabs.values()].map((tab) => tab.close()));
        } finally {
            await telemetry.shutdown();
        }
    };

    // unmount once the page goes
    window.addEventListener("pagehide", () => void unmount(), { once: true });

    return unmount;
}

/** Follow the host's display events until the signal aborts, reconnecting after a dropped stream. */
async function followDisplay(
    signal: AbortSignal,
    receive: (display: ViewDisplay) => void,
): Promise<void> {
    while (!signal.aborted) {
        try {
            // stop once the host refuses the page's credential
            const response = await fetch(new URL(DISPLAY_PATH, location.origin), { signal });
            if (response.status === 401 || response.status === 403) {
                return;
            }

            // read the event stream's data lines as displays
            const lines = (response.body ?? new ReadableStream<Uint8Array>()).pipeThrough(
                lineDecoder(),
            );
            for await (const line of lines) {
                if (line.startsWith("data: ")) {
                    receive(ViewDisplay.parse(JSON.parse(line.slice("data: ".length))));
                }
            }
        } catch (error) {
            if (signal.aborted) {
                return;
            }
            reportError(error);
        }

        // wait before reconnecting
        await new Promise((resolve) => {
            setTimeout(resolve, RECONNECT_MILLISECONDS);
        });
    }
}

/** Decode a byte stream into its lines. */
function lineDecoder(): TransformStream<Uint8Array, string> {
    const decoder = new TextDecoder();
    let rest = "";

    return new TransformStream({
        transform(chunk, controller) {
            // emit each complete line and keep the incomplete end
            const lines = (rest + decoder.decode(chunk, { stream: true })).split("\n");
            rest = lines.pop() ?? "";
            for (const line of lines) {
                controller.enqueue(line);
            }
        },
    });
}

/** Apply a display's theme and language to the page, dropping the custom properties it no longer sets. */
function restyle(display: ViewDisplay): void {
    // replace the root's custom properties and color scheme
    const root = document.documentElement;
    for (let index = root.style.length - 1; index >= 0; index--) {
        const property = root.style.item(index);
        if (!(property in display.style)) {
            root.style.removeProperty(property);
        }
    }
    for (const [property, value] of Object.entries(display.style)) {
        root.style.setProperty(property, value);
    }

    // follow the color scheme in the page's metadata and the language in its root
    document
        .querySelector('meta[name="color-scheme"]')
        ?.setAttribute("content", display.style["color-scheme"] ?? "light dark");
    root.lang = display.locale;
}

/** Fetch the catalogs a launch names. */
async function fetchCatalogs(references: readonly CatalogReference[]): Promise<Catalog[]> {
    return Promise.all(
        references.map(async (reference) => {
            // fetch the catalog from the view's origin
            const response = await fetch(new URL(reference.url, location.origin));
            if (!response.ok) {
                throw new TypeError(`the catalog at ${reference.url} answered ${response.status}`);
            }

            // require the package and language the launch names
            const catalog = Catalog.parse(await response.json());
            if (catalog.package !== reference.package || catalog.locale !== reference.locale) {
                throw new TypeError(
                    `the catalog at ${reference.url} is not the one the launch names`,
                );
            }

            return catalog;
        }),
    );
}

/** Open a tab for each scope a view requests permissions in over the page's endpoint, each once its tables exist. */
async function openTabs(
    context: ViewContext,
    endpoint: string,
    view: View,
): Promise<Map<string, BrowserTab>> {
    // group the object types by the scope each opens in, merging a home that is the view's space
    const byScope = new Map<string, Readonly<Record<string, ObjectType>>>();
    for (const name of ViewScope.options) {
        const scope = context[name];
        const objects = view.opens(name);
        if (scope !== undefined && objects.length > 0) {
            const named = Object.fromEntries(objects.map((object) => [object.name, object]));
            byScope.set(scope, { ...byScope.get(scope), ...named });
        }
    }

    // open each scope's tab over the origin's endpoint
    const url = new URL(endpoint, location.origin).href;

    return new Map(
        await Promise.all(
            [...byScope].map(async ([scope, objects]) => {
                const tab = await BrowserTab.open({
                    name: scope,
                    objects,
                    scope,
                    caller: context.user,
                    endpoint: { url },
                    // reopen the same endpoint
                    reconnect: () => ({ url }),
                    report: reportError,
                });
                await tab.ready;

                return [scope, tab] as const;
            }),
        ),
    );
}
