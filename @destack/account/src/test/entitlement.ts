import type { Decision } from "@destack/object";
import type { Identifier } from "@destack/schema";
import type { Entitlement, EntitlementUse } from "../server/entitlement.ts";

/** The accounts' plans in a test: the uses each account was allowed, all others refused. */
export class TestEntitlement {
    /** The uses each account was allowed, by account. */
    readonly #allowed = new Map<string, Set<string>>();

    /** Allow an account a use of its plan. */
    allow(accountId: Identifier<"account">, use: EntitlementUse): void {
        const allowed = this.#allowed.get(accountId) ?? new Set<string>();
        allowed.add(TestEntitlement.#describe(use));
        this.#allowed.set(accountId, allowed);
    }

    /** Read an account's plan: within it once allowed, else unentitled. */
    of(accountId: Identifier<"account">): Entitlement {
        return {
            decide: async (use): Promise<Decision> => {
                const used = TestEntitlement.#describe(use);

                return this.#allowed.get(accountId)?.has(used) === true
                    ? { status: "true", reason: "Within", message: `${used} within the plan` }
                    : {
                          status: "false",
                          reason: "Unentitled",
                          message: `no plan of the account grants ${used}`,
                      };
            },
        };
    }

    /** Describe a use as people read it. */
    static #describe(use: EntitlementUse): string {
        return use.kind === "resource" ? use.resource : `${use.tenancy} tenancy`;
    }
}
