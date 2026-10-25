import type { Environment } from "@destack/environment";
import type { ClientOf } from "@destack/object/client";
import type { Token } from "../credential/credential.ts";
import { ConnectionError } from "../error/error.ts";
import { type Integration, IntegrationIssuer } from "../object/integration.ts";
import { TokenGrant } from "../oauth/grant.ts";
import {
    type AuthorizationClient,
    type AuthorizationPage,
    AuthorizationServer,
    type DeviceAuthorization,
} from "../oauth/server.ts";
import type connectionPackage from "../package.ts";
import { schema } from "@destack/schema";
import type { Fetch } from "@destack/object";
import type { ObjectReference } from "@destack/sync";

/** The authorization server of a replicated integration, reached through its primary. */
export class AuthorizationProxy implements AuthorizationClient {
    /** The issuer identifier. */
    readonly issuer: string;
    /** The endpoints. */
    readonly endpoints: Integration["endpoints"];
    /** The integration. */
    readonly #reference: ObjectReference;
    /** The connections of the space storing the integration. */
    readonly #primary: ClientOf<typeof connectionPackage>;
    /** The server as the public client writes its pages. */
    readonly #pages: AuthorizationServer;

    /** Call a replicated integration's server through its primary, at the time of the work. */
    constructor(
        row: Integration,
        reference: ObjectReference,
        primary: ClientOf<typeof connectionPackage>,
        fetch: Fetch,
        now: number,
        environment: Pick<Environment, "crypto" | "random">,
    ) {
        // take the server, the integration and its primary
        this.issuer = IntegrationIssuer.url(row.issuer);
        this.endpoints = row.endpoints;
        this.#reference = reference;
        this.#primary = primary;

        // write pages as the public client, which needs no secret
        this.#pages = new AuthorizationServer({
            issuer: this.issuer,
            endpoints: row.endpoints,
            client: { clientId: row.clientId, authentication: row.authentication },
            fetch,
            now,
            environment,
        });
    }

    /** Write the authorization page a person approves a code grant on. */
    authorizationUrl(request: AuthorizationPage): URL {
        return this.#pages.authorizationUrl(request);
    }

    /** Request a token through the primary, refusing grants without possession. */
    async token(grant: Readonly<Record<string, string>>): Promise<Token> {
        const parsed = TokenGrant.safeParse(grant);
        if (!parsed.success) {
            throw ConnectionError.of(
                "PRIMARY_REQUIRED",
                `only the primary of integration ${this.#reference.id} requests a ${grant["grant_type"] ?? "missing"} grant`,
            );
        }

        return this.#primary.integration.requestToken({ ...this.#target(), grant: parsed.data });
    }

    /** Start a device authorization through the primary. */
    authorizeDevice(
        scopes: readonly string[],
        resource: string | undefined,
    ): Promise<DeviceAuthorization> {
        return this.#primary.integration.authorizeDevice({
            ...this.#target(),
            request: { scopes: [...scopes], ...(resource === undefined ? {} : { resource }) },
        });
    }

    /** Revoke a token through the primary. */
    async revoke(token: string, hint: "refresh_token" | "access_token"): Promise<void> {
        await this.#primary.integration.revokeToken({
            ...this.#target(),
            revocation: { token, hint },
        });
    }

    /** Select the integration as the connection package's procedures take it. */
    #target() {
        const { scope, id } = this.#reference;

        return { scope, id: schema.identifier("integration").parse(id) };
    }
}
