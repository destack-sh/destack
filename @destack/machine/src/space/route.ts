import {
    InstallationOrigin,
    S3Origin,
    SpaceAddress,
    type SpaceOrigin,
} from "@destack/account/address";
import { Directory } from "@destack/account/client";
import * as accountObject from "@destack/account/object";
import { type Identifier, present, schema } from "@destack/schema";
import { Egress, InstallationMount, RequestBody, SpaceMount } from "@destack/object";
import { type Clock, copyRequest, Forwarded } from "@destack/environment";
import { MachineError } from "../error/index.ts";
import { SPACE_RETRY_MILLISECONDS } from "./space.ts";
import { CALLER_HEADER } from "@destack/access";
import {
    CLIENT_HEAD_HEADER,
    ClientHead,
    PARTITION_HEADER,
    PARTITION_PARAMETER,
    ROUTE_HEADER,
} from "@destack/object/workload";

/** The header with the space a machine routes a request to. */
export const SPACE_HEADER = "Destack-Space";

/** The headers only a machine and its hosts write, which a client's request loses. */
const MACHINE_HEADERS = [ROUTE_HEADER, CLIENT_HEAD_HEADER, CALLER_HEADER] as const;

/** The path of the stack service's spaces below an account. */
const SPACES = /\/accounts\/[^/]+\/spaces$/u;

/** The path of the replica's stream, which the follower's space answers for one below it. */
const STREAM_PATH = "/store/stream";

/** The paths of the procedures whose bodies have the scope they act in. */
const BODY_SCOPED = /\/store\/(?:stream|push|watermark|call|broadcast)$/u;

/** The partition in a scoped route's path: a space, a device or a machine. */
const SCOPED = /\/(?:spaces|devices|machines)\/((?:space|device|machine)-[0-9a-f-]+)(?=\/|$)/gu;

/** The identifier of a partition: a space, a device or a machine. */
const PARTITION = /^(?:space|device|machine)-[0-9a-f-]+$/u;

/** The identifier schema of spaces. */
const SPACE_IDENTIFIER = schema.identifier("space");

/** The status moving a request at a former name for good, method and body kept, RFC 9110. */
const RENAMED_STATUS = 308;

/** The status moving a request to its space's machine for now, method and body kept, RFC 9110. */
const PLACED_STATUS = 307;

/** A request to a space a machine serves, with the request below an explicit space prefix. */
export interface SpaceRequest {
    /** The space the request is for. */
    readonly space: Identifier<"space">;
    /** The request the space's machine serves. */
    readonly request: Request;
    /** Where the request moves: to the space's current name, or to the machine serving it. */
    readonly moved?: SpaceMove;
}

/** Where a request to a space moves, and why. */
export interface SpaceMove {
    /** Whether the request named a non-current address, or another machine's space. */
    readonly kind: "renamed" | "placed";
    /** The URL the request moves to. */
    readonly url: string;
}

/** The partition a request addresses: the innermost scoped route's, else its scoped body's. */
export const PartitionRequest = {
    /** Read the partition of a request: its header's, its path's or its body's. */
    async of(request: Request, pathname: string): Promise<string | undefined> {
        return (
            PartitionRequest.header(request) ??
            PartitionRequest.path(pathname) ??
            (await PartitionRequest.body(request, pathname))
        );
    },

    /** Read the partition in a request's header, else a socket's in its query. */
    header(request: Request): string | undefined {
        return (
            request.headers.get(PARTITION_HEADER) ??
            new URL(request.url).searchParams.get(PARTITION_PARAMETER) ??
            undefined
        );
    },

    /** Read the innermost partition of a scoped route's path. */
    path(pathname: string): string | undefined {
        return [...pathname.matchAll(SCOPED)].at(-1)?.[1];
    },

    /** Read the partition of a scoped or JSON body: a stream's follower, else its scope. */
    async body(request: Request, pathname: string): Promise<string | undefined> {
        // read no body of other requests
        const isJson = request.headers.get("content-type")?.includes("json") === true;
        if (request.method !== "POST" || (!isJson && !BODY_SCOPED.test(pathname))) {
            return undefined;
        }

        // take the follower's partition for a stream, else the body's scope, within the store's body limit
        const body = await RequestBody.json(request.clone(), "a routed request");
        const { data: parsed } = schema
            .looseObject({ scope: schema.string(), target: schema.string().exactOptional() })
            .safeParse(body);
        const isStream = pathname.endsWith(STREAM_PATH) && PARTITION.test(parsed?.target ?? "");

        return isStream ? parsed?.target : parsed?.scope;
    },
};

/** Find the space a request is for, as a machine routes it to the space's object. */
export const SpaceRequest = {
    /** Find the spaces a machine serves by address or identifier in its directory replica. */
    served(
        directory: Directory,
        machine: Identifier<"machine">,
        clock: Pick<Clock, "now">,
    ): SpaceLookup {
        // require a space's placement on the machine
        const serves = async (space: Identifier<"space">) => {
            const [placed] = await accountObject.Placement.find(directory.snapshot, [space]);

            return placed?.machine === machine;
        };

        return {
            serves,
            find: async (address) => {
                // resolve the address's space, then require its placement
                const found = await SpaceRequest.resolve(directory, address);

                return found !== undefined && (await serves(found.space)) ? found : undefined;
            },
            locate: async (space) => {
                // read where the space's machine answers, unavailable until the space is placed
                const placed = await accountObject.Placement.find(directory.snapshot, [space]);
                if (placed.length === 0) {
                    throw MachineError.of(
                        "SPACE_UNAVAILABLE",
                        `'${space}' is unavailable: it is not placed yet`,
                        { data: { retryAfter: SPACE_RETRY_MILLISECONDS } },
                    );
                }

                return (await directory.locate(space, clock.now())).url;
            },
        };
    },

    /** Redirect a request to where it moves: a former name's for good, a placed space's for now. */
    move(moved: SpaceMove): Response {
        return Response.redirect(
            moved.url,
            moved.kind === "renamed" ? RENAMED_STATUS : PLACED_STATUS,
        );
    },

    /** Find the space at an address, with its current origin when the address is a former one. */
    async resolve(directory: Directory, address: SpaceAddress): Promise<SpaceMatch | undefined> {
        // resolve the address's space
        const named = await directory.find(accountObject.space, address);
        if (named === undefined) {
            return undefined;
        }
        const space = SPACE_IDENTIFIER.parse(named.id);

        // read where it answers now when the address names it by a former handle or name
        const current =
            named.state === "former"
                ? present(await directory.origin(space), `the current address of ${space}`)
                : undefined;

        return current === undefined ? { space } : { space, current };
    },

    /** Copy a request a client sent without the headers only a machine and a host write. */
    received(request: Request): Request {
        // pass the client's head on with an instance's egress call, which its gateway verifies
        const isEgress = new URL(request.url).pathname.startsWith(`${Egress.path}/`);
        const dropped = isEgress
            ? MACHINE_HEADERS.filter((name) => name !== CLIENT_HEAD_HEADER)
            : MACHINE_HEADERS;

        // pass a request without them on as it is
        if (!dropped.some((name) => request.headers.has(name))) {
            return request;
        }

        // drop each
        const headers = new Headers(request.headers);
        for (const name of dropped) {
            headers.delete(name);
        }

        return copyRequest(request, { headers });
    },

    /** Read the space of a request at the machine's own hosts. */
    async of(
        given: Request,
        origins: Pick<SpaceLookup, "find"> & {
            /** The domain the installation origins and S3 origins sit below. */
            readonly domain: string;
        },
    ): Promise<SpaceRequest | undefined> {
        // drop the headers only a machine hands on
        const request = SpaceRequest.received(given);

        // read the request's installation origin, S3 origin and path
        const origin = InstallationOrigin.of(request, origins.domain);
        const url = new URL(request.url);
        const s3 = S3Origin.of(request, origins.domain);
        const installed = InstallationMount.of(request);
        const explicit = SpaceMount.parse(url.pathname);
        const scoped = PartitionRequest.path(url.pathname);

        // take the space of an installation origin
        if (origin !== undefined) {
            const found = await origins.find(SpaceAddress.of(origin));

            return found === undefined
                ? undefined
                : located(request, found, origin, origins.domain);
        }
        // take the space of an S3 request at its host name
        else if (s3 !== undefined) {
            return addressed(request, s3);
        }
        // take the space of an installation's mount, leaving the mount
        else if (installed !== undefined) {
            return isSpace(installed.spaceId) ? addressed(request, installed.spaceId) : undefined;
        }
        // take the space below an explicit prefix
        else if (explicit !== undefined) {
            if (!isSpace(explicit.spaceId)) {
                return undefined;
            }
            url.pathname = explicit.path;

            return addressed(copyRequest(request, {}, url.href), explicit.spaceId);
        }
        // take the innermost space of a scoped route
        else if (scoped !== undefined && isSpace(scoped)) {
            return addressed(request, scoped);
        }
        // take the space of a creation's or a scoped procedure's body
        else if (request.method === "POST") {
            const space = await bodySpace(request, url.pathname);

            return space === undefined ? undefined : addressed(request, space);
        }
        // take no space otherwise
        else {
            return undefined;
        }
    },

    /** Read the space of a request at a name: its origin, its space prefix, else its route. */
    async host(
        given: Request,
        names: SpaceLookup & {
            /** The domain the installation origins and S3 origins sit below. */
            readonly domain: string;
            /** Find the route a host's path takes among the spaces served here, absent for none. */
            readonly route: (
                host: string,
                path: string,
                serves: (scope: string) => Promise<boolean>,
            ) => Promise<{ readonly scope: string; readonly id: Identifier<"route"> } | undefined>;
        },
    ): Promise<SpaceRequest | undefined> {
        // drop the machine's headers, then take the space of an installation or S3 origin served here
        const request = SpaceRequest.received(given);
        const origin = InstallationOrigin.of(request, names.domain);
        const s3 = S3Origin.of(request, names.domain);
        const found = origin === undefined ? undefined : await names.find(SpaceAddress.of(origin));
        if (origin !== undefined && found !== undefined) {
            return located(request, found, origin, names.domain);
        } else if (s3 !== undefined && (await names.serves(s3))) {
            return addressed(request, s3);
        }

        // read the space below an explicit prefix, and where it is placed when not here
        const url = new URL(request.url);
        const explicit = SpaceMount.parse(url.pathname);
        const space =
            explicit !== undefined && isSpace(explicit.spaceId) ? explicit.spaceId : undefined;
        const isServed = space !== undefined && (await names.serves(space));
        const placed =
            space === undefined || isServed
                ? undefined
                : new URL(`${url.pathname}${url.search}`, await names.locate(space));

        // serve a space placed here below its prefix
        if (explicit !== undefined && space !== undefined && isServed) {
            url.pathname = explicit.path;

            return addressed(copyRequest(request, {}, url.href), space);
        }
        // move a request for a space placed elsewhere to its machine, unless it is at that machine's name
        else if (space !== undefined && placed !== undefined && placed.host !== url.host) {
            return { space, request, moved: { kind: "placed", url: placed.href } };
        }

        // take the route a space served here has for the host and path, such as a relay's for other names
        const serves = (scope: string) =>
            isSpace(scope) ? names.serves(scope) : Promise.resolve(false);
        const route = await names.route(url.hostname, url.pathname, serves);

        return route === undefined ? undefined : routed(request, route);
    },
};

/** The space an address names, with its current origin for a former address. */
export interface SpaceMatch {
    /** The space. */
    readonly space: Identifier<"space">;
    /** The space's current name and handle, given for an address with a former one. */
    readonly current?: SpaceOrigin;
}

/** The spaces a machine serves, found by their origins' addresses. */
export interface SpaceLookup {
    /** Find the space of an origin's address, absent for none served here. */
    readonly find: (address: SpaceAddress) => Promise<SpaceMatch | undefined>;
    /** Report whether the machine serves a space. */
    readonly serves: (space: Identifier<"space">) => Promise<boolean>;
    /** Read the URL of the machine serving a space, refusing a space not placed yet. */
    readonly locate: (space: Identifier<"space">) => Promise<string>;
}

/** Address a request to its route's space, with the route and the client's head. */
function routed(
    request: Request,
    route: { readonly scope: string; readonly id: Identifier<"route"> },
): SpaceRequest {
    // mark the request with its route and the client's head
    const headers = new Headers(request.headers);
    headers.set(ROUTE_HEADER, route.id);
    headers.set(CLIENT_HEAD_HEADER, ClientHead.write(request));

    return addressed(copyRequest(request, { headers }), SPACE_IDENTIFIER.parse(route.scope));
}

/** Read the space of a request body. */
async function bodySpace(
    request: Request,
    pathname: string,
): Promise<Identifier<"space"> | undefined> {
    const isJson = request.headers.get("content-type")?.includes("json") === true;

    // take the new space of a creation
    if (SPACES.test(pathname)) {
        const body = await RequestBody.json(request.clone(), "a space creation");
        const created = schema.looseObject({ id: SPACE_IDENTIFIER }).safeParse(body);

        return created.success ? created.data.id : undefined;
    }
    // take the follower's space for a stream, else the scope of a scoped procedure's or a JSON body
    else if (isJson || BODY_SCOPED.test(pathname)) {
        const space = await PartitionRequest.body(request, pathname);

        return isSpace(space) ? space : undefined;
    }
    // take no space otherwise
    else {
        return undefined;
    }
}

/** Add a request's space to its header. */
function addressed(request: Request, space: Identifier<"space">): SpaceRequest {
    const headers = new Headers(request.headers);
    headers.set(SPACE_HEADER, space);

    return { space, request: copyRequest(request, { headers }) };
}

/** Address a request at an installation origin, moving one at a former name to the current. */
function located(
    request: Request,
    found: SpaceMatch,
    origin: InstallationOrigin,
    domain: string,
): SpaceRequest {
    // write the current origin: the space's current name, its defaults left out
    const addressedHere = addressed(request, found.space);
    const { current } = found;
    const url = new URL(request.url);
    const hostname = InstallationOrigin.format(
        current === undefined
            ? origin
            : { ...origin, space: current.space, handle: current.handle },
        domain,
    );

    // address a request asked at the current origin as it is, else move it there for good
    const asked = new URL(`http://${Forwarded.host(request)}`).hostname;
    if (hostname === asked) {
        return addressedHere;
    }
    url.hostname = hostname;

    return { ...addressedHere, moved: { kind: "renamed", url: url.href } };
}

/** Decide whether a value is a space's identifier. */
function isSpace(value: unknown): value is Identifier<"space"> {
    return SPACE_IDENTIFIER.safeParse(value).success;
}
