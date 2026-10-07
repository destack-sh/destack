import {
    account,
    type Allowance,
    allowance,
    type AllowanceResource,
} from "@destack/account/object";
import { eq } from "@destack/db";
import type { PackageId } from "@destack/package";
import { canonicalize, schema } from "@destack/schema";
import { Catalog } from "../catalog/catalog.ts";
import { type AccountCall, Entitlement, entitlement } from "../object/index.ts";
import { CatalogReference } from "../catalog/reference.ts";

/** The fields an allowance takes from the entitlements it is derived from. */
type Derived = Pick<Allowance, "resource" | "limit" | "used" | "state" | "values">;

/** Derive an account's allowances from its entitlements. */
export async function deriveAllowances(
    call: AccountCall,
    scope: string,
    current: (packageId: PackageId) => Promise<Catalog>,
): Promise<void> {
    // derive the wanted allowances, by resource
    const accountId = account.identifier(scope);
    const granted = await call.database
        .select()
        .from(entitlement.table)
        .where(eq(entitlement.table.scope, accountId));
    const wanted = await resolve(granted, Catalog.once(current));

    // update each kept allowance that changed, and remove those no longer granted
    const kept = await call.database
        .select()
        .from(allowance.table)
        .where(eq(allowance.table.scope, accountId));
    for (const row of kept) {
        const next = wanted.get(row.resource);
        if (next === undefined) {
            await call.invoke(allowance).delete({ id: row.id });
        } else if (!isSame(row, next)) {
            await call.invoke(allowance).update({ id: row.id, ...next });
        }
        wanted.delete(row.resource);
    }

    // create the new ones
    for (const next of wanted.values()) {
        await call.invoke(allowance).create(next);
    }
}

/** Resolve the allowance of each granted feature declaring one. */
async function resolve(
    granted: readonly Entitlement[],
    catalog: (packageId: PackageId) => Promise<Catalog>,
): Promise<Map<AllowanceResource, Derived>> {
    // name each granted feature once
    const features = new Map(
        granted.map((row) => {
            const reference = { packageId: row.packageId, name: row.feature };

            return [CatalogReference.key(reference), reference];
        }),
    );

    // resolve the allowance of each feature declaring one
    const wanted = new Map<AllowanceResource, Derived>();
    for (const reference of features.values()) {
        const resource = (await catalog(reference.packageId)).find(reference)?.definition.allowance;
        const resolved = resource === undefined ? null : Entitlement.resolve(granted, reference);
        if (resource !== undefined && resolved !== null) {
            wanted.set(resource, { resource, ...derive(resolved) });
        }
    }

    return wanted;
}

/** Derive an allowance's fields from a resolved grant. */
function derive(
    resolved: NonNullable<ReturnType<typeof Entitlement.resolve>>,
): Omit<Derived, "resource"> {
    // keep a metered grant's limit, usage and state
    if (resolved.kind === "metered") {
        return { limit: resolved.limit, used: resolved.usage, state: resolved.state, values: null };
    }
    // count a numeric static grant, unlimited when null, or list its values
    else if (resolved.kind === "static") {
        return resolved.value === null || typeof resolved.value === "number"
            ? { limit: resolved.value, used: null, state: null, values: null }
            : {
                  limit: null,
                  used: null,
                  state: null,
                  values: schema.array(schema.string()).parse(resolved.value),
              };
    }
    // allow access alone
    else {
        return { limit: null, used: null, state: null, values: null };
    }
}

/** Report whether a kept allowance already has the derived fields. */
function isSame(row: Derived, next: Derived): boolean {
    return (
        row.limit === next.limit &&
        row.used === next.used &&
        row.state === next.state &&
        canonicalize(row.values) === canonicalize(next.values)
    );
}
