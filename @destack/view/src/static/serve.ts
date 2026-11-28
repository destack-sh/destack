import type { Environment } from "@destack/environment";
import { BrowserEnvironment } from "@destack/environment/browser";
import { account, space } from "@destack/account/object";
import { ObjectType } from "@destack/object";
import type { ObjectConnection } from "@destack/object/client";
import { ObjectClient } from "@destack/object/device";
import type { EphemeralHost, ViewsStart } from "@destack/object/workload";
import { present } from "@destack/schema";
import type { ThemeDescription } from "@destack/theme";
import type { StaticSite, View } from "../declare/view.ts";
import { CONTEXT_ELEMENT } from "../declare/context.ts";
import { type PageHead, RenderRequest } from "../declare/render.ts";
import { createRequestEvent, createSSRResponse, renderToStream } from "@solidjs/web";
import { ViewError } from "../error/error.ts";
import type { ViewMount } from "../page/context.ts";
import { TELEMETRY_META, ViewPage } from "./page.ts";

/** A view's page answered on a server: its endpoints, and its HTML under a host's mount. */
export class PageServer {
    /** The page the server renders. */
    readonly page: ViewPage;
    /** The server's environment, which each render gives its request's URL. */
    readonly environment: Environment;
    /** The view's pages module once loaded. */
    #pages: Promise<StaticSite> | undefined;

    /** Answer a view's page on a server in its environment. */
    constructor(page: ViewPage, environment: Environment) {
        this.page = page;
        this.environment = environment;
    }

    /** Open a view's page to answer on a server. */
    static async open(
        view: View,
        declared: { readonly theme?: ThemeDescription },
        environment: Environment,
    ): Promise<PageServer> {
        return new PageServer(await ViewPage.open(view, declared), environment);
    }

    /** List the view's paths, not-found path and endpoint paths from its pages module. */
    async routes(): Promise<RouteTable> {
        const { paths, notFound, endpoints = {} } = await this.#loadPages();

        return {
            paths: [...paths],
            ...(notFound === undefined ? {} : { notFound }),
            endpoints: Object.keys(endpoints),
        };
    }

    /** Answer a request: an endpoint's file at its path, else the page's HTML under a mount. */
    async respond(request: Request, head: PageHead, mount: ViewMount | null): Promise<Response> {
        // answer a static view's endpoint at its path
        const { view } = this.page;
        const { render } = view;
        const environment = BrowserEnvironment.server(this.environment, request.url);
        const { endpoints = {} } = render.mode === "static" ? await this.#loadPages() : {};
        const endpoint = endpoints[new URL(request.url).pathname];
        if (endpoint !== undefined && render.mode === "static") {
            return await endpoint({ request, origin: render.origin, environment });
        }

        // link the context, stylesheets, preloads and entry at the end of the head
        const context =
            head.context === undefined
                ? []
                : [
                      `<script type="application/json" id="${CONTEXT_ELEMENT}">${JSON.stringify(head.context).replaceAll("<", "\\u003c")}</script>`,
                  ];
        const links = [
            ...context,
            ...(head.telemetry === undefined
                ? []
                : [`<meta name="${TELEMETRY_META}" content="${escape(head.telemetry)}">`]),
            ...head.stylesheets.map((href) => `<link rel="stylesheet" href="${escape(href)}">`),
            ...head.preloads.map((href) => `<link rel="modulepreload" href="${escape(href)}">`),
            `<script type="module" src="${escape(head.script)}" async></script>`,
        ].join("");

        // stream the page under the request, as the router reads its path from it, refusing a shell without a head
        const { provideRequestEvent } = await import("@solidjs/web/storage");
        const event = createRequestEvent(request);
        let failure: { readonly error: unknown } | undefined;
        const stream = provideRequestEvent(event, () =>
            renderToStream(() => this.page.render(mount, this.page.root(environment)), {
                // report each render error, keeping the first for a page that renders no shell
                onError: (error: unknown) => {
                    failure ??= { error };
                    globalThis.reportError(error);
                },
                onHead: () => {
                    throw ViewError.of(
                        "PAGE_HEAD_REQUIRED",
                        `head of view '${view.name}' is required: its document renders none`,
                    );
                },
            }),
        );

        // render until the shell flushes, holding its chunks, and splice the links into the chunk closing the head
        const held: string[] = [];
        let writable: { write: (chunk: string) => void; end: () => void } | undefined;
        let isLinked = false;
        let isEnded = false;
        const flushed = Promise.withResolvers<void>();
        stream.pipe({
            write: (chunk: string) => {
                // link the head once, into the chunk closing it
                const isClosing = !isLinked && chunk.includes("</head>");
                isLinked ||= isClosing;
                const spliced = isClosing ? chunk.replace("</head>", `${links}</head>`) : chunk;

                // hold it while no response reads, else write it
                if (writable === undefined) {
                    held.push(spliced);
                } else {
                    writable.write(spliced);
                }
                flushed.resolve();
            },
            end: () => {
                isEnded = true;
                writable?.end();
                flushed.resolve();
            },
        });
        await flushed.promise;

        // fail a page whose render ended before its shell, as no status can follow a body
        if (held.length === 0 && failure !== undefined) {
            throw failure.error;
        }

        // pass the held chunks on, then every later one as it flushes
        const linked = {
            pipe: (target: { write: (chunk: string) => void; end: () => void }) => {
                for (const chunk of held.splice(0)) {
                    target.write(chunk);
                }
                writable = target;
                if (isEnded) {
                    target.end();
                }
            },
        };

        return await createSSRResponse(linked, event);
    }

    /** Load the view's pages module once. */
    #loadPages(): Promise<StaticSite> {
        this.#pages ??= loadPages(this.page.view);

        return this.#pages;
    }
}

/** Start rendering a package's views as their visitors, reading stores through the host. */
export function serveViews(
    views: readonly View[],
    declared: { readonly theme?: ThemeDescription },
): ViewsStart {
    return (context) => {
        // open each view's page once, as its component and shell never change while the workload runs
        const server = present(context.server, "the server context of the installation with views");
        const ephemeral = present(context.ephemeral, "the memory databases of the views workload");
        const { environment } = context;
        const pages = new Map<string, Promise<PageServer>>();
        const page = (name: string) => {
            let opened = pages.get(name);
            if (opened === undefined) {
                const view = views.find((each) => each.name === name);
                if (view === undefined) {
                    throw ViewError.of("VIEW_NOT_FOUND", `view '${name}' not found`);
                }
                opened = PageServer.open(view, declared, environment);
                pages.set(name, opened);
            }

            return opened;
        };

        return {
            packages: [],
            views: {
                fetch: async (request) => {
                    // read what to render, and reach the stores as its visitor until the render ends
                    const asked = RenderRequest.parse(await request.json());
                    const rendering = await page(asked.view);
                    const opened = new AsyncDisposableStack();
                    const remote = server.act(asked.visitor);
                    opened.defer(() => remote.close());

                    // render under a mount whose scopes follow the visitor's replicas, closing them once the page streamed
                    try {
                        const mount = renderMount(
                            asked,
                            rendering.page.view,
                            { environment, remote, ephemeral },
                            opened,
                        );
                        const answered = await rendering.respond(
                            new Request(asked.url),
                            { ...asked.head, context: asked.context },
                            mount,
                        );

                        return closing(answered, opened);
                    } catch (error) {
                        await opened.disposeAsync();
                        throw error;
                    }
                },
            },
        };
    };
}

/** Pass a response's body on, closing what its render opened once the body ends or cancels. */
function closing(response: Response, opened: AsyncDisposableStack): Response {
    // close at once for a response without a body
    const { body } = response;
    if (body === null) {
        void opened.disposeAsync();

        return response;
    }

    // read the body through, closing at its end or cancel
    const reader = body.getReader();
    const passed = new ReadableStream<Uint8Array>({
        pull: async (controller) => {
            const read = await reader.read();
            if (read.done) {
                controller.close();
                await opened.disposeAsync();
            } else {
                controller.enqueue(read.value);
            }
        },
        cancel: async (reason) => {
            await reader.cancel(reason);
            await opened.disposeAsync();
        },
    });

    return new Response(passed, response);
}

/** Build a render's mount: the visitor's scopes on replicas, and nothing else reachable. */
function renderMount(
    asked: RenderRequest,
    view: View,
    reached: {
        readonly environment: Environment;
        readonly remote: ObjectConnection;
        readonly ephemeral: EphemeralHost;
    },
    stack: AsyncDisposableStack,
): ViewMount {
    // refuse what a render must not do: call packages, the page's origin, or other stores
    const refuse = (what: string) => () => {
        throw ViewError.of(
            "RENDER_INVALID",
            `view '${view.name}' is invalid: it calls its mount's '${what}' while it renders`,
        );
    };

    // open each scope's replica once for the whole render, as every node of the scope shares it
    const replicas = new Map<string, Promise<ObjectClient>>();
    const open = (scope: string) => {
        const opening = replicas.get(scope) ?? openReplica(scope, view, asked, reached, stack);
        replicas.set(scope, opening);

        return opening;
    };

    return {
        context: asked.context,
        clients: () => new Map(),
        open,
        packages: new Map(),
        origin: refuse("origin"),
        connect: refuse("connect"),
    };
}

/** Open a memory replica of a scope's objects the view opens, until the render ends. */
async function openReplica(
    scope: string,
    view: View,
    asked: Pick<RenderRequest, "context" | "visitor">,
    reached: {
        readonly environment: Environment;
        readonly remote: ObjectConnection;
        readonly ephemeral: EphemeralHost;
    },
    stack: AsyncDisposableStack,
): Promise<ObjectClient> {
    // keep the types the view opens in the scope's role in memory, refusing a scope it opens none in
    const opened = view.opens(scope === asked.context.account ? account : space);
    if (opened.length === 0) {
        throw ViewError.of(
            "OBJECT_INVALID",
            `view '${view.name}' is invalid: it opens no objects in scope '${scope}'`,
        );
    }
    const named = ObjectType.keyed(opened);
    const { environment, remote, ephemeral } = reached;

    // open the replica on its own stack, which closes what it opened should the open fail
    await using replica = new AsyncDisposableStack();
    const database = await ephemeral.open(ObjectClient.tables(named));
    replica.defer(() => database.close());
    const client = await ObjectClient.start(
        { database, objects: named, scope, caller: asked.visitor.subject, environment, remote },
        replica,
    );

    // follow the scope as the visitor until the render ends, closing a replica opened after it ended
    if (stack.disposed) {
        throw ViewError.of(
            "RENDER_UNAVAILABLE",
            `render of view '${view.name}' is unavailable: it ended before scope '${scope}' opened`,
        );
    }
    stack.use(replica.move());

    return client;
}

/** Load a static view's pages module, refusing a view of another mode. */
function loadPages(view: View): Promise<StaticSite> {
    const { render } = view;
    if (render.mode !== "static") {
        throw ViewError.of(
            "VIEW_INVALID",
            `view '${view.name}' is invalid: it prerenders no pages`,
        );
    }

    return render.pages();
}

/** A static view's routes: the paths of its pages, its not-found page and its endpoints. */
export interface RouteTable {
    /** The paths prerendered below the view's base. */
    readonly paths: readonly string[];
    /** The path rendering the not-found page, none when absent. */
    readonly notFound?: string;
    /** The paths of its endpoints. */
    readonly endpoints: readonly string[];
}

/** Escape a URL for an HTML attribute. */
function escape(url: string): string {
    return url.replaceAll("&", "&amp;").replaceAll('"', "&quot;").replaceAll("<", "&lt;");
}
