import { type DatabaseConnection, eq, Snapshot } from "@destack/db";
import type { Crypto } from "@destack/environment";
import { Digest, schema } from "@destack/schema";
import { OAuthError } from "../authentication/token.ts";
import { client, DEVICE_CLIENT_NAME } from "../object/client.ts";
import { OAUTH_SCOPES, type Application, application } from "../object/application.ts";
import {
    DOMAINS,
    InstallationOrigin,
    LOOPBACK_DOMAIN,
    OPEN_PATH,
    SpaceAddress,
} from "../address/address.ts";

/** The grant type of a device's sign-in by a code a person approves (RFC 8628 3.4). */
export const DEVICE_CODE = "urn:ietf:params:oauth:grant-type:device_code";

/** The grant type of a mailed sign-in code exchanged for a person's tokens on a new client. */
export const EMAIL_CODE = "urn:destack:params:oauth:grant-type:email-code";

/** The grants a device's client uses: a browser, a device code or a mailed code, then refreshes. */
const DEVICE_GRANTS: ReadonlySet<string> = new Set([
    "authorization_code",
    "refresh_token",
    DEVICE_CODE,
    EMAIL_CODE,
]);

/** The hosts of a native client's loopback redirect (RFC 8252 7.3). */
const LOOPBACK_HOSTS: ReadonlySet<string> = new Set(["127.0.0.1", "[::1]"]);

/** The path of a native client's loopback redirect. */
const LOOPBACK_PATH = "/callback";

/** The HTTP authentication scheme of a client's secret (RFC 6749 2.3.1). */
const BASIC = "Basic ";

/** The scopes an installation's origin may ask: the person and their profile, no refresh tokens. */
const INSTALLATION_SCOPES: readonly string[] = ["openid", "profile", "email"];

/** The grants an installation's origin uses: the code a person's sign-in returns to it. */
const INSTALLATION_GRANTS: ReadonlySet<string> = new Set(["authorization_code"]);

/** How the server knows a client: a device, a registered application, or an origin. */
export type OAuthClientRegistration =
    | {
          /** A device's client, known by its identifier and its loopback redirect. */
          readonly kind: "device";
      }
    | {
          /** An application people registered. */
          readonly kind: "application";
          /** Its registration. */
          readonly application: Application;
      }
    | {
          /** An installation's origin, its own identifier (OAuth Client ID Metadata Document). */
          readonly kind: "installation";
          /** The origin, such as https://notes--home--ada.destack.space. */
          readonly origin: string;
          /** The installation's name in the origin's label, null for a branch's digest. */
          readonly installation: string | null;
          /** The address of the installation's space, whose key signs the client's requests. */
          readonly space: string;
      };

/** A client of the authorization server: a device, an application, or an origin. */
export class OAuthClient {
    /** The client's public identifier. */
    readonly clientId: string;
    /** The name the consent and device pages show. */
    readonly name: string;
    /** How the server knows the client. */
    readonly registration: OAuthClientRegistration;

    /** Describe a client by its identifier, name and registration. */
    private constructor(clientId: string, name: string, registration: OAuthClientRegistration) {
        this.clientId = clientId;
        this.name = name;
        this.registration = registration;
    }

    /** Find a client by its identifier, refusing an unknown or withdrawn one. */
    static async find(clientId: string, database: DatabaseConnection): Promise<OAuthClient> {
        // read a device's client, accepting one its program has yet to register
        if (client.identifies(clientId)) {
            const row = await Snapshot.live(database)
                .read(client.table)
                .findFirst({ where: { id: client.identifier(clientId) } });
            if (row !== undefined && row.revokedAt !== null) {
                throw OAuthError.of("INVALID_CLIENT", `client ${clientId} was revoked`);
            }

            return new OAuthClient(clientId, row?.name ?? DEVICE_CLIENT_NAME, { kind: "device" });
        }

        // read an installation's origin, its own identifier, named by its installation, else its hostname for a digest label
        const origin = OAuthClient.#origin(clientId);
        if (origin !== undefined) {
            const name = origin.digest === undefined ? origin.name : new URL(clientId).hostname;

            return new OAuthClient(clientId, name, {
                kind: "installation",
                origin: clientId,
                installation: origin.digest === undefined ? origin.name : null,
                space: SpaceAddress.of({ space: origin.space, handle: origin.handle }),
            });
        }

        // read a registered application
        const [registration] = await database
            .select()
            .from(application.table)
            .where(eq(application.table.clientId, clientId))
            .limit(1);
        if (registration === undefined) {
            throw OAuthError.of("INVALID_CLIENT", `unknown client ${clientId}`);
        }

        return new OAuthClient(clientId, registration.name, {
            kind: "application",
            application: registration,
        });
    }

    /** The application's registration, absent for another client. */
    get application(): Application | undefined {
        return this.registration.kind === "application" ? this.registration.application : undefined;
    }

    /** Refuse scopes beyond the client's registered ones, every supported one otherwise. */
    requireScopes(scopes: readonly string[]): void {
        const allowed: readonly string[] =
            this.registration.kind === "installation"
                ? INSTALLATION_SCOPES
                : (this.application?.scopes ?? OAUTH_SCOPES);
        if (scopes.some((scope) => !allowed.includes(scope))) {
            throw OAuthError.of("INVALID_SCOPE", "the client may not ask for these scopes");
        }
    }

    /** Refuse new authorizations of a disabled application. */
    requireEnabled(): void {
        if (this.application?.isDisabled === true) {
            throw OAuthError.of("UNAUTHORIZED_CLIENT", "client is disabled");
        }
    }

    /** Refuse a redirect the client did not register, its loopback or its origin's open path. */
    requireRedirect(uri: string): void {
        // match a registered redirect exactly
        const { registration } = this;
        if (registration.kind === "application") {
            if (!registration.application.redirectUris.includes(uri)) {
                throw OAuthError.of("INVALID_REQUEST", "redirect_uri is not registered");
            }
        }
        // match an installation origin's open path exactly
        else if (registration.kind === "installation") {
            if (uri !== `${registration.origin}${OPEN_PATH}`) {
                throw OAuthError.of(
                    "INVALID_REQUEST",
                    "redirect_uri is not the origin's open path",
                );
            }
        }
        // match a device's loopback redirect on any port
        else {
            const url = URL.parse(uri);
            const isLoopback =
                url !== null &&
                url.protocol === "http:" &&
                LOOPBACK_HOSTS.has(url.hostname) &&
                url.pathname === LOOPBACK_PATH &&
                url.search === "" &&
                url.hash === "";
            if (!isLoopback) {
                throw OAuthError.of("INVALID_REQUEST", "redirect_uri is no loopback redirect");
            }
        }
    }

    /** Refuse a grant the client may not use. */
    requireGrant(grantType: string): void {
        const { registration } = this;
        const grants: ReadonlySet<string> =
            registration.kind === "application"
                ? new Set(registration.application.grantTypes)
                : registration.kind === "installation"
                  ? INSTALLATION_GRANTS
                  : DEVICE_GRANTS;
        if (!grants.has(grantType)) {
            throw OAuthError.of("UNAUTHORIZED_CLIENT", `the client may not use ${grantType}`);
        }
    }

    /** Read the installation origin a client identifier is: a space's https or a loopback one. */
    static #origin(clientId: string): InstallationOrigin | undefined {
        // require a bare origin, https below the spaces' domain or http on a loopback machine
        const url = URL.parse(clientId);
        if (url === null || url.origin !== clientId) {
            return undefined;
        } else if (url.protocol === "https:") {
            return InstallationOrigin.parse(url.hostname, DOMAINS.space);
        } else if (url.protocol === "http:") {
            return InstallationOrigin.parse(url.hostname, LOOPBACK_DOMAIN);
        }

        return undefined;
    }
}

/** The authentication of a client by its secret, or by its identifier alone. */
export const ClientAuthentication = {
    /** Verify an enabled client allowed a grant type, and read its request by a shape. */
    async grant<Shape extends schema.Schema>(
        request: { readonly body: Readonly<Record<string, string>>; readonly headers: Headers },
        grantType: string,
        shape: Shape,
        options: { readonly database: DatabaseConnection; readonly crypto: Crypto },
    ): Promise<{ readonly party: OAuthClient; readonly asked: schema.Infer<Shape> }> {
        // verify the client, enabled and allowed the grant type
        const { body, headers } = request;
        const party = await ClientAuthentication.verify(
            body,
            headers,
            options.database,
            options.crypto,
        );
        party.requireEnabled();
        party.requireGrant(grantType);

        // read the request by its shape
        const parsed = shape.safeParse(body);
        if (!parsed.success) {
            throw OAuthError.of("INVALID_REQUEST", `invalid ${grantType} request`);
        }

        return { party, asked: parsed.data };
    },

    /** Verify a request's client by its basic or posted secret, refusing a wrong or missing one. */
    async verify(
        body: Readonly<Record<string, string>>,
        headers: Headers,
        database: DatabaseConnection,
        crypto: Crypto,
    ): Promise<OAuthClient> {
        // read the client's identifier and secret from the basic header, else from the form
        const basic = headers.get("authorization");
        const isBasic = basic?.slice(0, BASIC.length).toLowerCase() === BASIC.toLowerCase();
        const [clientId, secret] =
            basic !== null && isBasic
                ? ClientAuthentication.basic(basic.slice(BASIC.length))
                : [body["client_id"], body["client_secret"]];
        if (clientId === undefined) {
            throw OAuthError.of("INVALID_CLIENT", "the request names no client");
        }

        // find the client and how it sent its secret
        const party = await OAuthClient.find(clientId, database);
        const method = party.application?.tokenEndpointAuthMethod ?? "none";
        const sent =
            basic?.startsWith(BASIC) === true ? "client_secret_basic" : "client_secret_post";

        // refuse a secret for a public client
        if (method === "none" && secret !== undefined) {
            throw OAuthError.of("INVALID_CLIENT", "a public client has no secret");
        }
        // refuse a secret sent another way than the client registered
        else if (method !== "none" && secret !== undefined && method !== sent) {
            throw OAuthError.of("INVALID_CLIENT", `the client authenticates with ${method}`);
        }
        // refuse a confidential client's missing or wrong secret
        else if (method !== "none") {
            const digest = secret === undefined ? undefined : await Digest.of(secret, crypto);
            if (digest === undefined || digest !== party.application?.clientSecret) {
                throw OAuthError.of("INVALID_CLIENT", "invalid client_secret");
            }
        }

        return party;
    },

    /** Read a basic header's client identifier and secret, each form-encoded (RFC 6749 2.3.1). */
    basic(credentials: string): [string, string] {
        // split the decoded pair at its first colon, refusing malformed base64 or escapes
        try {
            const decoded = new TextDecoder().decode(Uint8Array.fromBase64(credentials));
            const colon = decoded.indexOf(":");
            if (colon >= 0) {
                return [formDecode(decoded.slice(0, colon)), formDecode(decoded.slice(colon + 1))];
            }
        } catch (error) {
            throw OAuthError.of("INVALID_CLIENT", "malformed basic credentials", { cause: error });
        }
        throw OAuthError.of("INVALID_CLIENT", "malformed basic credentials");
    },
};

/** Decode a form-encoded value (WHATWG URL application/x-www-form-urlencoded). */
function formDecode(value: string): string {
    return decodeURIComponent(value.replaceAll("+", " "));
}
