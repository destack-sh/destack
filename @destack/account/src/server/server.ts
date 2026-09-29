import type { Table } from "@destack/db";
import { Snapshot } from "@destack/db/log";
import { identifier, schema } from "@destack/schema";
import { AuditRecorder } from "@destack/audit";
import { type AuditDestination, AuditOutbox } from "@destack/audit/outbox";
import { ObjectServer } from "@destack/object/server";
import { DirectoryDatabase } from "@destack/directory";
import { Journal } from "@destack/service/database";
import type { ServiceContext, ServiceImplementation } from "@destack/service/server";
import { accessTables, Authorizer } from "@destack/access";
import type { ObjectType } from "@destack/object";
import { Feed, Scope } from "@destack/sync";
import { replicaRouter } from "./access/index.ts";
import { account } from "./account/index.ts";
import { user } from "./user/index.ts";
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
        },
        database,
        audit,
        journal: new Journal(accountJournal),
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

            // serve authentication, access replication, the key index and the directory
            authentication: authenticationRouter(authentication, objects.authorizer),
            access: replicaRouter(
                new Feed(database, [
                    ...accessTables,
                    ...options.inherited.map((object) => object.table as Table),
                ]),
                new DirectoryDatabase(database),
                database,
                new Authorizer(
                    options.inherited.map((object) => object.policy),
                    options.inherited.map((object) => object.mapping),
                ),
            ),
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
