import { Action, type ObjectType, Place } from "@destack/object";
import { createSignal } from "solid-js";
import { hydrate } from "@solidjs/web";
import { TelemetryScope } from "@destack/telemetry";
import type { Environment } from "@destack/environment";
import type { BrowserCapability, BrowserEnvironment } from "@destack/environment/browser";
import "../styles/styles.ts";
import { Catalog, Locale, type LocaleTag, Localization, SOURCE_LOCALE } from "@destack/locale";
import { type JsonObject, present, schema, Version } from "@destack/schema";
import { OriginPath, PackageId } from "@destack/package";
import { BrowserTab, DeviceElection } from "@destack/object/client/browser";
import { BrowserClient } from "@destack/account/browser";
import { ObjectTypeReference, type Subject } from "@destack/sync";
import { principal } from "@destack/access";
import { type Endpoint, ObjectConnection, type ObjectNode } from "@destack/object/client";
import { reportToDevtools, startTelemetry } from "@destack/telemetry/browser";
import { OtlpExporter } from "@destack/telemetry/otlp";
import {
    BUILD_PARAMETER,
    CONTEXT_ELEMENT,
    CONTEXT_SCOPES,
    ScopePath,
    ViewContext,
} from "../declare/context.ts";
import type { ActionDeclaration } from "../palette/keybinding.ts";
import { PageUndo } from "../page/undo.ts";
import { account, space, user } from "@destack/account/object";
import { setting, SettingValue } from "@destack/setting/object";
import { Theme, type ThemeDescription } from "@destack/theme";
import { destackTheme } from "@destack/theme/declare";
import { Presentation, type Page, PRESENTED_SETTINGS } from "./presentation.ts";
import type { View } from "../declare/view.ts";
import type { ViewMount } from "../page/context.ts";
import { ViewPage } from "../static/page.ts";
import { appendFaces } from "../document/font.tsx";
import { destackFonts } from "../document/typeface.ts";
import { recordVisits } from "./visit.ts";
import { DeviceRequest, DeviceRole } from "./request.ts";
import { mountPackages } from "../page/view.ts";
import { PageAuthorization } from "./authorization.ts";
import { PaletteOverlay } from "../palette/overlay.ts";
import { ActionRegistry, palette, redo, undo } from "../page/action.ts";
import { ViewError } from "../error/error.ts";

/** Mount a view with its package's theme and actions once its scopes' objects are open. */
export async function mount(
    view: View,
    declared: {
        readonly actions: readonly ActionDeclaration[];
        readonly theme?: ThemeDescription;
    },
    environment: Environment<"workers" | BrowserCapability>,
    deviceWorker: URL,
): Promise<() => Promise<void>> {
    // answer the page that authorizes in this window, rendering nothing
    if (PageAuthorization.answer(environment)) {
        return async () => {};
    }

    // read the view's context from the page, and the release its package's build serves
    const element = document.getElementById(CONTEXT_ELEMENT);
    if (element === null) {
        throw ViewError.of("PAGE_CONTEXT_BROKEN", `the page has no '${CONTEXT_ELEMENT}' context`);
    }
    const context = ViewContext.parse(JSON.parse(element.textContent));
    const release = view.package.version;

    // stand for the person's device of this installation at once, its worker loading while the page asks for its request
    const deviceName = `${context.installation}.${context.user.id}`;
    const requested = Promise.withResolvers<JsonObject>();
    const election = DeviceElection.start(
        deviceName,
        BrowserTab.worker(deviceWorker, deviceName, requested.promise, environment),
        environment,
    );

    // export the page's telemetry and uncaught failures to its origin, which stamps the build it serves
    const { network, navigation } = environment;
    const exporter = OtlpExporter.http(
        {
            endpoint: `${context.base}${OriginPath.telemetry}`,
            isKeepalive: true,
            fetch: (request) => network.fetch(request),
        },
        reportToDevtools,
    );
    const telemetry = await startTelemetry(
        exporter.options({ name: view.package.name, version: release }),
        environment,
        window,
    );

    // mount the page in a span, each of its phases in its own
    const instruments = TelemetryScope.of(import.meta.destack.package, telemetry);

    return await instruments.span(
        "destack.page.mount",
        { "destack.view.name": view.name },
        async () => {
            // open the view's page beside the tabs, as it needs none of them
            const rendered = { ...environment, telemetry };
            const opening = instruments.span("destack.page.component", {}, () =>
                ViewPage.open(view, declared),
            );

            // open the view's scopes below the page's base, a person's own scope on this browser's device
            const origin = new URL(navigation.currentEntry.url).origin;
            const root = `${origin}${context.base}`;
            const isPerson = principal.user.is(context.user);
            const device = isPerson
                ? await instruments.span("destack.page.session", {}, () =>
                      new BrowserClient({
                          path: `${context.base}${OriginPath.session}`,
                          origin,
                          environment,
                      }).device(),
                  )
                : undefined;
            const authorization = new PageAuthorization(root, environment);
            const call = pageOrigin(root, authorization);
            const tabbing = instruments.span("destack.page.tabs", {}, () =>
                openTabs(
                    context,
                    view,
                    { device, root, name: deviceName, election, request: requested.resolve },
                    { ...environment, telemetry },
                ),
            );
            const packages = mountPackages(view.packages, context.space, call);
            const connections = new Map<string, ObjectConnection>();
            const connect = (path: string, scope: string): ObjectConnection => {
                // connect to each path's store once for the page, each scope over its socket
                const known = connections.get(path);
                if (known !== undefined) {
                    return known.at(scope);
                }
                const connection = ObjectConnection.open({
                    scope,
                    url: ObjectConnection.url(`${root}${path}`),
                    homes: (home) => connect(ScopePath.of(home), home),
                    environment,
                });
                connections.set(path, connection);

                return connection;
            };

            // present the page as this browser last showed it, else its defaults, under the package's theme
            const stopping = new AbortController();
            const theme = declared.theme === undefined ? destackTheme : new Theme(declared.theme);
            const presented = { context, packageId: view.package.id, theme };
            const visiting =
                device === undefined ? visitor(presented, environment.locale) : undefined;
            const first = await instruments.span("destack.page.presentation", {}, () =>
                firstPresentation(presented, visiting, environment),
            );
            appendFaces(document, destackFonts);
            restyle(first);

            // reload a development build once its origin serves another build, as live reload servers do
            if (release === Version.development) {
                void reloadOnChange(
                    `${root}${OriginPath.reload}`,
                    context.build,
                    environment,
                    stopping.signal,
                );
            }

            // hydrate the server's page in the language it rendered, its scopes empty until the device's tabs open
            const page = await opening;
            const [clients, setClients] = createSignal<ReadonlyMap<string, ObjectNode>>(new Map(), {
                ownedWrite: true,
            });
            const [localization, setLocalization] = createSignal(page.root(rendered).localization, {
                ownedWrite: true,
            });
            const [keybindings, setKeybindings] = createSignal(first.keybindings, {
                ownedWrite: true,
            });
            const actions = new ActionRegistry(keybindings);
            const mounted: ViewMount = {
                context,
                clients,
                open: async (scope, closing) => {
                    // take the device's tab of a scope it holds, else open another space's
                    const opened = await tabbing;

                    return opened.tabs.get(scope) ?? (await opened.open(scope, closing));
                },
                packages,
                origin: call,
                connect,
            };
            const dispose = await instruments.span("destack.page.hydrate", {}, () =>
                hydrate(
                    () =>
                        page.render(mounted, {
                            actions,
                            localization: Localization.follow(localization),
                            environment: rendered,
                        }),
                    document,
                ),
            );

            // act on the object the view opened for, and offer the package's actions on the types it opens
            const { target } = context;
            if (target !== undefined) {
                actions.provide(() => Place.current(target));
            }
            for (const { name, description } of declared.actions) {
                const types = [
                    description.call,
                    ...(description.call?.inputs.map(({ type }) => type) ?? []),
                ];
                const isOpened = types.every(
                    (type) => type === undefined || typeOf(view, type) !== undefined,
                );
                if (isOpened) {
                    const read = (type: ObjectTypeReference) =>
                        present(typeOf(view, type), `the type ${type.type}`);
                    actions.offer(Action.of(name, description, read));
                }
            }

            // localize the page in its person's language once its own package's catalogs arrive
            const localizing = await PageLocalization.open(setLocalization, {
                owner: view.package.id,
                locale: first.locale,
                signal: stopping.signal,
                environment,
                origin: root,
                catalogs: new Set(context.catalogs),
            });

            // run the person's key combinations and open the command palette over the view
            let shown = first;
            const overlay = new PaletteOverlay({
                actions,
                declared: declared.actions,
                navigation,
                localization: () => localizing.localization,
                presentation: () => shown,
            });
            overlay.listen(stopping.signal);
            actions.handle(palette, { run: (event) => overlay.open(event.element) });

            // record a visit of the view as it loads and each time it moves to another path
            const scope = TelemetryScope.of({ ...view.package, version: release }, telemetry);
            recordVisits(view.name, scope, environment, stopping.signal);

            // release the page's rendering, connections, tabs and telemetry on unmount
            const unmount = async () => {
                // stop the page's work and close its connections
                stopping.abort();
                dispose();
                for (const connection of connections.values()) {
                    connection.close();
                }
                try {
                    const { tabs } = await tabbing;
                    await Promise.all([...tabs.values()].map((tab) => tab.close()));
                    await election.close();
                } finally {
                    await telemetry.shutdown();
                }
            };
            window.addEventListener("pagehide", () => void unmount(), { once: true });

            // follow the device once its tabs open: its scopes, undo across them and the person's presentation
            const { tabs, ready } = await tabbing;
            setClients(tabs);
            actions.attach({ clients: tabs, scope: context.space });
            const pageUndo = new PageUndo([...tabs.values()].map((client) => client.undoStack));
            pageUndo.listen(stopping.signal);
            actions.handle(undo, {
                run: async () => {
                    await pageUndo.undo();
                },
                isEnabled: pageUndo.canUndo,
            });
            actions.handle(redo, {
                run: async () => {
                    await pageUndo.redo();
                },
                isEnabled: pageUndo.canRedo,
            });
            const following =
                visiting ??
                follow(
                    present(tabs.get(context.user.id), "the person's own scope"),
                    presented,
                    present(device, "the person's device"),
                    stopping.signal,
                );

            // restyle on each change, localize in a new language and follow the keybindings
            void (async () => {
                for await (const next of following) {
                    restyle(next);
                    localizing.switch(next.locale);
                    shown = next;
                    setKeybindings(next.keybindings);
                    Presentation.cache(presented, next, environment.storage);
                }
            })().catch((error: unknown) => {
                if (!stopping.signal.aborted) {
                    globalThis.reportError(error);
                }
            });

            // fail with a device that cannot open
            await instruments.span("destack.page.ready", {}, () => ready);

            return unmount;
        },
    );
}

/** Where a rendering's catalogs sit, the language it starts in and how long it lives. */
interface RenderingOptions {
    /** The package whose build serves the page, beside whose catalogs the others sit. */
    readonly owner: PackageId;
    /** The language it starts in. */
    readonly locale: LocaleTag;
    /** Abort its loads once the page goes. */
    readonly signal: AbortSignal;
    /** The page's network its catalogs load through. */
    readonly environment: Pick<BrowserEnvironment, "network">;
    /** The page's origin with its base, below which its build serves the catalogs. */
    readonly origin: string;
    /** The catalogs the page's builds serve, by path below its base. */
    readonly catalogs: ReadonlySet<string>;
}

/** A page's localization in its person's language, loading each package's catalogs on first use. */
class PageLocalization {
    /** Publish a localization to the page, which renders its messages again in place. */
    readonly #publish: (localization: Localization) => void;
    /** The package whose build serves the page, beside whose catalogs the others sit. */
    readonly #owner: PackageId;
    /** Abort the loads once the page goes. */
    readonly #signal: AbortSignal;
    /** The page's network and origin its catalogs load through, and the catalogs served. */
    readonly #source: Pick<RenderingOptions, "environment" | "origin" | "catalogs">;
    /** The packages whose messages the page renders, whose catalogs each language loads. */
    readonly #packages: Set<PackageId>;
    /** The loaded catalogs by language and package, none for a package without a translation. */
    readonly #catalogs: Map<LocaleTag, Map<PackageId, Catalog[]>>;
    /** The language rendered. */
    #locale: LocaleTag;
    /** The localization published, which the page's own overlays render their messages in. */
    #localization: Localization;

    /** Localize a page in a language with its own package's catalogs. */
    private constructor(
        publish: (localization: Localization) => void,
        options: RenderingOptions,
        own: Catalog[],
    ) {
        // keep the publisher, the language and the own package's catalogs, and publish
        this.#publish = publish;
        this.#owner = options.owner;
        this.#source = options;
        this.#signal = options.signal;
        this.#packages = new Set([options.owner]);
        this.#catalogs = new Map([[options.locale, new Map([[options.owner, own]])]]);
        this.#locale = options.locale;
        this.#localization = this.#localize();
        publish(this.#localization);
    }

    /** Localize a page in a language once its own package's catalogs arrive. */
    static async open(
        publish: (localization: Localization) => void,
        options: RenderingOptions,
    ): Promise<PageLocalization> {
        const own = await fetchCatalogs(options.owner, options.locale, options.owner, options);

        return new PageLocalization(publish, options, own);
    }

    /** The localization published. */
    get localization(): Localization {
        return this.#localization;
    }

    /** Localize in a language once the catalogs of every package the page renders arrive. */
    switch(locale: LocaleTag): void {
        // keep the language when it stays
        if (locale === this.#locale) {
            return;
        }

        // draw the latest language once its catalogs arrive
        this.#locale = locale;
        this.#refresh(locale, [...this.#packages]);
    }

    /** Load a package's catalogs once its first message renders, publishing again with them. */
    #request(packageId: PackageId): void {
        // load a package once, in the language shown
        if (!this.#packages.has(packageId)) {
            this.#packages.add(packageId);
            this.#refresh(this.#locale, [packageId]);
        }
    }

    /** Load some packages' catalogs in a language, publishing again while it is shown. */
    #refresh(locale: LocaleTag, packages: readonly PackageId[]): void {
        this.#load(locale, packages).then(
            () => {
                // publish while the page still shows the language
                if (locale === this.#locale && !this.#signal.aborted) {
                    this.#localization = this.#localize();
                    this.#publish(this.#localization);
                }
            },
            (error: unknown) => globalThis.reportError(error),
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
                loaded.set(
                    packageId,
                    await fetchCatalogs(packageId, locale, this.#owner, this.#source),
                );
            }),
        );
    }

    /** Bind the language shown to its loaded catalogs, asking for each package's on first use. */
    #localize(): Localization {
        const catalogs = [...(this.#catalogs.get(this.#locale)?.values() ?? [])].flat();

        return Localization.of(this.#locale, catalogs, {
            request: (packageId) => this.#request(packageId),
        });
    }
}

/** Follow the person's own scope at the device, presenting the page again after each change. */
async function* follow(
    tab: BrowserTab,
    page: Page,
    device: Subject,
    signal: AbortSignal,
): AsyncGenerator<Presentation, void> {
    // follow the person's presented settings along their chain and on this device, and their profile's language
    const person = schema.identifier("user").parse(page.context.user.id);
    const at = { scope: person, device: schema.identifier("device").parse(device.id) };
    const settings = SettingValue.follow(tab, PRESENTED_SETTINGS, at, signal)[
        Symbol.asyncIterator
    ]();
    const profile = tab
        .queryType(user)
        .findFirst({ scopes: "every", where: { id: person } })
        .subscribe();
    const locales = profile.watch(signal)[Symbol.asyncIterator]();

    // present the page from the latest of each, again after either changes
    try {
        let placed = await settings.next();
        let read = await locales.next();
        let nextPlaced = settings.next();
        let nextRead = locales.next();
        while (placed.done !== true && read.done !== true) {
            // present the latest settings in the person's language
            const { chain, values, device: own } = placed.value;
            const row = present(read.value, "the person's profile");
            yield Presentation.resolve(page, { locale: row.locale, chain }, values, own);

            // take whichever changes next, asking that one for its following change
            const settled = await Promise.race([
                nextPlaced.then((step) => ({ kind: "settings" as const, step })),
                nextRead.then((step) => ({ kind: "locale" as const, step })),
            ]);
            if (settled.kind === "settings") {
                placed = settled.step;
                nextPlaced = settings.next();
            } else {
                read = settled.step;
                nextRead = locales.next();
            }
        }
    } finally {
        await profile.close();
    }
}

/** Reload the page once its origin's reload stream ends: on another build, or a lost origin. */
async function reloadOnChange(
    path: string,
    build: string,
    environment: Pick<BrowserEnvironment, "network" | "navigation">,
    signal: AbortSignal,
): Promise<void> {
    // open the stream, a lost origin ending it at once
    const url = `${path}?${new URLSearchParams({ [BUILD_PARAMETER]: build }).toString()}`;
    const response = await environment.network.fetch(new Request(url, { signal })).then(
        (opened) => opened,
        () => undefined,
    );

    // refuse a refused stream, which a reload would only meet again
    if (response !== undefined && !response.ok) {
        throw ViewError.of(
            "RESPONSE_INVALID",
            `reload stream is invalid: its origin answered '${String(response.status)}'`,
        );
    }

    // read it to its end, a lost origin ending it too, then reload unless the page stopped it
    await response?.text().then(
        () => undefined,
        () => undefined,
    );
    if (!signal.aborted) {
        environment.navigation.reload();
    }
}

/** Present a page to an anonymous visitor once, in the browser's language and the defaults. */
async function* visitor(page: Page, locale: LocaleTag): AsyncGenerator<Presentation, void> {
    yield Presentation.resolve(page, { locale, chain: [] }, [], []);
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
    options: Pick<RenderingOptions, "environment" | "origin" | "catalogs">,
): Promise<Catalog[]> {
    // fetch only the catalogs the page's builds serve
    const served = Locale.fallback(locale, SOURCE_LOCALE).filter((tag) =>
        options.catalogs.has(Catalog.path({ package: packageId, locale: tag }, owner)),
    );
    const fetched = await Promise.all(
        served.map(async (tag) => {
            // fetch the catalog at its build path
            const path = `/${Catalog.path({ package: packageId, locale: tag }, owner)}`;
            const request = new Request(`${options.origin}${path}`);
            const response = await options.environment.network.fetch(request);
            if (!response.ok) {
                throw ViewError.of(
                    "CATALOG_UNREACHABLE",
                    `could not read the catalog at '${path}': status ${response.status}`,
                );
            }

            // require the package and language of its path
            const catalog = Catalog.parse(await response.json());
            if (catalog.package !== packageId || catalog.locale !== tag) {
                throw ViewError.of(
                    "CATALOG_UNREACHABLE",
                    `could not use the catalog at '${path}': it is for another package or language`,
                );
            }

            return [catalog];
        }),
    );

    return fetched.flat();
}

/** Open a tab for each scope a view's context opens over the person's device, and others later. */
async function openTabs(
    context: ViewContext,
    view: View,
    devices: {
        readonly device: Subject | undefined;
        readonly root: string;
        readonly name: string;
        readonly election: DeviceElection;
        readonly request: (request: JsonObject) => void;
    },
    environment: Environment<"workers" | BrowserCapability>,
): Promise<{
    readonly tabs: Map<string, BrowserTab>;
    readonly open: ViewMount["open"];
    readonly ready: Promise<void>;
}> {
    // follow a person's own settings and profile, and list the parts each scope plays for the view
    const { device } = devices;
    const roles = new Map<string, DeviceRole[]>(
        DeviceRole.played(device).includes("user") ? [[context.user.id, ["user"]]] : [],
    );
    for (const name of CONTEXT_SCOPES) {
        const scope = context[name];
        const role = name === "account" ? "account" : "space";
        const isOpened = view.opens(name === "account" ? account : space).length > 0;
        if (scope !== undefined && isOpened) {
            roles.set(scope, [...(roles.get(scope) ?? []), role]);
        }
    }

    // ask the leading worker for the person's device of this installation, which its tabs share, as installations below one origin keep apart
    const { name, election } = devices;
    devices.request(
        schema.jsonObject().parse(
            DeviceRequest.parse({
                name,
                caller: context.user,
                ...(device === undefined ? {} : { device }),
                root: devices.root,
                space: context.space,
            }),
        ),
    );

    // open a scope's tab over the device, usable at once as its sends wait for the device, ready once its access replica holds the scope's chain
    const instruments = TelemetryScope.of(import.meta.destack.package, environment.telemetry);
    const openTab = (scope: string, played: readonly DeviceRole[]) => {
        const tab = BrowserTab.open({
            name,
            election,
            objects: DeviceRole.objects(view.objects, played),
            scope,
            caller: context.user,
            ...(device === undefined ? {} : { device }),
            environment,
        });
        const ready = instruments.span(
            "destack.tab.ready",
            { "destack.scope.id": scope },
            () => tab.ready,
        );

        return { tab, ready };
    };

    // open another space of the person while a component asks, closing its tab once it stops
    const open = async (scope: string, signal: AbortSignal) => {
        // open its tab, closed with the asking component
        const { tab, ready } = openTab(scope, ["space"]);
        signal.addEventListener("abort", () => void tab.close(), { once: true });

        // hand it out once ready
        await ready;

        return tab;
    };

    // open the context's scopes, each in the parts it plays
    const opened = [...roles].map(([scope, played]) => ({ scope, ...openTab(scope, played) }));
    const tabs = new Map(opened.map(({ scope, tab }) => [scope, tab] as const));

    // be ready once every tab is, the person's own settings and profile synced for the page
    const own = opened.find(({ scope }) => scope === context.user.id)?.tab;
    const synced = own === undefined ? [] : [syncOwn(own, context.user.id)];
    const ready = Promise.all([...opened.map((each) => each.ready), ...synced]).then(() => {});

    return { tabs, open, ready };
}

/** Follow the person's settings and profile in their tab, settling once their first rows arrive. */
async function syncOwn(tab: BrowserTab, scope: string): Promise<void> {
    // follow the profile and settings of the person's own scope
    const { query } = tab;
    const profileKey = ObjectTypeReference.key(user.typeReference);
    const settingKey = ObjectTypeReference.key(setting.typeReference);
    const profile = present(query[profileKey], "the person's profile type");
    const settings = present(query[settingKey], "the person's setting type");

    // settle once both have their first rows
    await Promise.all([
        profile.findMany({ scopes: "every", where: { id: scope } }).subscribe().ready,
        settings.findMany({ where: { scope } }).subscribe().ready,
    ]);
}

/** Take a page's first presentation: the person's last in this browser, else defaults. */
async function firstPresentation(
    page: Page,
    visiting: AsyncGenerator<Presentation, void> | undefined,
    environment: Pick<BrowserEnvironment, "storage" | "locale">,
): Promise<Presentation> {
    // show a person their last presentation in this browser, else the defaults until their device answers
    const remembered =
        visiting === undefined ? Presentation.cached(page, environment.storage) : undefined;
    if (remembered !== undefined) {
        return remembered;
    }
    // show a person this browser has not shown the page the defaults
    else if (visiting === undefined) {
        return Presentation.resolve(page, { locale: environment.locale, chain: [] }, [], []);
    }

    // show a visitor the page's own presentation
    const opened = await visiting.next();
    if (opened.done === true) {
        throw new TypeError("the visitor's presentation ended before the page showed");
    }

    return opened.value;
}

/** Call a path below the page's origin and base, which its session cookie authenticates. */
function pageOrigin(root: string, authorization: PageAuthorization): (path: string) => Endpoint {
    return (path) => ({ url: `${root}${path}`, fetch: (request) => authorization.fetch(request) });
}

/** Find a type a view opens, absent for another. */
function typeOf(
    view: View,
    { packageId, type }: Pick<ObjectTypeReference, "packageId" | "type">,
): ObjectType | undefined {
    return view.objects.find((each) => each.package.id === packageId && each.name === type);
}
