import { Environment } from "@destack/environment";
import type { BrowserCapability } from "@destack/environment/browser";
import { Localization, SOURCE_LOCALE } from "@destack/locale";
import type { JsonObject } from "@destack/schema";
import { TelemetryScope } from "@destack/telemetry";
import { reportToDevtools, startTelemetry } from "@destack/telemetry/browser";
import { OTLP_ORIGIN_PATH, OtlpExporter } from "@destack/telemetry/otlp";
import type { ThemeDescription } from "@destack/theme";
import { type Component, createComponent } from "solid-js";
import { hydrate } from "@solidjs/web";
import type { PackageVersion } from "@destack/package";
import type { View } from "../declare/view.ts";
import { recordVisits } from "../browser/visit.ts";
import { ViewError } from "../error/error.ts";
import { ViewRoot, type ViewRootProperties } from "../page/mount.ts";
import { type IslandProperties, IslandSlot } from "./island.tsx";
import { StaticPage } from "./page.ts";

/** The telemetry a page starts, exporting to the origin serving it. */
type PageTelemetry = Awaited<ReturnType<typeof startTelemetry>>;

/** The island components a page's entry loads, by their package-relative module. */
export type IslandModules = Readonly<Record<string, () => Promise<Component<JsonObject>>>>;

/** Hydrate a static view's prerendered page whole, exporting its telemetry to its origin. */
export async function hydrateStatic(
    view: View,
    declared: { readonly theme?: ThemeDescription },
    environment: Environment<"workers" | BrowserCapability>,
): Promise<void> {
    // hydrate the page under the page's telemetry
    const telemetry = await startPageTelemetry(view, environment);
    const page = await StaticPage.open(
        view,
        declared,
        Environment.instrument(environment, telemetry),
    );
    const dispose = hydrate(() => page.render(), document);

    // follow the page until it goes
    followPage(view, telemetry, environment, [dispose]);
}

/** Hydrate each island of a static view's page from its module, leaving the rest as HTML. */
export async function hydrateIslands(
    view: { readonly name: string; readonly package: PackageVersion },
    islands: IslandModules,
    environment: Environment<"workers" | BrowserCapability>,
): Promise<void> {
    // load every island's component at once, each root with its module and properties
    const telemetry = await startPageTelemetry(view, environment);
    const rendered = Environment.instrument(environment, telemetry);
    const localization = Localization.of(SOURCE_LOCALE, []);
    const roots = [...document.querySelectorAll<HTMLElement>("[data-island]")];
    const loaded = await Promise.all(roots.map((root) => loadIsland(root, islands)));

    // hydrate each root in document order by the id the server rendered it under, its children kept in its slot
    const disposals = loaded.map(({ root, Island, properties }) =>
        hydrateIsland(root, Island, properties, {
            package: view.package,
            environment: rendered,
            localization,
        }),
    );

    // follow the page until it goes
    followPage(view, telemetry, environment, disposals);
}

/** Load an island root's component and read its properties, its children in its slot. */
async function loadIsland(
    root: HTMLElement,
    islands: IslandModules,
): Promise<{
    readonly root: HTMLElement;
    readonly Island: Component<JsonObject>;
    readonly properties: JsonObject;
}> {
    // load the component of the root's module
    const { island: module, islandProperties } = root.dataset;
    const load = module === undefined ? undefined : islands[module];
    if (module === undefined || load === undefined) {
        throw ViewError.of(
            "ISLAND_INVALID",
            `island '${module ?? "without a module"}' is invalid: the page loads no such island`,
        );
    }
    const Island = await load();

    // read its properties, giving it its slot when it has children
    const parsed: unknown =
        islandProperties === undefined ? undefined : JSON.parse(islandProperties);
    if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
        throw ViewError.of(
            "ISLAND_PROPERTIES_INVALID",
            `island '${module}' is invalid: its properties must match its schema`,
        );
    }
    const properties =
        "islandChildren" in root.dataset
            ? { ...parsed, children: createComponent(IslandSlot, {}) }
            : parsed;

    return { root, Island, properties };
}

/** Hydrate one island root under the view's root, as the server rendered it inside its id. */
export function hydrateIsland<Properties extends IslandProperties<Properties>>(
    root: HTMLElement,
    Island: Component<Properties>,
    properties: Properties,
    page: Omit<ViewRootProperties, "children" | "mount">,
): () => void {
    return hydrate(
        () =>
            createComponent(ViewRoot, {
                ...page,
                get children() {
                    return createComponent(Island, properties);
                },
            }),
        root,
        { renderId: islandId(root) },
    );
}

/** Read the hydration id the server rendered an island root under. */
function islandId(root: HTMLElement): string {
    const { islandId: id } = root.dataset;
    if (id === undefined) {
        throw ViewError.of("ISLAND_INVALID", "island root is invalid: it has no hydration id");
    }

    return id;
}

/** Start a page's telemetry, exporting it and uncaught failures to the origin serving it. */
function startPageTelemetry(
    view: { readonly package: PackageVersion },
    environment: Environment<"workers" | BrowserCapability>,
): Promise<PageTelemetry> {
    const exporter = OtlpExporter.http(
        {
            endpoint: OTLP_ORIGIN_PATH,
            isKeepalive: true,
            fetch: (request) => environment.network.fetch(request),
        },
        reportToDevtools,
    );

    return startTelemetry(exporter.options(view.package), environment, window);
}

/** Record a page's visits, and release its renderings and telemetry once it goes. */
function followPage(
    view: { readonly name: string; readonly package: PackageVersion },
    telemetry: PageTelemetry,
    environment: Environment<"workers" | BrowserCapability>,
    disposals: readonly (() => void)[],
): void {
    const stopping = new AbortController();
    recordVisits(
        view.name,
        TelemetryScope.of(view.package, telemetry),
        environment,
        stopping.signal,
    );
    window.addEventListener(
        "pagehide",
        () => {
            stopping.abort();
            for (const dispose of disposals) {
                dispose();
            }
            void telemetry.shutdown();
        },
        { once: true },
    );
}
