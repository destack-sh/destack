import type { Metering } from "./metering.ts";
import { customer, Customer } from "../object/index.ts";

/** Implement customers: derive each account's entitlements. */
export function implementCustomer(metering: Metering): typeof customer {
    return customer.implement({
        methods: {
            entitle: async (call) => {
                // derive the account's entitlements
                await Customer.entitle(
                    call.input.accountId,
                    metering.catalog(call, call.self, call.scope),
                    call,
                );

                return {};
            },
        },
    });
}
