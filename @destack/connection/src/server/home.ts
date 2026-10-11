import type { ServiceClient } from "@destack/service/client";
import type { Token } from "../credential/credential.ts";
import { ConnectionError } from "../error/error.ts";
import type { Integration } from "../object/integration.ts";
import { TokenGrant } from "../oauth/grant.ts";
import {
    type AuthorizationClient,
    type AuthorizationPage,
    AuthorizationServer,
    type DeviceAuthorization,
} from "../oauth/server.ts";
import type { connectionService } from "../service/service.ts";
import { type Clock, schema } from "@destack/schema";
import type { Fetch } from "@destack/service";
import type { ObjectReference } from "@destack/sync";

/** A copied integration's client: its authorization pages written here, its authenticated requests sent through its home, which keeps the secret. */
export class HomeClient implements AuthorizationClient {
    /** The issuer identifier. */
    readonly issuer: string;
    /** The endpoints. */
    readonly endpoints: Integration["endpoints"];
    /** The integration. */
    readonly #reference: ObjectReference;
    /** The connection service of the space keeping the integration. */
    readonly #home: ServiceClient<typeof connectionService>;
    /** The server as the public client writes its pages. */
    readonly #pages: AuthorizationServer;

    /** Call a copied integration's server through its home, on the store's clock. */
    constructor(
        row: Integration,
        reference: ObjectReference,
        home: ServiceClient<typeof connectionService>,
        fetch: Fetch,
        clock: Clock,
    ) {
        // keep the server, the integration and its home
        this.issuer = row.issuer;
        this.endpoints = row.endpoints;
        this.#reference = reference;
        this.#home = home;

        // write pages as the public client, which needs no secret
        this.#pages = new AuthorizationServer({
            issuer: row.issuer,
            endpoints: row.endpoints,
            client: { clientId: row.clientId, authentication: row.authentication },
            fetch,
            clock,
        });
    }

    /** Write the authorization page a person approves a code grant on. */
    authorizationUrl(request: AuthorizationPage): URL {
        return this.#pages.authorizationUrl(request);
    }

    /** Request a token through the home, refusing a grant that proves no possession, such as client credentials. */
    async token(grant: Readonly<Record<string, string>>): Promise<Token> {
        const parsed = TokenGrant.safeParse(grant);
        if (!parsed.success) {
            throw new ConnectionError(
                "INVALID_AUTHORIZATION",
                `a ${grant["grant_type"] ?? "missing"} grant proves no possession, so only the home of integration ${this.#reference.id} requests it`,
            );
        }

        return this.#home.integration.requestToken({ ...this.#target(), grant: parsed.data });
    }

    /** Start a device authorization through the home. */
    authorizeDevice(
        scopes: readonly string[],
        resource: string | undefined,
    ): Promise<DeviceAuthorization> {
        return this.#home.integration.authorizeDevice({
            ...this.#target(),
            request: { scopes: [...scopes], ...(resource === undefined ? {} : { resource }) },
        });
    }

    /** Revoke a token through the home. */
    async revoke(token: string, hint: "refresh_token" | "access_token"): Promise<void> {
        await this.#home.integration.revokeToken({
            ...this.#target(),
            revocation: { token, hint },
        });
    }

    /** Select the integration as the connection service's procedures take it. */
    #target() {
        const { scope, id } = this.#reference;

        return { scope, id: schema.identifier("integration").parse(id) };
    }
}
