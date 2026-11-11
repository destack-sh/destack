import { type PackageVersion, PackageId } from "@destack/package";
import {
    InstallationMount,
    PackageMount,
    SpaceMount,
    ObjectTypePool,
    ObjectType,
    storeProcedures,
    RequestBody,
} from "@destack/object";
import { present, schema } from "@destack/schema";
import * as sync from "@destack/sync";
import { DynamicObjectClient } from "@destack/object/client";
import { HOME_PATH } from "@destack/view/declare";
import type { Environment } from "@destack/environment";
import type {} from "@destack/package/import-meta";
import { MachineError } from "../error/index.ts";

/** The path prefix of the store procedures below a service's mount. */
const STORE_PREFIX = "/store/";

/** The path of the replica push below a service's mount, which a page sends a scope's primary. */
const PUSH_PATH = present(storeProcedures.push["~orpc"].route.path, "the replica push's path");

/** The largest push a page sends a scope's primary, in bytes: 100 mutations of 10 KiB. */
const PUSH_BYTES = 1024 * 1024;

/** The headers bound to the page's own origin: its credentials and its host. */
const PAGE_HEADERS = ["cookie", "authorization", "host"];

/** A push a page sends a scope's primary: the scope and the mutations' calls. */
const Push = schema.looseObject({
    scope: schema.string(),
    mutations: schema.array(sync.Mutation),
});

/** Where a page's calls to a scope go through its origin: a space's machine, or the universe. */
export class ScopePrimary {
    /** The scope. */
    readonly scope: string;
    /** The package of the scope's rows: a space's stack, or the universe's account package. */
    readonly package: PackageVersion;
    /** The URL of that package's store at the scope's primary. */
    readonly url: string;
    /** Where the scope's primary lives. */
    readonly home:
        | {
              /** A space, served by a machine. */
              readonly kind: "space";
              /** The origin of the machine serving the space. */
              readonly origin: string;
          }
        | {
              /** The universe. */
              readonly kind: "universe";
          };
    /** Send a request to a package at the primary as the page's person. */
    readonly #send: (request: Request, audience: PackageId) => Promise<Response>;
    /** The environment the primary's client runs in, whose clock stamps its calls. */
    readonly #environment: Environment;

    /** Take the primary of a scope, sending as the page's person. */
    constructor(
        fields: Pick<ScopePrimary, "scope" | "package" | "url" | "home">,
        send: (request: Request, audience: PackageId) => Promise<Response>,
        environment: Environment,
    ) {
        // set the scope, its store and the machine serving it
        this.scope = fields.scope;
        this.package = fields.package;
        this.url = fields.url;
        this.home = fields.home;
        this.#send = send;
        this.#environment = environment;
    }

    /** Read the scope a request's path names below the home path. */
    static scopeOf(url: URL): string {
        const [scope = ""] = url.pathname.slice(HOME_PATH.length + 1).split("/");

        return scope;
    }

    /** Forward a page's request below the scope's home path to its primary, as its person. */
    async forward(request: Request, url: URL): Promise<Response> {
        // read the path below the scope
        const path = `/${url.pathname
            .slice(HOME_PATH.length + 1)
            .split("/")
            .slice(1)
            .join("/")}`;

        // push changes of the scope's rows alone to its store
        if (path === PUSH_PATH) {
            return this.#push(request);
        }
        // call the scope's store procedures
        else if (path.startsWith(STORE_PREFIX)) {
            return this.#forward(request, `${this.url}${path}${url.search}`, this.package.id);
        }
        // call an installation's package, or a package the space's store serves, by its mount
        else if (this.home.kind === "space") {
            const called = new Request(new URL(`${path}${url.search}`, this.home.origin).href);
            const installed = InstallationMount.of(called);
            const mounted = PackageMount.of(installed?.request ?? called);
            const below = installed === undefined ? SpaceMount.path(this.scope) : "";
            const target = new URL(`${below}${path}${url.search}`, this.home.origin).href;
            if (
                (installed !== undefined && installed.spaceId !== this.scope) ||
                mounted === undefined
            ) {
                throw MachineError.of(
                    "HOME_PATH_NOT_FOUND",
                    `home path not found: a page must call '${this.scope}' packages below '${[HOME_PATH, this.scope].join("/")}'`,
                );
            }

            return this.#forward(request, target, PackageId.parse(mounted.packageId));
        }
        // refuse anything else of the universe
        else {
            throw MachineError.of(
                "HOME_PATH_NOT_FOUND",
                "home path not found: a page must call the universe's primary at its store procedures",
            );
        }
    }

    /** Push a page's changes of the scope's rows to its store, refusing another scope's. */
    async #push(request: Request): Promise<Response> {
        // read the push, refusing one to another scope
        if (request.method !== "POST") {
            throw MachineError.of(
                "HOME_PATH_NOT_FOUND",
                `home path not found: a page must push to a scope's primary at '${HOME_PATH}/<scope>${PUSH_PATH}'`,
            );
        }
        const body = await RequestBody.json(request, "a push", PUSH_BYTES);
        const pushed = Push.parse(body);
        if (pushed.scope !== this.scope) {
            throw MachineError.of(
                "PUSH_DENIED",
                `push is forbidden: a push to the primary of '${this.scope}' can only change rows of '${this.scope}'`,
            );
        }

        // refuse a call to another scope, by the types the primary describes, leaving the grant to it
        const remote = new DynamicObjectClient({
            objects: [],
            scope: this.scope,
            endpoint: {
                url: this.url,
                fetch: (sent: Request) => this.#send(sent, PackageId.parse(this.package.id)),
            },
            environment: this.#environment,
        });
        const calls = pushed.mutations.flatMap((mutation) => mutation.calls);
        const types = ObjectTypePool.of([]).declare(
            await remote.describe(calls.map((call) => call.type)),
        );
        const isOutside = calls.some(
            (call) =>
                ObjectType.find(types, call)?.readScope(call.method, call.input) !== this.scope,
        );
        if (isOutside) {
            throw MachineError.of(
                "PUSH_DENIED",
                `push is forbidden: a push to the primary of '${this.scope}' can only change rows of '${this.scope}'`,
            );
        }

        // push to the primary without the page's own credentials
        return this.#send(
            new Request(`${this.url}${PUSH_PATH}`, {
                method: "POST",
                headers: ScopePrimary.#withoutPageHeaders(request.headers),
                body: JSON.stringify(body),
            }),
            PackageId.parse(this.package.id),
        );
    }

    /** Forward a request to a package at the primary, without the page's own credentials. */
    async #forward(request: Request, target: string, audience: PackageId): Promise<Response> {
        // read a call's body within the store's limit, which the target reads whole too
        const isBodiless = request.method === "GET" || request.method === "HEAD";
        const body = isBodiless ? undefined : await RequestBody.bytes(request, "a page's call");

        return this.#send(
            new Request(target, {
                method: request.method,
                headers: ScopePrimary.#withoutPageHeaders(request.headers),
                ...(body === undefined ? {} : { body }),
                signal: request.signal,
            }),
            audience,
        );
    }

    /** Copy a page's request headers without those bound to its origin. */
    static #withoutPageHeaders(given: Headers): Headers {
        const headers = new Headers(given);
        for (const name of PAGE_HEADERS) {
            headers.delete(name);
        }

        return headers;
    }
}
