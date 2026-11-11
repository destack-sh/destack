import type { DatabaseConnection } from "@destack/db";
import { type Identifier, schema } from "@destack/schema";
import { InstallationMount, PackageMount } from "@destack/object";
import { copyRequest } from "@destack/environment";
import { CLIENT_HEAD_HEADER, ROUTE_HEADER } from "@destack/object/workload";
import { Route, type RouteRule } from "@destack/stack/object";
import type { ViewMount } from "./origin.ts";
import type { SpaceAdapter } from "../space/space.ts";
import type {} from "@destack/package/import-meta";
import { MachineError } from "../error/index.ts";

/** A rule's filter of one kind. */
type FilterOf<Kind extends string> = Extract<
    NonNullable<RouteRule["filters"]>[number],
    { kind: Kind }
>;

/** What a route does with a request: respond itself, or hand it to an adapter, views or a site. */
export type RouteAction =
    | {
          /** The route returns a response itself: a redirect, or none for no matching rule. */
          readonly kind: "response";
          /** The response. */
          readonly response: Response;
      }
    | {
          /** The route hands the request to an installation's adapter below its package's mount. */
          readonly kind: "adapter";
          /** The request below the installation's mount. */
          readonly request: Request;
          /** Change the adapter's response as the rule's filters ask. */
          readonly respond: (response: Response) => Response;
      }
    | {
          /** The route serves an installation's views below its path. */
          readonly kind: "view";
          /** The installation whose views serve the request. */
          readonly installation: Identifier<"installation">;
          /** The views' mount below the route's path, with the view and object it fixes. */
          readonly mount: ViewMount;
          /** The request, its headers changed as the rule's filters ask. */
          readonly request: Request;
          /** Change the views' response as the rule's filters ask. */
          readonly respond: (response: Response) => Response;
      }
    | {
          /** The route sends the request on to another site at its fixed origin. */
          readonly kind: "url";
          /** The request to the site. */
          readonly request: Request;
          /** Change the site's response as the rule's filters ask. */
          readonly respond: (response: Response) => Response;
      };

/** The requests a machine hands a space by the route it found. */
export const RouteRequest = {
    /** Read what a request the machine routed to the space becomes, absent for one it did not. */
    async mount(
        database: DatabaseConnection,
        space: Identifier<"space">,
        request: Request,
        adapters: (installationId: Identifier<"installation">) => readonly SpaceAdapter[],
    ): Promise<RouteAction | undefined> {
        // read the route the machine found
        const header = request.headers.get(ROUTE_HEADER);
        if (header === null) {
            return undefined;
        }
        const id = schema.identifier("route").parse(header);
        const found = await Route.find(database, space, id);
        if (found === undefined) {
            throw MachineError.of("ROUTE_NOT_FOUND", `route '${id}' of space '${space}' not found`);
        }

        // take the rule serving the request, refusing one no rule matches
        const rule = Route.rule(found, request);
        if (rule === undefined) {
            return { kind: "response", response: new Response(null, { status: 404 }) };
        }

        // redirect, else change the request as the filters ask
        const filters = rule.filters ?? [];
        const redirect = filters.find(
            (filter): filter is FilterOf<"redirect"> => filter.kind === "redirect",
        );
        if (redirect !== undefined) {
            const location = RouteRequest.replace(new URL(request.url), found.path, redirect);
            const response = new Response(null, {
                status: redirect.status,
                headers: { location: location.href },
            });

            return { kind: "response", response };
        }
        const routed = filters.reduce((current, filter) => RouteRequest.filter(current, filter), {
            request,
            path: found.path,
        });
        const changed = routed.request;
        const respond = (response: Response) =>
            filters
                .filter(
                    (filter): filter is FilterOf<"response-headers"> =>
                        filter.kind === "response-headers",
                )
                .reduce((current, filter) => RouteRequest.headers(current, filter), response);

        // serve the entry
        const entry = rule.entry;
        const base = routed.path === "/" ? "" : routed.path;
        if (entry === undefined) {
            throw MachineError.of("ROUTE_NOT_FOUND", `entry of a rule of route '${id}' not found`);
        }
        // send the request on to the site, its origin fixed and the path below the route's appended
        else if (entry.kind === "url") {
            const url = new URL(changed.url);
            const site = new URL(entry.url);
            const below = url.pathname.slice(base.length);
            const target = `${site.origin}${site.pathname.replace(/\/$/u, "")}${below}${url.search}`;

            // send none of the visitor's credentials for this hostname to the other site
            const headers = new Headers(RouteRequest.strip(changed).headers);
            headers.delete("cookie");
            headers.delete("authorization");

            return { kind: "url", request: copyRequest(changed, { headers }, target), respond };
        }

        // serve an entry of the route's installation
        const installationId = found.installationId;
        if (installationId === null) {
            throw MachineError.of(
                "ROUTE_NOT_FOUND",
                `installation of an entry of route '${id}' not found`,
            );
        }
        // hand the installation's adapter the request at its path, the path below the route's after it
        else if (entry.kind === "adapter") {
            const adapter = adapters(installationId).find((each) => each.name === entry.adapter);
            if (adapter === undefined) {
                throw MachineError.of(
                    "ROUTE_NOT_FOUND",
                    `adapter '${entry.adapter}' of route '${id}' in installation '${installationId}' not found`,
                );
            }
            const url = new URL(changed.url);
            const mount = InstallationMount.url(changed.url, space, installationId);
            const below = url.pathname.slice(base.length);
            const path = `${PackageMount.path(adapter.packageId)}${adapter.path}${below === "/" ? "" : below}`;

            return {
                kind: "adapter",
                request: copyRequest(changed, {}, `${mount}${path}${url.search}`),
                respond,
            };
        }
        // serve the installation's whole origin below the route's path
        else if (entry.kind === "origin") {
            return {
                kind: "view",
                installation: installationId,
                mount: { base },
                request: changed,
                respond,
            };
        }
        // serve the installation's view below the route's path, opening the rule's object
        else {
            const mount = {
                base,
                view: entry.view,
                ...(entry.object === undefined ? {} : { target: entry.object }),
            };

            return { kind: "view", installation: installationId, mount, request: changed, respond };
        }
    },

    /** Change a request and the path its entry strips as one filter asks. */
    filter(
        routed: { readonly request: Request; readonly path: string },
        filter: NonNullable<RouteRule["filters"]>[number],
    ): { readonly request: Request; readonly path: string } {
        // replace the URL's path, the entry then reading the rewritten path whole
        const { request, path } = routed;
        if (filter.kind === "rewrite") {
            const url = RouteRequest.replace(new URL(request.url), path, filter);

            return { request: copyRequest(request, {}, url.href), path: "/" };
        }
        // set, add and remove the request's headers
        else if (filter.kind === "request-headers") {
            const headers = new Headers(request.headers);
            RouteRequest.modify(headers, filter);

            return { request: copyRequest(request, { headers }), path };
        }

        // leave a response's or a redirect's filter to the action
        return routed;
    },

    /** Replace a URL's scheme, hostname and path as a redirect or a rewrite asks. */
    replace(
        url: URL,
        path: string,
        filter: Pick<FilterOf<"redirect">, "scheme" | "hostname" | "path">,
    ): URL {
        // replace the scheme and the hostname
        const replaced = new URL(url.href);
        if (filter.scheme !== undefined) {
            replaced.protocol = `${filter.scheme}:`;
        }
        if (filter.hostname !== undefined) {
            replaced.hostname = filter.hostname;
        }

        // replace the whole path, or the prefix the route's path matched
        if (filter.path?.kind === "full") {
            replaced.pathname = filter.path.value;
        } else if (filter.path?.kind === "prefix") {
            const below = url.pathname.slice(path === "/" ? 0 : path.length);
            const prefix = filter.path.value === "/" ? "" : filter.path.value;
            replaced.pathname = `${prefix}${below}` || "/";
        }

        return replaced;
    },

    /** Change a response's headers as a filter asks. */
    headers(response: Response, filter: FilterOf<"response-headers">): Response {
        const headers = new Headers(response.headers);
        RouteRequest.modify(headers, filter);

        return new Response(response.body, {
            status: response.status,
            statusText: response.statusText,
            headers,
        });
    },

    /** Set, add and remove headers as a filter lists them. */
    modify(headers: Headers, filter: FilterOf<"request-headers" | "response-headers">): void {
        for (const [name, value] of Object.entries(filter.set ?? {})) {
            headers.set(name, value);
        }
        for (const [name, value] of Object.entries(filter.add ?? {})) {
            headers.append(name, value);
        }
        for (const name of filter.remove ?? []) {
            headers.delete(name);
        }
    },

    /** Copy a routed request without the machine's headers, which no view reads. */
    strip(request: Request): Request {
        // drop the route and the forwarded head
        const headers = new Headers(request.headers);
        headers.delete(ROUTE_HEADER);
        headers.delete(CLIENT_HEAD_HEADER);

        return copyRequest(request, { headers });
    },
};
