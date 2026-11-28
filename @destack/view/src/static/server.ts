import type { Environment } from "@destack/environment";
import { BrowserEnvironment } from "@destack/environment/browser";
import { account, space } from "@destack/account/object";
import { ObjectType } from "@destack/object";
import type { ObjectConnection } from "@destack/object/client";
import { ObjectClient } from "@destack/object/device";
import type { EphemeralHost, Workload, WorkloadContext } from "@destack/object/workload";
import { present } from "@destack/schema";
import type { ThemeDescription } from "@destack/theme";
import type { StaticSite, View } from "../declare/view.ts";
import { CONTEXT_ELEMENT } from "../declare/context.ts";
import { type PageHead, RenderRequest } from "../declare/render.ts";
import { ViewError } from "../error/error.ts";
import type { ViewMount } from "../page/context.ts";
import { createRequestEvent, createSSRResponse, renderToStream } from "@solidjs/web";
import { TELEMETRY_META, ViewPage } from "./page.ts";

/** A view on a server: its page's endpoints and HTML, and its renders as a visitor. */
export class ViewServer {
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
    ): Promise<ViewServer> {
        return new ViewServer(await ViewPage.open(view, declared), environment);
    }

    /** Start a package's view workload: render each view its host asks for as the visitor. */
    static start(
        views: readonly View[],
        declared: { readonly theme?: ThemeDescription },
    ): Workload["start"] {
        return (context) => {
            // open each view's server once, as a page never changes while the workload runs
            const servers = new Map<string, Promise<ViewServer>>();
            const server = (name: string) => {
                // find the view, refusing one the package lacks
                const view = views.find((each) => each.name === name);
                if (view === undefined) {
                    throw ViewError.of("VIEW_NOT_FOUND", `view '${name}' not found`);
                }

                // open its server on first use
                const opening =
                    servers.get(name) ?? ViewServer.open(view, declared, context.environment);
                servers.set(name, opening);

                return opening;
            };

            // render each asked view as its visitor
            return {
                packages: [],
                view: {
                    fetch: async (request) => {
                        const asked = RenderRequest.parse(await request.json());

                        return await (await server(asked.view)).render(asked, context);
                    },
                },
            };
        };
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
        const { page } = this;
        const { view } = page;
        const { render } = view;
        const rendered = BrowserEnvironment.server(this.environment, request.url);
        const { endpoints = {} } = render.mode === "static" ? await this.#loadPages() : {};
        const endpoint = endpoints[new URL(request.url).pathname];
        if (endpoint !== undefined && render.mode === "static") {
            return await endpoint({ request, origin: render.origin, environment: rendered });
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

        // stream the page under the request, refusing a document without a head
        const { provideRequestEvent } = await import("@solidjs/web/storage");
        const event = createRequestEvent(request);
        let failure: { readonly error: unknown } | undefined;
        const stream = provideRequestEvent(event, () =>
            renderToStream(() => page.render(mount, page.root(rendered)), {
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

        // hold the chunks until the shell flushes, splicing the links into the chunk closing the head
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

    /** Render the view as a visitor, its scopes on replicas closed once the page streamed. */
    async render(
        asked: RenderRequest,
        context: Pick<WorkloadContext, "server" | "ephemeral">,
    ): Promise<Response> {
        // reach the stores as the visitor until the render ends
        const server = present(context.server, "the server context of the installation with views");
        const ephemeral = present(context.ephemeral, "the memory databases of the view workload");
        const opened = new AsyncDisposableStack();
        const remote = server.connectAs(asked.visitor);
        opened.defer(() => remote.close());

        // render under a mount whose scopes follow the visitor's replicas, closing them once the page streamed
        try {
            const answered = await this.respond(
                new Request(asked.url),
                { ...asked.head, context: asked.context },
                this.#mount(asked, { remote, ephemeral }, opened),
            );

            return await ViewServer.#closing(answered, opened);
        } catch (error) {
            await opened.disposeAsync();
            throw error;
        }
    }

    /** Load the view's pages module once. */
    #loadPages(): Promise<StaticSite> {
        this.#pages ??= loadPages(this.page.view);

        return this.#pages;
    }

    /** Build a render's mount: the visitor's scopes on replicas, and nothing else reachable. */
    #mount(
        asked: RenderRequest,
        reached: { readonly remote: ObjectConnection; readonly ephemeral: EphemeralHost },
        stack: AsyncDisposableStack,
    ): ViewMount {
        // refuse what a render must not do: call packages, the page's origin, or other stores
        const { view } = this.page;
        const refuse = (what: string) => () => {
            throw ViewError.of(
                "RENDER_INVALID",
                `view '${view.name}' is invalid: it calls its mount's '${what}' while it renders`,
            );
        };

        // open each scope's replica once for the whole render, as every node of the scope shares it
        const replicas = new Map<string, Promise<ObjectClient>>();
        const open = (scope: string) => {
            const opening = replicas.get(scope) ?? this.#replica(scope, asked, reached, stack);
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
    async #replica(
        scope: string,
        asked: Pick<RenderRequest, "context" | "visitor">,
        reached: { readonly remote: ObjectConnection; readonly ephemeral: EphemeralHost },
        stack: AsyncDisposableStack,
    ): Promise<ObjectClient> {
        // keep the types the view opens in the scope's role in memory, refusing a scope it opens none in
        const { view } = this.page;
        const opened = view.opens(scope === asked.context.account ? account : space);
        if (opened.length === 0) {
            throw ViewError.of(
                "OBJECT_INVALID",
                `view '${view.name}' is invalid: it opens no objects in scope '${scope}'`,
            );
        }
        const named = ObjectType.keyed(opened);

        // open the replica on its own stack, which closes what it opened should the open fail
        await using replica = new AsyncDisposableStack();
        const database = await reached.ephemeral.open(ObjectClient.tables(named));
        replica.defer(() => database.close());
        const client = await ObjectClient.start(
            {
                database,
                objects: named,
                scope,
                caller: asked.visitor.subject,
                environment: this.environment,
                remote: reached.remote,
            },
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

    /** Pass a response's body on, closing what its render opened once the body ends or cancels. */
    static async #closing(response: Response, opened: AsyncDisposableStack): Promise<Response> {
        // close at once for a response without a body
        const { body } = response;
        if (body === null) {
            await opened.disposeAsync();

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

/** Escape a URL for an HTML attribute. */
function escape(url: string): string {
    return url.replaceAll("&", "&amp;").replaceAll('"', "&quot;").replaceAll("<", "&lt;");
}
