import { principal } from "@destack/access";
import { Snapshot } from "@destack/db/log";
import { identifier, schema } from "@destack/schema";
import { AuditRecorder } from "@destack/audit";
import { type AuditDestination, AuditOutbox } from "@destack/audit/outbox";
import { ObjectServer } from "@destack/object/server";
import { Journal, type JournalKey } from "@destack/service/database";
import type { ServiceContext, ServiceImplementation } from "@destack/service/server";
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
import type { Authentication } from "../authentication/authentication.ts";
import type { Caller } from "@destack/service/authentication";
import { accountAudit, accountPackage } from "../audit/index.ts";
import { accountObjects, accountService } from "../service/service.ts";
import { accountJournal } from "../stack/journal.ts";

/** The account a protected procedure on its contents selects. */
const AccountSelection = schema.object({ accountId: identifier("account") }).passthrough();

/** Serve the account service. */
export function implementService(
    authentication: Authentication,
    options: {
        /** The connection authorization flow over the configured providers and vaults. */
        readonly connections: Connections;
        /** The audit history the outbox delivers to. */
        readonly history: AuditDestination;
        /** Read the deployment's key that sensitive inputs are fingerprinted under. */
        readonly journalKey: JournalKey;
        /** The other object types with inherited rows the database holds and relays. */
        readonly inherited: readonly ObjectType[];
    },
): ServiceImplementation {
    // decide every call through the account policies
    const database = authentication.database;
    const outbox = new AuditOutbox(database);
    const audit = (scope: string, context?: ServiceContext) =>
        context === undefined
            ? AuditRecorder.system(outbox, {
                  package: accountPackage,
                  service: "account",
                  scope,
              })
            : accountAudit(readCaller(context), database, context.requestId, scope);

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
        },
        policies: options.inherited,
        database,
        audit,
        journal: new Journal(accountJournal, options.journalKey),
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
        controllers: [...objects.controllers(), outbox.controller(options.history)],
        router: {
            // serve the object methods, and their shared replica procedures
            ...objects.router(),

            // serve authentication, the key index and the directory
            authentication: authenticationRouter(authentication, objects.authorizer),
            directory: directoryRouter(database, objects.authorizer),
        },
        audit: AuditRecorder.procedure(({ context }) => audit(Scope.universe.id, context)),
        route: async (request) =>
            authentication.handles(new URL(request.url).pathname)
                ? authentication.handler(request)
                : undefined,
        responseHeaders: { "Cache-Control": "no-store" },
    };
}

/** Read the verified caller of a request, a user's or a host's, absent when verification failed. */
function readCaller(context: ServiceContext): Caller | null {
    return context.authenticationError === undefined ? context.caller : null;
}
