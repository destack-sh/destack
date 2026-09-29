import { principal } from "@destack/access";
import { and, eq, unique, type DatabaseConnection, type Select } from "@destack/db";
import { defineObject, field, method } from "@destack/object";
import { schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import { account, type Account } from "./account.ts";
import { revokeOnce } from "./revocation.ts";

/** Software an account administers, with service tokens. */
export const serviceAccount = defineObject({
    name: "service-account",
    tier: "global",
    plural: "serviceAccounts",
    scope: account,
    represents: principal.serviceAccount,
    fields: {
        /** The account-local name. */
        name: field.string(schema.string().min(1).max(200)),
        /** The time the account withdrew it. */
        revokedAt: field.time().optional(),
    },
    permissions: ["create", "read", "update", "issue", "revoke"],
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("create", { fields: ["name"] }),
        update: method.update("update", { fields: ["name"] }),
        revoke: method({ permission: "revoke" }).handle((call) =>
            revokeOnce(call, "service account"),
        ),
    },
    constraints: (identity) => [
        unique("service_scope_name").on(identity.scope, identity.name),
        unique("service_scope_scope_id").on(identity.scope, identity.id),
    ],
});
/** A persisted service account. */
export type ServiceAccount = Select<typeof serviceAccount.table>;

/** A service account's own revocation and the standing of its account. */
export class ServiceAccountStanding {
    /** The time the account withdrew the service account. */
    readonly revokedAt: number | null;
    /** Whether its account is neither suspended nor being deleted. */
    readonly isAccountActive: boolean;

    /** Hold a read standing. */
    private constructor(revokedAt: number | null, isAccountActive: boolean) {
        this.revokedAt = revokedAt;
        this.isAccountActive = isAccountActive;
    }

    /** Read the standing of a service account of an account, absent for an unknown one. */
    static async read(
        accountId: Account["id"],
        serviceAccountId: ServiceAccount["id"],
        database: DatabaseConnection,
    ): Promise<ServiceAccountStanding | undefined> {
        // read the service account with its account's suspension and deletion
        const [row] = await database
            .select({
                revokedAt: serviceAccount.table.revokedAt,
                suspendedAt: Scope.table.suspendedAt,
                deletionRequestedAt: account.table.deletionRequestedAt,
            })
            .from(serviceAccount.table)
            .innerJoin(account.table, eq(account.table.id, serviceAccount.table.scope))
            .innerJoin(Scope.table, eq(Scope.table.scope, serviceAccount.table.scope))
            .where(
                and(
                    eq(serviceAccount.table.id, serviceAccountId),
                    eq(serviceAccount.table.scope, accountId),
                ),
            )
            .limit(1);
        if (row === undefined) {
            return undefined;
        }

        return new ServiceAccountStanding(
            row.revokedAt,
            row.suspendedAt === null && row.deletionRequestedAt === null,
        );
    }

    /** Whether the service account authenticates: unrevoked in an active account. */
    get isActive(): boolean {
        return this.revokedAt === null && this.isAccountActive;
    }
}
