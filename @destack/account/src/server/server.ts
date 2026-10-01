import { principal } from "@destack/access";
import { Snapshot } from "@destack/db/log";
import { identifier, schema } from "@destack/schema";
import type { AuditDestination } from "@destack/audit";
import { ObjectServer } from "@destack/object/server";
import { type CallKey } from "@destack/service/request";
import type { ServiceImplementation } from "@destack/service/server";
import type { ObjectType } from "@destack/object";
import { Scope } from "@destack/sync";
import { account } from "./account/index.ts";
import { user } from "./user/index.ts";
import { membership } from "./membership/index.ts";
import { oauthClient, oauthConsent } from "./oauth/index.ts";
import { device, deviceKey } from "./device/index.ts";
import { personalAccessToken, serviceToken } from "./token/index.ts";
import { directoryRouter } from "./directory/index.ts";
import type { Connections } from "./connection/index.ts";
import { authenticationRouter } from "./authentication/verification.ts";
import type { Authenticator } from "../authentication/authentication.ts";
import { accountPackage } from "../audit/index.ts";
import { accountObjects, accountService } from "../service/service.ts";
import type { DnsResolver } from "../dns/index.ts";
import { DomainVerifier } from "./domain/index.ts";
import type { TokenIssuerOptions } from "@destack/service/authentication";
import { hostRouter } from "./host/index.ts";
import { host, hostKey, region } from "../object/index.ts";

/** The account a protected procedure on its contents selects. */
const AccountSelection = schema.object({ accountId: identifier("account") }).passthrough();

/** Serve the account service. */
export function implementService(
    authenticator: Authenticator,
    options: {
        /** The connection authorization flow over the configured providers and vaults. */
        readonly connections: Connections;
        /** The audit history the journal delivers calls to. */
        readonly history: AuditDestination;
        /** The resolver reading domains' challenge records. */
        readonly resolver: DnsResolver;
        /** The universe's token issuer and its signing key, granting hosts access tokens. */
        readonly tokens: Pick<TokenIssuerOptions, "issuer" | "sign">;
        /** Read the deployment's key that sensitive inputs are fingerprinted under. */
        readonly callKey: CallKey;
        /** The other object types with inherited rows the database holds and relays. */
        readonly inherited: readonly ObjectType[];
    },
): ServiceImplementation {
    // decide every call through the account policies
    const database = authenticator.database;

    // serve the objects, with the server behaviour of those needing it
    const objects = new ObjectServer({
        objects: {
            ...accountObjects,
            account,
            user,
            personalAccessToken,
            serviceToken,
            device,
            deviceKey,
            oauthClient,
            oauthConsent,
            connection: options.connections.handle(),
            membership,
            domain: new DomainVerifier(options.resolver).handle(),
            host,
            hostKey,
        },
        policies: [...options.inherited, region],
        database,
        callKey: options.callKey,
        origin: { package: accountPackage, service: accountService.name },
        history: options.history,
        context: (context, scope) => {
            // let a host act for the cells it is: itself and the regions it serves
            const access = context.access(scope);
            const cells = access.subjects
                .filter((subject) => principal.host.is(subject) || principal.region.is(subject))
                .map((subject) => principal.cell.reference(Scope.universe.id, subject.id));

            return cells.length === 0
                ? access
                : { ...access, subjects: [...access.subjects, ...cells] };
        },
    });

    return {
        service: accountService,
        access: {
            authorizer: objects.authorizer,
            database,
            target: async ({ input }) => {
                // decide on the named account
                const { accountId } = AccountSelection.parse(input);

                return Scope.object(Snapshot.live(database), accountId);
            },
        },
        controllers: objects.controllers(),
        router: {
            // serve the object methods, and their shared replica procedures
            ...objects.router(),

            // serve authentication, the key index and the directory
            authentication: authenticationRouter(authenticator, objects.authorizer),
            directory: directoryRouter(database, objects.authorizer),
            hostToken: hostRouter(database, options.tokens),
        },
        audit: objects.implement(accountService).audit,
        route: async (request) =>
            authenticator.handles(new URL(request.url).pathname)
                ? authenticator.handler(request)
                : undefined,
        responseHeaders: { "Cache-Control": "no-store" },
    };
}
