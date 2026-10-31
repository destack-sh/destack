import type { BrowserEnvironment } from "@destack/environment/browser";
import "../styles/styles.ts";
import { Localization, SOURCE_LOCALE } from "@destack/locale";
import type { ThemeDescription } from "@destack/theme";
import { type Component, createComponent, NoHydration } from "solid-js";
import { createRequestEvent, createSSRResponse, type JSX, renderToStream } from "@solidjs/web";
import type { StaticPages, View } from "../declare/view.ts";
import { ViewError } from "../error/error.ts";
import { ViewRoot } from "../page/mount.ts";
import { ViewDocument } from "./document.tsx";
import { IslandContext, IslandPage } from "./island.tsx";

/** A static view's page: its component inside its page shell, prerendered and then hydrated. */
export class StaticPage {
    /** The view the page shows. */
    readonly view: View;
    /** The view's root component. */
    readonly component: Component;
    /** The page shell wrapping the component. */
    readonly document: Component<{ readonly children?: JSX.Element }>;
    /** The environment the components run in. */
    readonly environment: BrowserEnvironment;
    /** The view's pages module once loaded, on the server only. */
    #pages: Promise<StaticPages> | undefined;

    /** Create a page of a view's loaded component and shell in an environment. */
    constructor(
        view: View,
        component: Component,
        document: Component<{ readonly children?: JSX.Element }>,
        environment: BrowserEnvironment,
    ) {
        // keep the view, its loaded modules and the environment
        this.view = view;
        this.component = component;
        this.document = document;
        this.environment = environment;
    }

    /** Open a view's page, in the view's own shell or the runtime's under the package's theme. */
    static async open(
        view: View,
        declared: { readonly theme?: ThemeDescription },
        environment: BrowserEnvironment,
    ): Promise<StaticPage> {
        // load the component and the view's own shell
        const { default: component } = await view.component();
        const { render } = view;
        const own =
            render.mode === "static" && render.document !== undefined
                ? (await render.document()).default
                : undefined;

        // fall back to the runtime's shell under the package's theme
        const document =
            own ??
            ((properties: { readonly children?: JSX.Element }) =>
                createComponent(ViewDocument, {
                    ...(declared.theme === undefined ? {} : { theme: declared.theme }),
                    get children() {
                        return properties.children;
                    },
                }));

        return new StaticPage(view, component, document, environment);
    }

    /** Render the page: the component under its view root inside the page shell. */
    render(): JSX.Element {
        // render the component under its view root inside the shell
        const { component, view, environment } = this;
        const page = (): JSX.Element =>
            createComponent(this.document, {
                get children() {
                    return createComponent(ViewRoot, {
                        package: view.package,
                        environment,
                        localization: Localization.of(SOURCE_LOCALE, []),
                        get children() {
                            return createComponent(component, {});
                        },
                    });
                },
            });

        // hydrate the page whole, or render it without hydration but for its islands
        if (view.render.mode !== "static" || view.render.hydrate !== "islands") {
            return page();
        }

        return createComponent(NoHydration, {
            get children() {
                return createComponent(IslandContext, {
                    value: new IslandPage(view.package),
                    get children() {
                        return page();
                    },
                });
            },
        });
    }

    /** List the view's paths, not-found path and endpoint paths from its pages module. */
    async routes(): Promise<StaticRoutes> {
        const { paths, notFound, endpoints = {} } = await this.#loadPages();

        return {
            paths: [...paths],
            ...(notFound === undefined ? {} : { notFound }),
            endpoints: Object.keys(endpoints),
        };
    }

    /** Load the view's pages module once. */
    #loadPages(): Promise<StaticPages> {
        this.#pages ??= loadPages(this.view);

        return this.#pages;
    }

    /** Answer a request: an endpoint's file at its path, else the page's complete HTML. */
    async respond(request: Request, assets: PageAssets): Promise<Response> {
        // answer an endpoint at its path
        const { render } = this.view;
        const { endpoints = {} } = await this.#loadPages();
        const endpoint = endpoints[new URL(request.url).pathname];
        if (endpoint !== undefined && render.mode === "static") {
            return await endpoint({
                request,
                origin: render.origin,
                environment: this.environment,
            });
        }

        // render the page under the request, as the router reads its path from it, loading the server's request scope
        const { provideRequestEvent } = await import("@solidjs/web/storage");
        const event = createRequestEvent(request);
        const html = await provideRequestEvent(event, () => renderToStream(() => this.render()));

        // link the stylesheets and load the entry at the end of the head, refusing a document without one
        if (!html.includes("</head>")) {
            throw ViewError.of(
                "PAGE_HEAD_MISSING",
                `the document of view ${this.view.name} renders no head`,
            );
        }
        const links = [
            ...assets.stylesheets.map((href) => `<link rel="stylesheet" href="${escape(href)}">`),
            `<script type="module" src="${escape(assets.script)}" async></script>`,
        ];

        return createSSRResponse(html.replace("</head>", `${links.join("")}</head>`), event);
    }
}

/** Load a static view's pages module, refusing a view of another mode. */
function loadPages(view: View): Promise<StaticPages> {
    const { render } = view;
    if (render.mode !== "static") {
        throw ViewError.of("VIEW_NOT_STATIC", `view ${view.name} prerenders no pages`);
    }

    return render.pages();
}

/** A static view's routes: the paths of its pages, its not-found page and its endpoints. */
export interface StaticRoutes {
    /** The paths prerendered below the view's base. */
    readonly paths: readonly string[];
    /** The path rendering the not-found page, none when absent. */
    readonly notFound?: string;
    /** The paths of its endpoints. */
    readonly endpoints: readonly string[];
}

/** The assets a page links: its entry module and stylesheets, by URL. */
export interface PageAssets {
    /** The module the page loads. */
    readonly script: string;
    /** The stylesheets the page links, in order. */
    readonly stylesheets: readonly string[];
}

/** Escape a URL for an HTML attribute. */
function escape(url: string): string {
    return url.replaceAll("&", "&amp;").replaceAll('"', "&quot;").replaceAll("<", "&lt;");
}
