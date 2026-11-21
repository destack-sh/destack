import { BasicCredential, type Environment } from "@destack/environment";
import { OriginPath } from "@destack/package";
import { type DatabaseConnection, eq, Snapshot } from "@destack/db";
import { Digest, schema } from "@destack/schema";
import { OAuthError } from "../authentication/token.ts";
import { client, DEVICE_CLIENT_NAME } from "../object/client.ts";
import { OAUTH_SCOPES, type Application, application } from "../object/application.ts";
import {
    InstallationOrigin,
    LOOPBACK_DOMAIN,
    SpaceAddress,
    type UniverseOrigin,
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
    static async find(
        clientId: string,
        database: DatabaseConnection,
        universe: UniverseOrigin,
    ): Promise<OAuthClient> {
        // read a device's client, accepting one its program has yet to register
        if (client.identifies(clientId)) {
            const row = await Snapshot.live(database)
                .read(client.table)
                .findFirst({ where: { id: client.identifier(clientId) } });
            if (row !== undefined && row.revokedAt !== null) {
                throw OAuthError.of(
                    "CLIENT_UNAUTHENTICATED",
                    `authentication required: client '${clientId}' was revoked`,
                );
            }

            return new OAuthClient(clientId, row?.name ?? DEVICE_CLIENT_NAME, { kind: "device" });
        }

        // read an installation's origin, its own identifier, named by its installation, else its hostname for a digest label
        const origin = OAuthClient.#origin(clientId, universe);
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
            throw OAuthError.of(
                "CLIENT_UNAUTHENTICATED",
                `authentication required: client '${clientId}' is unknown`,
            );
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
            throw OAuthError.of(
                "SCOPE_INVALID",
                "scope is invalid: the client may not ask for these scopes",
            );
        }
    }

    /** Refuse new authorizations of a disabled application. */
    requireEnabled(): void {
        if (this.application?.isDisabled === true) {
            throw OAuthError.of("CLIENT_INVALID", "client is invalid: it is disabled");
        }
    }

    /** Refuse a redirect the client did not register, its loopback or its origin's open path. */
    requireRedirect(uri: string): void {
        // match a registered redirect exactly
        const { registration } = this;
        if (registration.kind === "application") {
            if (!registration.application.redirectUris.includes(uri)) {
                throw OAuthError.of(
                    "REQUEST_INVALID",
                    "redirect_uri is invalid: it is not registered",
                );
            }
        }
        // match an installation origin's open path exactly
        else if (registration.kind === "installation") {
            if (uri !== `${registration.origin}${OriginPath.open}`) {
                throw OAuthError.of(
                    "REQUEST_INVALID",
                    "redirect_uri is invalid: it is not the origin's open path",
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
                throw OAuthError.of(
                    "REQUEST_INVALID",
                    "redirect_uri is invalid: it is no loopback redirect",
                );
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
            throw OAuthError.of(
                "CLIENT_INVALID",
                `client is invalid: it may not use '${grantType}'`,
            );
        }
    }

    /** Read the installation origin a client identifier is: a space's or a loopback one. */
    static #origin(clientId: string, universe: UniverseOrigin): InstallationOrigin | undefined {
        // require a bare origin, a name below the spaces' domain or http on a loopback machine
        const url = URL.parse(clientId);
        if (url === null || url.origin !== clientId) {
            return undefined;
        }

        // read a name below the spaces' domain, else one on a loopback machine
        const named =
            universe.url(url.hostname) === clientId
                ? InstallationOrigin.parse(url.hostname, universe.domains.space)
                : undefined;
        const loopback =
            url.protocol === "http:"
                ? InstallationOrigin.parse(url.hostname, LOOPBACK_DOMAIN)
                : undefined;

        return named ?? loopback;
    }
}

/** What a client's authentication reads its registration and digests its secret with. */
export interface ClientAuthenticationOptions {
    /** The account database with the clients' registrations. */
    readonly database: DatabaseConnection;
    /** The universe, below whose domains installations' clients answer. */
    readonly universe: UniverseOrigin;
    /** The WebCrypto digesting a client's secret. */
    readonly environment: Pick<Environment, "crypto">;
}

/** The authentication of a client by its secret, or by its identifier alone. */
export const ClientAuthentication = {
    /** Verify an enabled client allowed a grant type, and read its request by a shape. */
    async grant<Shape extends schema.Schema>(
        request: { readonly body: Readonly<Record<string, string>>; readonly headers: Headers },
        grantType: string,
        shape: Shape,
        options: ClientAuthenticationOptions,
    ): Promise<{ readonly party: OAuthClient; readonly asked: schema.Infer<Shape> }> {
        // verify the client, enabled and allowed the grant type
        const { body, headers } = request;
        const party = await ClientAuthentication.verify(body, headers, options);
        party.requireEnabled();
        party.requireGrant(grantType);

        // read the request by its shape
        const parsed = shape.safeParse(body);
        if (!parsed.success) {
            throw OAuthError.of(
                "REQUEST_INVALID",
                `'${grantType}' request is invalid: it does not match its schema`,
            );
        }

        return { party, asked: parsed.data };
    },

    /** Verify a request's client by its basic or posted secret, refusing a wrong or missing one. */
    async verify(
        body: Readonly<Record<string, string>>,
        headers: Headers,
        options: ClientAuthenticationOptions,
    ): Promise<OAuthClient> {
        // read the client's identifier and secret from the basic header, else from the form
        const basic = headers.get("authorization");
        const isBasic = BasicCredential.is(basic);
        const [clientId, secret] = isBasic
            ? ClientAuthentication.basic(basic)
            : [body["client_id"], body["client_secret"]];
        if (clientId === undefined) {
            throw OAuthError.of(
                "CLIENT_UNAUTHENTICATED",
                "authentication required: the request names no client",
            );
        }

        // find the client and how it sent its secret
        const { database, universe } = options;
        const { crypto } = options.environment;
        const party = await OAuthClient.find(clientId, database, universe);
        const method = party.application?.tokenEndpointAuthMethod ?? "none";
        const sent = isBasic ? "client_secret_basic" : "client_secret_post";

        // refuse a secret for a public client
        if (method === "none" && secret !== undefined) {
            throw OAuthError.of(
                "CLIENT_UNAUTHENTICATED",
                "authentication required: a public client has no secret",
            );
        }
        // refuse a secret sent another way than the client registered
        else if (method !== "none" && secret !== undefined && method !== sent) {
            throw OAuthError.of(
                "CLIENT_UNAUTHENTICATED",
                `authentication required: the client authenticates with '${method}'`,
            );
        }
        // refuse a confidential client's missing or wrong secret
        else if (method !== "none") {
            const digest = secret === undefined ? undefined : await Digest.of(secret, crypto);
            if (digest === undefined || digest !== party.application?.clientSecret) {
                throw OAuthError.of(
                    "CLIENT_UNAUTHENTICATED",
                    "authentication required: the client_secret is invalid",
                );
            }
        }

        return party;
    },

    /** Read a basic header's client identifier and secret, each form-encoded (RFC 6749 2.3.1). */
    basic(authorization: string | null): [string, string] {
        // read the pair and decode each half, refusing malformed base64 or escapes
        const read = BasicCredential.read(authorization);
        try {
            if (read !== undefined) {
                return [formDecode(read.username), formDecode(read.password)];
            }
        } catch (error) {
            throw OAuthError.of(
                "CLIENT_UNAUTHENTICATED",
                "authentication required: the basic credentials are malformed",
                { cause: error },
            );
        }
        throw OAuthError.of(
            "CLIENT_UNAUTHENTICATED",
            "authentication required: the basic credentials are malformed",
        );
    },
};

/** Decode a form-encoded value (WHATWG URL application/x-www-form-urlencoded). */
function formDecode(value: string): string {
    return decodeURIComponent(value.replaceAll("+", " "));
}
