import { type Authentication, Caller, principal } from "@destack/access";
import { and, eq, isNull, type DatabaseConnection } from "@destack/db";
import type { MethodContextOf, ResultOf } from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import {
    Assertion,
    type AssertionKey,
    AssertionVerifier,
    Bearer,
    INSTALLATION_KEY,
    TOKEN_LIFETIME_MILLISECONDS,
    type TokenIssuer,
} from "@destack/access/token";
import { installation } from "@destack/object";
import { rollout } from "../../object/rollout.ts";
import { installationKey } from "../../object/key.ts";
import type {} from "@destack/package/import-meta";
import { StackError } from "../../error/index.ts";

/** The keys installations sign their assertions with, and the tokens they are granted for them. */
export const InstallationAssertion = {
    /** Verify the assertions a store's installations sign for a package, spending each once. */
    verifier(
        objects: Pick<ObjectStore, "database" | "environment">,
        audience: PackageId,
    ): AssertionVerifier {
        return new AssertionVerifier({
            endpoint: PackageId.resource(audience),
            audience,
            keys: (id, _issuer, now) => InstallationAssertion.key(objects.database, id, now),
            crypto: objects.environment.crypto,
            database: objects.database,
        });
    },

    /** Verify a request's installation assertion, answering undefined for a request without one. */
    async verify(
        verifier: AssertionVerifier,
        request: Request,
        now: number,
    ): Promise<Authentication | undefined> {
        const bearer = Bearer.read(request.headers);

        return bearer !== undefined && Assertion.is(bearer)
            ? verifier.verify(bearer, now)
            : undefined;
    },

    /** Read the standing key of an enabled installation an assertion refers to. */
    async key(
        database: DatabaseConnection,
        id: string,
        now: number,
    ): Promise<AssertionKey | undefined> {
        // find the key of an enabled installation whose rollout, if any, is not being deleted
        const keyId = schema.identifier("installation-key").safeParse(id);
        const [found] = !keyId.success
            ? []
            : await database
                  .select({ key: installationKey.table })
                  .from(installationKey.table)
                  .innerJoin(
                      installation.table,
                      eq(installation.table.id, installationKey.table.parentId),
                  )
                  .leftJoin(rollout.table, eq(rollout.table.id, installation.table.id))
                  .where(
                      and(
                          eq(installationKey.table.id, keyId.data),
                          eq(installation.table.status, "enabled"),
                          isNull(rollout.table.deletedAt),
                      ),
                  );

        // refuse a revoked or expired key
        const key = found?.key;
        if (
            key === undefined ||
            key.revokedAt !== null ||
            (key.expiresAt !== null && key.expiresAt <= now)
        ) {
            return undefined;
        }

        return {
            id: key.id,
            subject: principal.installation.reference(key.scope, key.parentId),
            publicKey: key.publicKey,
        };
    },

    /** Grant the calling installation a token of its space for its own key's assertion. */
    async token(
        context: MethodContextOf<typeof installation, "token">,
        issuer: TokenIssuer,
    ): Promise<ResultOf<typeof installation, "token">> {
        // read the key the installation's assertion proved
        const { now } = context;
        const target = context.target;
        const { credential } = context.caller;
        const keyId = schema.identifier("installation-key").safeParse(credential?.id);
        const key = keyId.success
            ? await context.read(installationKey).findFirst({ where: { id: keyId.data } })
            : undefined;
        if (credential?.kind !== INSTALLATION_KEY || key?.parentId !== target.id) {
            throw StackError.of(
                "FOREIGN_ASSERTION",
                "an installation token requires the installation's own assertion",
            );
        }

        // record the key's use as the installation it proved
        await context.mutate(context.caller, [
            installationKey.call("use", installationKey.target(key)),
        ]);

        // sign a token of only the installation for the audience as its space
        const subject = principal.installation.reference(context.scope, target.id);

        return issuer.issue(
            {
                caller: Caller.of(subject),
                credential: {
                    kind: INSTALLATION_KEY,
                    id: key.id,
                    audience: context.input.audience,
                    scope: context.scope,
                    verifiedAt: now,
                    expiresAt: now + TOKEN_LIFETIME_MILLISECONDS,
                },
            },
            now,
            context.environment,
        );
    },
};
