import type { DatabaseConnection } from "@destack/db";
import { type Identifier, schema } from "@destack/schema";
import { InstallationMount, PackageMount } from "@destack/object";
import { copyRequest } from "@destack/environment";
import { CLIENT_HEAD_HEADER, ROUTE_HEADER } from "@destack/object/workload";
import { Route } from "@destack/stack/object";
import type { ViewMount } from "./origin.ts";
import type { SpaceAdapter } from "../space/space.ts";
import type {} from "@destack/package/import-meta";
import { MachineError } from "../error/index.ts";

/** What a routed request becomes: a redirect, an adapter's request, or a view's. */
export type RouteAnswer =
    | {
          /** The route redirects. */
          readonly kind: "redirect";
          /** The redirect. */
          readonly response: Response;
      }
    | {
          /** The route hands the request to an installation's adapter below its package's mount. */
          readonly kind: "adapter";
          /** The request below the installation's mount. */
          readonly request: Request;
      }
    | {
          /** The route serves an installation's views below its path. */
          readonly kind: "view";
          /** The installation whose views serve the request. */
          readonly installation: Identifier<"installation">;
          /** The views' mount below the route's path, with the view and object it fixes. */
          readonly mount: ViewMount;
      };

/** The requests a machine hands a space by the route it found. */
export const RouteRequest = {
    /** Read what a request the machine routed to the space becomes, absent for one it did not. */
    async mount(
        database: DatabaseConnection,
        space: Identifier<"space">,
        request: Request,
        adapters: (installationId: Identifier<"installation">) => readonly SpaceAdapter[],
    ): Promise<RouteAnswer | undefined> {
        // read the route the machine found
        const header = request.headers.get(ROUTE_HEADER);
        if (header === null) {
            return undefined;
        }
        const id = schema.identifier("route").parse(header);
        const found = await Route.find(database, space, id);
        if (found === undefined) {
            throw MachineError.of("ROUTE_NOT_FOUND", `space ${space} has no route ${id}`);
        }

        // answer a redirect
        const destination = Route.destination(found);
        const base = found.path === "/" ? "" : found.path;
        if (destination.kind === "redirect") {
            const response = new Response(null, {
                status: destination.status,
                headers: { location: destination.location },
            });

            return { kind: "redirect", response };
        }
        // hand the installation's adapter the request at its path, the path below the route's after it
        else if (destination.kind === "adapter") {
            const { installationId } = destination;
            const adapter = adapters(installationId).find(
                (each) => each.name === destination.adapter,
            );
            if (adapter === undefined) {
                throw MachineError.of(
                    "ROUTE_NOT_FOUND",
                    `route ${id} leads to adapter ${destination.adapter}, which installation ${installationId} does not declare`,
                );
            }
            const url = new URL(request.url);
            const mount = InstallationMount.url(request.url, space, installationId);
            const below = url.pathname.slice(base.length);
            const path = `${PackageMount.path(adapter.packageId)}${adapter.path}${below === "/" ? "" : below}`;

            return {
                kind: "adapter",
                request: copyRequest(request, {}, `${mount}${path}${url.search}`),
            };
        }
        // serve the installation's whole origin below the route's path
        else if (destination.kind === "installation") {
            return { kind: "view", installation: destination.installationId, mount: { base } };
        }
        // serve the installation's view below the route's path, opening the route's object
        else {
            const mount = {
                base,
                view: destination.view,
                ...(destination.object === null ? {} : { target: destination.object }),
            };

            return { kind: "view", installation: destination.installationId, mount };
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
