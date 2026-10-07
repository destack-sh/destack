import "@destack/theme/theme.css";
import { Catalog, Localization, SOURCE_LOCALE } from "@destack/locale";
import type { ObjectType } from "@destack/object";
import { BrowserTab } from "@destack/object/browser";
import { PermissionScope } from "@destack/access";
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
import { listenForCommands } from "../palette/overlay.ts";

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
    const { release, endpoint, catalogs, ...context } = ViewLaunch.parse(
        JSON.parse(element.textContent),
    );

    // export the page's telemetry and uncaught failures to its origin, which stamps the build it serves
    const exporter = OtlpExporter.origin(reportToDevtools);
    const telemetry = await startTelemetry(
        exporter.options({ name: view.package.name, version: release }),
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
    const rendering = new LocalizedRendering(
        (locale, translated) =>
            renderView(
                Component,
                document.body,
                locale === undefined ? context : { ...context, locale },
                clients,
                translated,
                services,
            ),
        context.locale,
        translations,
    );
    const stopping = new AbortController();

    // open the command palette, restyle on each display change and render again in a new language
    void listenForCommands(stopping.signal, () => rendering.localization).catch(reportError);
    void followDisplay(stopping.signal, (next) => {
        restyle(next);
        rendering.switch(next, stopping.signal);
    });

    // release the page's display, render, tabs and telemetry on unmount
    const unmount = async () => {
        stopping.abort();
        rendering.dispose();
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

/** A view rendered into the page, rendered again in each new language once that language's catalogs arrive. */
class LocalizedRendering {
    /** Render the view in a language with its catalogs, returning how to remove it. */
    readonly #render: (locale: ViewContext["locale"], catalogs: Catalog[]) => () => void;
    /** Remove the current rendering. */
    #dispose: () => void;
    /** The language rendered or being fetched. */
    #locale: ViewContext["locale"];
    /** The localization rendered, which the page's own overlays render their messages in. */
    #localization: Localization;
    /** The count of language switches, which drops a switch a later one replaced. */
    #switches = 0;

    /** Render the view in a language with its catalogs. */
    constructor(
        render: (locale: ViewContext["locale"], catalogs: Catalog[]) => () => void,
        locale: ViewContext["locale"],
        catalogs: Catalog[],
    ) {
        // render in the language and keep its localization
        this.#render = render;
        this.#locale = locale;
        this.#localization = Localization.of(locale ?? SOURCE_LOCALE, catalogs);
        this.#dispose = render(locale, catalogs);
    }

    /** The localization rendered. */
    get localization(): Localization {
        return this.#localization;
    }

    /** Render in a display's language once its catalogs arrive, keeping a language that stays and reporting a failed fetch. */
    switch(next: ViewDisplay, signal: AbortSignal): void {
        // keep the language when it stays
        if (next.locale === this.#locale) {
            return;
        }

        // render the latest switch once its catalogs arrive
        this.#locale = next.locale;
        this.#switches += 1;
        const switched = this.#switches;
        fetchCatalogs(next.catalogs).then(
            (fetched) => {
                if (switched !== this.#switches || signal.aborted) {
                    return;
                }
                this.#dispose();
                this.#localization = Localization.of(next.locale, fetched);
                this.#dispose = this.#render(next.locale, fetched);
            },
            (error: unknown) => reportError(error),
        );
    }

    /** Remove the rendering. */
    dispose(): void {
        this.#dispose();
    }
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
    for (const name of PermissionScope.options) {
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
