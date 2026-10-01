import { principal } from "@destack/access";
import type { DatabaseConnection } from "@destack/db";
import { DirectoryStore } from "@destack/directory";
import { Scope, type Subject } from "@destack/sync";
import {
    Authentication,
    AUTHENTICATION_LIFETIME_MILLISECONDS,
    TokenIssuer,
    type TokenIssuerOptions,
} from "@destack/service/authentication";
import { ServiceError } from "@destack/service/error";
import { originalRequest } from "@destack/service/request";
import { implement, type ServiceContext } from "@destack/service/server";
import { hostToken } from "../../service/host.ts";
import { HostAssertion } from "./assertion.ts";

/** The host procedures' implementations. */
const procedures = implement(hostToken).$context<ServiceContext>();

/** Grant hosts access tokens for the keys they prove, signed by the universe's issuer. */
export function hostRouter(
    database: DatabaseConnection,
    tokens: Pick<TokenIssuerOptions, "issuer" | "sign">,
) {
    return procedures.router({
        grant: procedures.grant.handler(async ({ input, context }) => {
            // verify the host's assertion for this request, spending it
            const request = originalRequest(context.request);
            const now = Date.now();
            const key = await HostAssertion.verify(
                input.assertion,
                { method: request.method, url: request.url },
                database,
                now,
            );

            // require a space to sit in the host's cell or its region's
            const subject = principal.host.reference(key.accountId, key.hostId);
            const regions: Subject[] =
                key.regionId === null
                    ? []
                    : [principal.region.reference(Scope.universe.id, key.regionId)];
            if (input.spaceId !== undefined) {
                const zone = await new DirectoryStore(database).locate(input.spaceId);
                const cells: readonly (string | null)[] = [key.hostId, key.regionId];
                if (zone === undefined || !cells.includes(zone.cell)) {
                    throw new ServiceError("FORBIDDEN", {
                        message: `${input.spaceId} is served elsewhere`,
                    });
                }
            }

            // sign a token of the host and its region for the audience
            const issuer = new TokenIssuer({ ...tokens, authority: { kind: "universe" } });

            return issuer.issue(
                new Authentication({
                    credential: { kind: "host-key", id: key.id },
                    audience: input.audience,
                    ...(input.spaceId === undefined ? {} : { scope: input.spaceId }),
                    subject,
                    subjects: [subject, ...regions],
                    verifiedAt: now,
                    expiresAt: now + AUTHENTICATION_LIFETIME_MILLISECONDS,
                }),
                now,
            );
        }),
    });
}
