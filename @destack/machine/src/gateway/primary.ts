import { type PackageVersion, PackageId } from "@destack/package";
import {
    CONNECTION_PATH,
    CONNECTION_SCOPE_PARAMETER,
    InstallationMount,
    PackageMount,
    SpaceMount,
    RequestBody,
} from "@destack/object";
import { HOME_PATH } from "@destack/view/declare";
import type {} from "@destack/package/import-meta";
import { MachineError } from "../error/index.ts";

/** The headers bound to the page's own origin: its credentials and its host. */
const PAGE_HEADERS = ["cookie", "authorization", "host"];

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

    /** Take the primary of a scope, sending as the page's person. */
    constructor(
        fields: Pick<ScopePrimary, "scope" | "package" | "url" | "home">,
        send: (request: Request, audience: PackageId) => Promise<Response>,
    ) {
        // set the scope, its store and the machine serving it
        this.scope = fields.scope;
        this.package = fields.package;
        this.url = fields.url;
        this.home = fields.home;
        this.#send = send;
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

        // connect to the scope's store, confined to the scope's rows
        if (path === CONNECTION_PATH) {
            const confined = new URL(`${this.url}${CONNECTION_PATH}`);
            confined.searchParams.set(CONNECTION_SCOPE_PARAMETER, this.scope);

            return this.#forward(request, confined.href, this.package.id);
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
                "home path not found: a page must connect to the universe's primary at its store",
            );
        }
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
