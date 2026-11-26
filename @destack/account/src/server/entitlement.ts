import type { Decision } from "@destack/object";
import type { Tenancy } from "../object/space.ts";

/** A use of an account's plan the universe decides: a space's tenancy, or custom domains. */
export type EntitlementUse =
    | { readonly kind: "tenancy"; readonly tenancy: Tenancy }
    | { readonly kind: "resource"; readonly resource: "domain" };

/** What an account's plan allows, as finance decides it. */
export interface Entitlement {
    /** Decide a use of the plan now. */
    readonly decide: (use: EntitlementUse) => Promise<Decision>;
}
