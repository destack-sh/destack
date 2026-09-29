import { ServiceMount, type Service, type ServiceRouter } from "@destack/service";
import { createClient } from "@destack/service/client";
import { ServiceError } from "@destack/service/error";
import type { Claim, ClaimOwner, Expiry, ObjectClaims } from "../claim/claim.ts";
import { Moved } from "../moved/moved.ts";
import type { Cell, Zone } from "../zone/zone.ts";

/** The universe's cell router: which cell serves each zone, where each cell answers, and which object claims each unique name. */
export abstract class Directory {
    // zones and cells

    /** Place a zone in its cell at an epoch and refuse an earlier epoch. */
    abstract place(zone: Zone): Promise<void>;

    /** Withdraw a zone its cell serves at an epoch. */
    abstract withdraw(zone: Zone): Promise<void>;

    /** Find the zone of a scope's databases. */
    abstract locate(scope: string): Promise<Zone | undefined>;

    /** List the zones a scope contains, such as an account's spaces. */
    abstract list(scope: string): Promise<readonly Zone[]>;

    /** Mark a zone its cell serves at an epoch as moving to a target cell. */
    abstract move(zone: Zone, target: string): Promise<void>;

    /** Follow the zones moving to a cell until the signal aborts. */
    abstract incoming(cell: string, signal: AbortSignal): AsyncIterable<readonly Zone[]>;

    /** Record the URL a cell answers at. */
    abstract publish(cell: string, scope: string, endpoint: string): Promise<void>;

    /** Read a cell, absent before it published an endpoint. */
    abstract cell(id: string): Promise<Cell | undefined>;

    // claims

    /** Reserve a request's claims until its write commits and refuse names of other objects. */
    abstract claim(claims: readonly Claim[], requestId: string, now: number): Promise<void>;

    /** Confirm a request's reserved claims and release the names its objects dropped. */
    abstract confirm(requestId: string, owned: readonly ObjectClaims[]): Promise<void>;

    /** Release the reservations of a request with a failed write. */
    abstract release(requestId: string): Promise<void>;

    /** Replace an object's claims after a write without reservations and refuse names of other objects. */
    abstract replace(owned: ObjectClaims, requestId: string, now: number): Promise<void>;

    /** Find the object owning a confirmed name. */
    abstract owner(index: string, key: string): Promise<ClaimOwner | undefined>;

    /** List the expired reservations of some indexes, and the next deadline. */
    abstract expired(indexes: readonly string[], now: number): Promise<Expiry>;

    // clients

    /** Create a client of a service where a cell mounts it, following a moved scope to its new cell. */
    client<Router extends ServiceRouter>(
        service: Service<Router>,
        cell: string,
        fetch: (request: Request) => Promise<Response>,
    ) {
        // resolve the current cell's endpoint for every request
        let current = cell;
        const endpoint = async () => {
            const found = await this.cell(current);
            if (found === undefined) {
                throw new ServiceError("NOT_FOUND", {
                    message: `cell ${current} publishes no endpoint`,
                });
            }

            return `${found.endpoint.replace(/\/+$/, "")}${ServiceMount.path(service.package.id)}`;
        };

        return createClient(service, {
            url: endpoint,
            fetch: async (request, options) => {
                // send the request and keep its body for the moved scope's cell
                const sent = new Request(request, options);
                const body = sent.body === null ? null : await sent.clone().arrayBuffer();
                const from = await endpoint();
                const response = await fetch(sent);
                const moved = response.status === 421 ? await movedTo(response) : undefined;
                if (moved === undefined) {
                    return response;
                }

                // resend once to the cell the scope moved to
                current = moved.cell;
                const target = `${await endpoint()}${sent.url.slice(from.length)}`;

                return fetch(
                    new Request(target, { method: sent.method, headers: sent.headers, body }),
                );
            },
        });
    }
}

/** Read the cell a MOVED response points to, absent for other responses. */
async function movedTo(response: Response): Promise<Moved | undefined> {
    const body = (await response.clone().json()) as { code?: unknown; data?: unknown };
    const moved = Moved.schema.safeParse(body.data);

    return body.code === "MOVED" && moved.success ? moved.data : undefined;
}
