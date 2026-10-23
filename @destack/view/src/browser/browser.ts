import "../styles/styles.ts";
import { Catalog, Locale, type LocaleTag, Localization, SOURCE_LOCALE } from "@destack/locale";
import { PACKAGE_PATH } from "@destack/object";
import { present, schema } from "@destack/schema";
import { PackageId } from "@destack/package";
import type { ObjectType } from "@destack/object";
import { BrowserTab } from "@destack/object/browser";
import { BrowserClient } from "@destack/account/browser";
import type { Subject } from "@destack/sync";
import type { ClientOptions } from "@destack/object/client";
import { reportToDevtools, startTelemetry } from "@destack/telemetry/browser";
import { OTLP_ORIGIN_PATH, OtlpExporter } from "@destack/telemetry/otlp";
import { CLIENT_PATH, CONTEXT_ELEMENT, HOME_PATH, ViewContext } from "../declare/context.ts";
import type { CommandDeclaration } from "../declare/command.ts";
import { user } from "@destack/account/object";
import { deviceSetting, setting, SettingValue } from "@destack/setting/object";
import { Theme, type ThemeDescription } from "@destack/theme";
import { destackTheme } from "@destack/theme/declare";
import { Snapshot } from "@destack/db";
import { Presentation, type Page, PRESENTED_SETTINGS } from "./presentation.ts";
import type { View } from "../declare/view.ts";
import { CONTEXT_SCOPES } from "../declare/context.ts";
import { renderView } from "../page/mount.ts";
import { mountPackages } from "../page/view.ts";
import { listenForCommands } from "../palette/overlay.ts";
import { Clock } from "@destack/environment";
import { ViewError } from "../error/error.ts";

/** Mount a view with its package's theme and commands once its scopes' objects are open. */
export async function mount(
    view: View,
    declared: {
        readonly commands: readonly CommandDeclaration[];
        readonly theme?: ThemeDescription;
    },
): Promise<() => Promise<void>> {
    // read the view's context from the page, and the release its package's build serves
    const element = document.getElementById(CONTEXT_ELEMENT);
    if (element === null) {
        throw ViewError.of("PAGE_CONTEXT_MISSING", `the page has no ${CONTEXT_ELEMENT} context`);
    }
    const context = ViewContext.parse(JSON.parse(element.textContent));
    const release = view.package.version;

    // export the page's telemetry and uncaught failures to its origin, which stamps the build it serves
    const exporter = OtlpExporter.http(
        { endpoint: OTLP_ORIGIN_PATH, isKeepalive: true },
        reportToDevtools,
    );
    const telemetry = await startTelemetry(
        exporter.options({ name: view.package.name, version: release }),
    );

    // open the view's scopes and the person's own on this browser's device over the origin's package path
    const device = await new BrowserClient({ path: CLIENT_PATH }).device();
    const origin = pageOrigin();
    const tabs = await openTabs(context, view, device, origin);
    const own = present(tabs.get(context.user.id), "the person's own scope");
    const clients = new Map([...tabs].map(([scope, tab]) => [scope, tab.client]));
    const packages = mountPackages(view.packages, origin);

    // resolve what the page shows the person against the package's theme, else Destack's
    const stopping = new AbortController();
    const theme =
        declared.theme === undefined
            ? destackTheme
            : new Theme(declared.theme.package, declared.theme);
    const page = { context, packageId: view.package.id, theme, commands: declared.commands };
    const shown = follow(own, page, device, stopping.signal);
    const opened = await shown.next();
    if (opened.done === true) {
        throw new TypeError("the person's presentation ended before the page showed");
    }
    const first = opened.value;
    restyle(first);

    // render the root component under the view and its clients, once its package's catalogs arrive
    const { default: Component } = await view.component();
    const rendering = await LocalizedRendering.open(
        (locale, localization) =>
            renderView(
                Component,
                document.body,
                { context, clients, packages, origin },
                localization,
            ),
        { owner: view.package.id, locale: first.locale, signal: stopping.signal },
    );

    // open the command palette over the view by the person's keybindings
    let bound = first.keybindings;
    listenForCommands(
        stopping.signal,
        () => rendering.localization,
        () => bound,
    );

    // restyle on each change, render again in a new language and follow the keybindings
    void (async () => {
        for await (const next of shown) {
            restyle(next);
            rendering.switch(next.locale);
            bound = next.keybindings;
        }
    })().catch((error: unknown) => {
        if (!stopping.signal.aborted) {
            reportError(error);
        }
    });

    // record a visit of the view as it loads and after each navigation within it
    const { visit } = telemetry.scope({ ...view.package, version: release });
    const visited = (referrer?: string) =>
        visit(view.name, {
            path: location.pathname,
            title: document.title,
            locale: navigator.language,
            ...(referrer === undefined ? {} : { referrer }),
        });
    visited(document.referrer === "" ? undefined : new URL(document.referrer).host);
    window.navigation?.addEventListener("navigatesuccess", () => visited(), {
        signal: stopping.signal,
    });

    // release the page's rendering, tabs and telemetry on unmount
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

/** Where a rendering's catalogs sit, the language it starts in and how long it lives. */
interface RenderingOptions {
    /** The package whose build serves the page, beside whose catalogs the others sit. */
    readonly owner: PackageId;
    /** The language it starts in. */
    readonly locale: LocaleTag;
    /** Abort its loads once the page goes. */
    readonly signal: AbortSignal;
}

/** A view rendered in its person's language, loading each package's catalogs on first use. */
class LocalizedRendering {
    /** Render the view in a language under a localization, returning how to remove it. */
    readonly #render: (locale: LocaleTag, localization: Localization) => () => void;
    /** The package whose build serves the page, beside whose catalogs the others sit. */
    readonly #owner: PackageId;
    /** Abort the loads once the page goes. */
    readonly #signal: AbortSignal;
    /** The packages whose messages the page renders, whose catalogs each language loads. */
    readonly #packages: Set<PackageId>;
    /** The loaded catalogs by language and package, none for a package without a translation. */
    readonly #catalogs: Map<LocaleTag, Map<PackageId, Catalog[]>>;
    /** The language rendered. */
    #locale: LocaleTag;
    /** The localization rendered, which the page's own overlays render their messages in. */
    #localization: Localization;
    /** Remove the current rendering. */
    #dispose: () => void;

    /** Draw a view in a language with its own package's catalogs. */
    private constructor(
        render: (locale: LocaleTag, localization: Localization) => () => void,
        options: RenderingOptions,
        own: Catalog[],
    ) {
        // keep the renderer, the language and the own package's catalogs, and draw
        this.#render = render;
        this.#owner = options.owner;
        this.#signal = options.signal;
        this.#packages = new Set([options.owner]);
        this.#catalogs = new Map([[options.locale, new Map([[options.owner, own]])]]);
        this.#locale = options.locale;
        this.#localization = this.#localize();
        this.#dispose = this.#render(this.#locale, this.#localization);
    }

    /** Render a view in a language once its own package's catalogs arrive. */
    static async open(
        render: (locale: LocaleTag, localization: Localization) => () => void,
        options: RenderingOptions,
    ): Promise<LocalizedRendering> {
        const own = await fetchCatalogs(options.owner, options.locale, options.owner);

        return new LocalizedRendering(render, options, own);
    }

    /** The localization rendered. */
    get localization(): Localization {
        return this.#localization;
    }

    /** Render in a language once the catalogs of every package the page renders arrive. */
    switch(locale: LocaleTag): void {
        // keep the language when it stays
        if (locale === this.#locale) {
            return;
        }

        // draw the latest language once its catalogs arrive
        this.#locale = locale;
        this.#refresh(locale, [...this.#packages]);
    }

    /** Remove the rendering. */
    dispose(): void {
        this.#dispose();
    }

    /** Load a package's catalogs once its first message renders, drawing again with them. */
    #request(packageId: PackageId): void {
        // load a package once, in the language shown
        if (!this.#packages.has(packageId)) {
            this.#packages.add(packageId);
            this.#refresh(this.#locale, [packageId]);
        }
    }

    /** Load some packages' catalogs in a language, drawing again while it is shown. */
    #refresh(locale: LocaleTag, packages: readonly PackageId[]): void {
        this.#load(locale, packages).then(
            () => {
                // draw while the page still shows the language
                if (locale === this.#locale && !this.#signal.aborted) {
                    this.#draw();
                }
            },
            (error: unknown) => reportError(error),
        );
    }

    /** Load the catalogs of some packages in a language that it has yet to load. */
    async #load(locale: LocaleTag, packages: readonly PackageId[]): Promise<void> {
        // fetch the packages the language has no catalogs of yet
        const loaded = this.#catalogs.get(locale) ?? new Map<PackageId, Catalog[]>();
        this.#catalogs.set(locale, loaded);
        const missing = packages.filter((packageId) => !loaded.has(packageId));
        await Promise.all(
            missing.map(async (packageId) => {
                loaded.set(packageId, await fetchCatalogs(packageId, locale, this.#owner));
            }),
        );
    }

    /** Draw the view again in the language shown, with the catalogs loaded for it. */
    #draw(): void {
        this.#dispose();
        this.#localization = this.#localize();
        this.#dispose = this.#render(this.#locale, this.#localization);
    }

    /** Bind the language shown to its loaded catalogs, asking for each package's on first use. */
    #localize(): Localization {
        const catalogs = [...(this.#catalogs.get(this.#locale)?.values() ?? [])].flat();

        return Localization.of(this.#locale, catalogs, {
            request: (packageId) => this.#request(packageId),
        });
    }
}

/** Follow the person's own scope, presenting the page again after each change. */
async function* follow(
    tab: BrowserTab,
    page: Page,
    device: Subject,
    signal: AbortSignal,
): AsyncGenerator<Presentation, void> {
    // present the page to its person
    const { database } = tab.client;
    const person = schema.identifier("user").parse(page.context.user.id);

    // present the person's settings along their chain in their language
    const presented = async (): Promise<Presentation> => {
        // read the values along their chain and their profile's language from one snapshot
        const snapshot = Snapshot.live(database);
        const { chain, values } = await SettingValue.list(snapshot, PRESENTED_SETTINGS, person);
        const row = await snapshot.read(user.table).findFirst({ where: { id: person } });
        const locale = present(row, "the person's profile").locale;

        // read the values this browser's device keeps for itself, from the same snapshot
        const kept = await snapshot.read(deviceSetting.table).findMany({
            where: { scope: schema.identifier("device").parse(device.id) },
        });

        return Presentation.resolve(page, { locale, chain }, values, kept);
    };

    // present now, then after each change of their settings, their device's or their language
    const { sequence } = await database.log.position();
    yield await presented();
    for await (const changed of database.log.follow(
        { tables: [setting.table, deviceSetting.table, user.table], after: sequence },
        signal,
    )) {
        if (changed.changes.length > 0) {
            yield await presented();
        }
    }
}

/** Apply the theme's style and the language the page shows. */
function restyle(display: Presentation): void {
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

/** Fetch a package's catalogs along a language's fallback chain from the page's build. */
async function fetchCatalogs(
    packageId: PackageId,
    locale: LocaleTag,
    owner: PackageId,
): Promise<Catalog[]> {
    const fetched = await Promise.all(
        Locale.fallback(locale, SOURCE_LOCALE).map(async (tag) => {
            // fetch the catalog at its build path, a package without one answering not found
            const path = `/${Catalog.path({ package: packageId, locale: tag }, owner)}`;
            const response = await fetch(new URL(path, location.origin));
            if (response.status === 404) {
                await response.body?.cancel();

                return [];
            } else if (!response.ok) {
                throw ViewError.of(
                    "CATALOG_UNAVAILABLE",
                    `the catalog at ${path} responded with ${response.status}`,
                );
            }

            // require the package and language of its path
            const catalog = Catalog.parse(await response.json());
            if (catalog.package !== packageId || catalog.locale !== tag) {
                throw ViewError.of(
                    "CATALOG_MISMATCH",
                    `the catalog at ${path} is for another package or language`,
                );
            }

            return [catalog];
        }),
    );

    return fetched.flat();
}

/** Open a tab for each scope a view's context opens, once its tables exist. */
async function openTabs(
    context: ViewContext,
    view: View,
    device: Subject,
    origin: (path: string) => ClientOptions,
): Promise<Map<string, BrowserTab>> {
    // follow the person's own settings and profile, and group the view's types by the scope each lives in
    const byScope = new Map<string, Readonly<Record<string, ObjectType>>>([
        [context.user.id, { setting, deviceSetting, user }],
    ]);
    for (const name of CONTEXT_SCOPES) {
        const scope = context[name];
        const objects = view.opens(name);
        if (scope !== undefined && objects.length > 0) {
            const named = Object.fromEntries(objects.map((object) => [object.name, object]));
            byScope.set(scope, { ...byScope.get(scope), ...named });
        }
    }

    // open each scope's tab at the origin's package path, calling other scopes' homes below its home path
    const url = new URL(PACKAGE_PATH, location.origin).href;

    return new Map(
        await Promise.all(
            [...byScope].map(async ([scope, objects]) => {
                const tab = await BrowserTab.open({
                    name: scope,
                    objects,
                    scope,
                    caller: context.user,
                    device,
                    endpoint: { url },
                    // reopen the same endpoint
                    reconnect: () => ({ url }),
                    homes: (home: string) => origin(`${HOME_PATH}/${home}`),
                    clock: Clock.system,
                    report: reportError,
                });
                await tab.ready;

                return [scope, tab] as const;
            }),
        ),
    );
}

/** Call a path below the page's origin, which its session cookie authenticates. */
function pageOrigin(): (path: string) => ClientOptions {
    return (path) => ({ url: new URL(path, location.origin).href });
}
