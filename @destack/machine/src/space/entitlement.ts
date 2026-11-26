import { account, Account, type Space, type SpaceResource } from "@destack/account/object";
import type { DatabaseConnection } from "@destack/db/database";
import type { SettingRetention } from "@destack/event";
import { Entitlement, entitlementShape } from "@destack/finance/object";
import type { Decision } from "@destack/object";
import { type Identifier, present, schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import { MachineError } from "../error/index.ts";

/** A retention setting's definition, read for the days it keeps events by default. */
const RetentionSetting = schema.looseObject({ default: schema.number().int().positive() });

/** Decide a served space's resource conditions by its account's plan. */
export async function enforceSpace(
    row: Space,
    database: DatabaseConnection,
    now: number,
): Promise<Readonly<Record<SpaceResource, Decision>>> {
    // decide each resource's condition by the account's plan
    const entitlement = planOf(database, account.identifier(row.scope));
    const decide = (resource: SpaceResource) =>
        entitlement.decide({ kind: "resource", resource }, now);
    const conditions = {
        database: await decide("database"),
        bucket: await decide("bucket"),
        compute: await decide("compute"),
        network: await decide("network"),
    };

    // refuse writes within the space while its database is exceeded
    const [link] = await Scope.chain(database.snapshot(), row.id);
    const isRefused = conditions.database.status === "false";
    if (present(link, `the scope of space ${row.id}`).isCapped !== isRefused) {
        await database.commit((transaction) =>
            Scope.cap(transaction, row.id, isRefused ? now : null),
        );
    }

    return conditions;
}

/** Cap a space's event retention at each setting's default unless its account's plan keeps more. */
export function capRetention(
    database: DatabaseConnection,
    retention: SettingRetention,
    clock: { readonly now: () => number },
): SettingRetention {
    return async (setting, owner, scope) => {
        // read the days the space chose and whether its account's plan allows custom retention
        const days = await retention(setting, owner, scope);
        const { id } = await Account.of(database.snapshot(), scope);
        const allowed = await planOf(database, id).decide(
            { kind: "resource", resource: "retention" },
            clock.now(),
        );
        if (allowed.status === "unknown") {
            throw MachineError.of(
                "ENTITLEMENTS_UNAVAILABLE",
                `the plan of account '${id}' is unavailable: it has not replicated yet`,
            );
        }

        // keep the chosen days with custom retention, else cap them at the setting's default
        return allowed.status === "true"
            ? days
            : Math.min(days, RetentionSetting.parse(setting).default);
    };
}

/** Read an account's plan from a space database replicating it from the universe. */
export function planOf(
    database: DatabaseConnection,
    accountId: Identifier<"account">,
): Entitlement {
    const key = { scope: Scope.universe.id, account: accountId };

    return new Entitlement(database, accountId, { shape: entitlementShape, key });
}
