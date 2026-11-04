import { Directory, InstallationOrigin, SpaceAddress } from "@destack/account/address";
import * as accountObject from "@destack/account/object";
import { type Identifier, schema } from "@destack/schema";
import { InstallationMount, RequestBody, SpaceMount } from "@destack/object";
import { copyRequest } from "@destack/environment";
import { CALLER_HEADER } from "@destack/access";
import { CLIENT_HEAD_HEADER, ClientHead } from "@destack/object/workload";

/** The header with the space a machine routes a request to. */
export const SPACE_HEADER = "Destack-Space";

/** The header with the route the machine found for a request to a name. */
export const ROUTE_HEADER = "Destack-Route";

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

/** A request to a space a machine serves, with the request below an explicit space prefix. */
export interface SpaceRequest {
    /** The space the request is for. */
    readonly space: Identifier<"space">;
    /** The request the space's machine serves. */
    readonly request: Request;
}

/** The partition a request addresses: the innermost scoped route's, else its scoped body's. */
export const PartitionRequest = {
    /** Read the partition of a request at a path, absent for a request addressing none. */
    async of(request: Request, pathname: string): Promise<string | undefined> {
        return (
            PartitionRequest.scoped(pathname) ?? (await PartitionRequest.body(request, pathname))
        );
    },

    /** Read the innermost partition of a scoped route's path. */
    scoped(pathname: string): string | undefined {
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

/** The host name of a space's buckets over S3: its label below an S3 domain. */
export const S3Host = {
    /** Write a space's S3 host below an S3 domain, such as s3.localhost. */
    of(space: Identifier<"space">, domain: string): string {
        return `${space}.${domain}`;
    },

    /** Read the space of an S3 host, absent for any other host. */
    space(hostname: string): Identifier<"space"> | undefined {
        const [label, service] = hostname.split(".");

        return service === "s3" && isSpace(label) ? label : undefined;
    },
};

/** Find the space a request is for, as a machine routes it to the space's object. */
export const SpaceRequest = {
    /** Find the space of an installation origin's address in the machine's directory replica. */
    served(
        directory: Directory,
        machine: Identifier<"machine">,
    ): (address: SpaceAddress) => Promise<string | undefined> {
        return async (address) => {
            // resolve the address's space, then require its placement on the machine
            const named = await directory.find(accountObject.space, address);
            const [placed] =
                named === undefined
                    ? []
                    : await accountObject.Placement.find(directory.snapshot, [
                          schema.identifier("space").parse(named.id),
                      ]);

            return placed?.machine === machine ? named?.id : undefined;
        };
    },

    /** Copy a request a client sent without the headers only a machine and a host write. */
    received(request: Request): Request {
        // pass a request without them on as it is
        if (!MACHINE_HEADERS.some((name) => request.headers.has(name))) {
            return request;
        }

        // drop each
        const headers = new Headers(request.headers);
        for (const name of MACHINE_HEADERS) {
            headers.delete(name);
        }

        return copyRequest(request, { headers });
    },

    /** Read the space of a request at the machine's own hosts. */
    async of(
        given: Request,
        origins: {
            /** The domain the installation origins sit below. */
            readonly domain: string;
            /** Find the space of an origin's address, absent for none. */
            readonly find: (address: SpaceAddress) => Promise<string | undefined>;
        },
    ): Promise<SpaceRequest | undefined> {
        // drop the headers only a machine hands on
        const request = SpaceRequest.received(given);

        // read the request's installation origin, S3 host and path
        const origin = InstallationOrigin.of(request, origins.domain);
        const url = new URL(request.url);
        const s3 = S3Host.space(url.hostname);
        const installed = InstallationMount.of(request);
        const explicit = SpaceMount.parse(url.pathname);
        const scoped = PartitionRequest.scoped(url.pathname);

        // take the space of an installation origin
        if (origin !== undefined) {
            const space = await origins.find(SpaceAddress.of(origin));

            return space === undefined
                ? undefined
                : addressed(request, SPACE_IDENTIFIER.parse(space));
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

    /** Read the space of a request to a name. */
    async named(
        given: Request,
        names: {
            /** The domain the installation origins sit below. */
            readonly domain: string;
            /** Find the space of an origin's address, absent for none. */
            readonly find: (address: SpaceAddress) => Promise<string | undefined>;
            /** Find the route a host's path takes, absent for none. */
            readonly route: (
                host: string,
                path: string,
            ) => Promise<{ readonly scope: string; readonly id: Identifier<"route"> } | undefined>;
        },
    ): Promise<SpaceRequest | undefined> {
        // drop the machine's headers, then take the space of an installation origin served here
        const request = SpaceRequest.received(given);
        const origin = InstallationOrigin.of(request, names.domain);
        const space = origin === undefined ? undefined : await names.find(SpaceAddress.of(origin));
        if (space !== undefined) {
            return addressed(request, SPACE_IDENTIFIER.parse(space));
        }

        // take the route the host and path take, with the route and the client's head
        const url = new URL(request.url);
        const route = await names.route(url.hostname, url.pathname);
        if (route === undefined) {
            return undefined;
        }
        const headers = new Headers(request.headers);
        headers.set(ROUTE_HEADER, route.id);
        headers.set(CLIENT_HEAD_HEADER, ClientHead.write(request));

        return addressed(copyRequest(request, { headers }), SPACE_IDENTIFIER.parse(route.scope));
    },
};

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

/** Decide whether a value is a space's identifier. */
function isSpace(value: unknown): value is Identifier<"space"> {
    return SPACE_IDENTIFIER.safeParse(value).success;
}
