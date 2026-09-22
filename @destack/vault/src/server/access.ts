import { permitsCredential, permitsDelegation, sameSubject, type Subject } from "@destack/access";
import { and, eq, isNull, or, sql, type SQL, type DatabaseConnection } from "@destack/db";
import {
    roleBinding,
    roleQuery,
    space,
    serviceAccount,
    deployment,
    installation,
    deploymentSecretBinding,
    secret,
} from "@destack/model/regional";
import { ServiceError } from "@destack/service/error";
import { vaultPackage } from "../audit/index.ts";
import type { VaultAccess, VaultContext } from "./context.ts";

/** Require current regional grants, delegation restrictions and workload secret bindings. */
export async function authorizeVault(
    context: VaultContext,
    request: VaultAccess,
    transaction: DatabaseConnection,
): Promise<void> {
    // apply the shared delegation protocol to the exact persisted target
    const access = context.caller.context(vaultPackage.id, Date.now(), request.spaceId);
    const now = access.now;
    const [type, name] = request.operation.split(".");
    const permission = { packageId: vaultPackage.id, type, name };
    const objectId =
        request.operation === "secret.purge"
            ? request.spaceId
            : (request.secretId ?? request.vaultId ?? request.spaceId);
    if (
        !permitsCredential(permission, { scope: request.spaceId, id: objectId }, access) ||
        !permitsDelegation(permission, { scope: request.spaceId, id: objectId }, access)
    ) {
        throw new ServiceError("FORBIDDEN");
    }

    // require represented authority and every delegated actor to retain their own grant
    const identities = [
        access.subjects,
        ...(access.delegations ?? []).map((entry) => [entry.actor]),
    ];
    for (const subjects of identities) {
        // select one current matching grant without loading other members' grants
        const selected = subjects.map((subject) =>
            bindingPredicate(subject, request.spaceId, context),
        );
        const allowed = roleQuery.where(transaction, permission, {
            scope: eq(roleBinding.spaceId, request.spaceId),
            subject: selected.length ? or(...selected)! : sql`false`,
            object: objectId,
            now,
        });
        const grant = await transaction
            .select({ id: space.id })
            .from(space)
            .where(
                and(
                    eq(space.id, request.spaceId),
                    request.operation === "version.read" ? eq(space.state, "enabled") : undefined,
                    allowed,
                ),
            )
            .get();
        if (!grant) {
            throw new ServiceError("FORBIDDEN");
        }
    }

    // constrain each regional software identity to its authenticated running deployment
    const subjects = [
        ...access.subjects,
        ...(access.delegations ?? []).map((entry) => entry.actor),
    ];
    for (const subject of subjects) {
        if (subject.kind === "service-account" && subject.authority === request.spaceId) {
            await authorizeDeployment(request, subject, transaction, context);
        }
    }
}

/** Select bindings for a verified subject within its administering authority. */
function bindingPredicate(subject: Subject, spaceId: string, context: VaultContext): SQL {
    // resolve users through verified global membership records
    if (subject.kind === "user" && subject.authority === "global") {
        const memberships = (context.caller.authentication.memberships ?? []).filter((membership) =>
            sameSubject(membership.subject, subject),
        );

        return memberships.length
            ? or(
                  ...memberships.map((membership) =>
                      and(
                          sql`${roleBinding.accountId} = ${membership.accountId}`,
                          sql`${roleBinding.accountMembershipId} = ${membership.id}`,
                      ),
                  ),
              )!
            : sql`false`;
    }
    // qualify groups and global software identities by their administering account
    else if (subject.kind === "group") {
        return sql`${roleBinding.accountId} = ${subject.authority} AND ${roleBinding.groupId} = ${subject.id}`;
    } else if (subject.kind === "service-account" && subject.authority === spaceId) {
        return sql`${roleBinding.serviceAccountId} = ${subject.id}`;
    } else if (subject.kind === "service-account") {
        return sql`${roleBinding.accountId} = ${subject.authority} AND ${roleBinding.accountServiceAccountId} = ${subject.id}`;
    } else {
        return sql`false`;
    }
}

/** Require live workload authority and its declared secret selection. */
async function authorizeDeployment(
    request: VaultAccess,
    subject: Subject,
    transaction: DatabaseConnection,
    context: VaultContext,
): Promise<void> {
    // require a verified deployment identity even when its role allows the operation
    const deploymentId = context.caller.authentication.deployments?.find((entry) =>
        sameSubject(entry.subject, subject),
    )?.id;
    if (!deploymentId) {
        throw new ServiceError("FORBIDDEN");
    }

    // require the authenticated identity to belong to a currently running deployment
    const running = await transaction
        .select({ identity: serviceAccount.id })
        .from(deployment)
        .innerJoin(serviceAccount, eq(serviceAccount.id, deployment.serviceAccountId))
        .innerJoin(installation, eq(installation.id, deployment.installationId))
        .where(
            and(
                sql`${deployment.id} = ${deploymentId}`,
                eq(deployment.spaceId, request.spaceId),
                isNull(serviceAccount.revokedAt),
                eq(installation.state, "enabled"),
                or(eq(deployment.state, "active"), eq(deployment.state, "draining")),
            ),
        )
        .get();
    if (running?.identity !== subject.id) {
        throw new ServiceError("FORBIDDEN");
    }

    // plaintext reads require a deployment binding in addition to explicit role permissions
    if (request.operation === "version.read" && request.secretId) {
        const selected = await transaction
            .select({ version: deploymentSecretBinding.version, current: secret.currentVersion })
            .from(deploymentSecretBinding)
            .innerJoin(secret, eq(secret.id, deploymentSecretBinding.secretId))
            .where(
                and(
                    sql`${deploymentSecretBinding.deploymentId} = ${deploymentId}`,
                    eq(deploymentSecretBinding.spaceId, request.spaceId),
                    eq(deploymentSecretBinding.secretId, request.secretId),
                ),
            );
        const allowed = selected.some(
            (binding) => (binding.version ?? binding.current) === request.version,
        );
        if (!allowed) {
            throw new ServiceError("FORBIDDEN");
        }
    }
}
