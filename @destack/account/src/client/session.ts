import type { Environment } from "@destack/environment";
import { CallError, type Fetch, PackageMount, type Package } from "@destack/object";
import { type Bookmark, type ClientOptions, createClient } from "@destack/object/client";
import objectPackage from "@destack/object/package";
import type { PackageId } from "@destack/package";
import { present } from "@destack/schema";
import accountPackage from "../package.ts";
import { OAuthError } from "../authentication/token.ts";
import type { Credential } from "./credential.ts";
import type { TokenBinding } from "./issuer.ts";

/** A principal's calls to packages through its credential, its tokens added. */
export class Session<Principal extends Credential = Credential> {
    /** The credential adding the principal's tokens. */
    readonly credential: Principal;
    /** The universe's origin, which serves the universe's packages. */
    readonly universe: string;
    /** The positions the calls saw, which later calls wait for. */
    readonly bookmark: Bookmark;
    /** The environment the calls run in. */
    readonly environment: Environment;
    /** The fetch sending the calls. */
    readonly #send: Fetch;

    /** Call packages as a credential's principal, sending through a fetch. */
    constructor(
        credential: Principal,
        options: {
            readonly universe: string;
            readonly send: Fetch;
            readonly bookmark: Bookmark;
            readonly environment: Environment;
        },
    ) {
        // keep the credential, the universe, the bookmark, the environment and the fetch
        this.credential = credential;
        this.universe = options.universe;
        this.bookmark = options.bookmark;
        this.environment = options.environment;
        this.#send = options.send;
    }

    /** Create a fetch calling a package with the principal's tokens, bound as asked. */
    fetch(audience: PackageId, binding: TokenBinding = {}): Fetch {
        return (request) => this.#authorized(request, audience, binding);
    }

    /** Connect to a package the universe serves. */
    endpoint(audience: PackageId): ClientOptions {
        return {
            url: PackageMount.url(this.universe, audience),
            fetch: this.fetch(audience),
            bookmark: this.bookmark,
        };
    }

    /** Connect to a package mounted below an endpoint, bound as asked. */
    client<Handle extends Package>(handle: Handle, endpoint: string, binding: TokenBinding = {}) {
        return createClient<Handle["router"]>(
            handle,
            {
                url: PackageMount.url(endpoint, handle.id),
                fetch: this.fetch(handle.id, binding),
                bookmark: this.bookmark,
            },
            this.environment,
        );
    }

    /** Connect to the account package the universe serves. */
    accounts() {
        return createClient(accountPackage, this.endpoint(accountPackage.id), this.environment);
    }

    /** Connect to the object package the universe serves, which keeps the principal's grants. */
    objects() {
        return createClient(objectPackage, this.endpoint(objectPackage.id), this.environment);
    }

    /** Send a call with the principal's token, answering a consent it lacks as the callee would. */
    async #authorized(request: Request, audience: PackageId, binding: TokenBinding) {
        // add the token, answering a consent its exchange asks with the call error a callee sends
        const authorized = await this.credential.authorize(request, audience, binding).then(
            (added) => ({ added }),
            (error: unknown) => {
                if (!OAuthError.is(error, "CONSENT_REQUIRED")) {
                    throw error;
                }

                return { consent: present(CallError.of(error), "the call error of a consent") };
            },
        );

        return "consent" in authorized
            ? Response.json(authorized.consent.toJSON(), { status: authorized.consent.status })
            : this.#send(authorized.added);
    }
}
