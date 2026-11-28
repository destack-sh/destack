import { ExchangeSubject } from "@destack/access/token";
import { schema } from "@destack/schema";
import { ViewContext } from "./context.ts";

/** What a page's head links: its entry, stylesheets, preloads, telemetry endpoint and context. */
export const PageHead = schema.object({
    /** The module the page loads. */
    script: schema.string().min(1),
    /** The stylesheets the page links, in order. */
    stylesheets: schema.array(schema.string()).readonly(),
    /** The modules the page preloads beside its entry. */
    preloads: schema.array(schema.string()).readonly(),
    /** The path the origin serving the page takes its telemetry at, absent where it takes none. */
    telemetry: schema.string().exactOptional(),
    /** What the host gives a client view, absent on a static page. */
    context: ViewContext.exactOptional(),
});
/** What a page's head links. */
export type PageHead = schema.Infer<typeof PageHead>;

/** What a host asks its view workload for: a view at a page's address, for its visitor. */
export const RenderRequest = schema.object({
    /** The view's name in its package. */
    view: schema.string().min(1),
    /** The page's address, which the view's router reads. */
    url: schema.url(),
    /** What the page's head links beside its context. */
    head: PageHead.omit({ context: true }),
    /** What the host gives the view. */
    context: ViewContext,
    /** Whom the render reads as: the person under their grant within their session, or anyone. */
    visitor: ExchangeSubject,
});
/** What a host asks its view workload for. */
export type RenderRequest = schema.Infer<typeof RenderRequest>;
