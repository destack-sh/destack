import { principal, type Subject } from "@destack/access";
import { account, region } from "@destack/account/object";
import { AuditRecorder } from "@destack/audit";
import { AuditOutbox } from "@destack/audit/outbox";
import type { DatabaseConnection } from "@destack/db";
import { DirectoryDatabase } from "@destack/directory";
import { ObjectServer } from "@destack/object/server";
import { Scope } from "@destack/sync";
import {
    Caller,
    CALLER_LIFETIME_MILLISECONDS,
    TokenIssuer,
    type TokenIssuerOptions,
} from "@destack/service/authentication";
import { Journal } from "@destack/service/database";
import { ServiceError } from "@destack/service/error";
import { originalRequest } from "@destack/service/request";
import {
    implement,
    type ServiceContext,
    type ServiceImplementation,
} from "@destack/service/server";
import { HostAssertion } from "../identity/assertion.ts";
import { host, hostKey } from "../object/index.ts";
import { hostService } from "../service/index.ts";
import { hostJournal } from "../stack/index.ts";

/** What the global tier serves hosts with. */
export interface HostServiceOptions {
    /** The global database holding hosts, their keys, the directory and the audit outbox its composer delivers. */
    readonly database: DatabaseConnection;
    /** The universe's token issuer and its signing key. */
    readonly tokens: Pick<TokenIssuerOptions, "issuer" | "sign">;
}

/** The token procedures of the host service. */
const tokens = implement(hostService.router.token).$context<ServiceContext>();

/** Serve hosts and their keys as objects, decided by their accounts' roles, and grant hosts access tokens. */
export function implementService(options: HostServiceOptions): ServiceImplementation {
    // serve the objects over the global database, recording audit events in its outbox
    const audit = AuditRecorder.service(new AuditOutbox(options.database), {
        package: host.package,
        service: hostService.name,
    });
    const objects = new ObjectServer({
        objects: { host, hostKey },
        policies: [account.policy, region],
        database: options.database,
        audit,
        journal: new Journal(hostJournal),
    });

    return {
        ...objects.implement(hostService),
        router: {
            ...objects.router(),
            token: tokens.router({
                grant: tokens.grant.handler(async ({ input, context }) => {
                    // verify the host's assertion for this request, spending it
                    const request = originalRequest(context.request);
                    const now = Date.now();
                    const key = await HostAssertion.verify(
                        input.assertion,
                        { method: request.method, url: request.url },
                        options.database,
                        now,
                    );

                    // require a space to sit in the host's cell or its region's
                    const subject = principal.host.reference(key.accountId, key.hostId);
                    const regions: Subject[] =
                        key.regionId === null
                            ? []
                            : [principal.region.reference(Scope.universe.id, key.regionId)];
                    if (input.spaceId !== undefined) {
                        const zone = await new DirectoryDatabase(options.database).locate(
                            input.spaceId,
                        );
                        const cells: readonly (string | null)[] = [key.hostId, key.regionId];
                        if (zone === undefined || !cells.includes(zone.cell)) {
                            throw new ServiceError("FORBIDDEN", {
                                message: `${input.spaceId} is served elsewhere`,
                            });
                        }
                    }

                    // sign a token of the host and its region for the audience
                    const issuer = new TokenIssuer({
                        ...options.tokens,
                        authority: { kind: "universe" },
                    });

                    return issuer.issue(
                        new Caller({
                            credential: { kind: "host-key", id: key.id },
                            audience: input.audience,
                            ...(input.spaceId === undefined ? {} : { scope: input.spaceId }),
                            subject,
                            subjects: [subject, ...regions],
                            verifiedAt: now,
                            expiresAt: now + CALLER_LIFETIME_MILLISECONDS,
                        }),
                        now,
                    );
                }),
            }),
        },
    };
}
