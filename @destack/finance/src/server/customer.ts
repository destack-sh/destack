import { customer, Customer } from "../object/index.ts";

/** Implement customers: announce each account's usage alerts. */
export function implementCustomer(): typeof customer {
    return customer.implement({
        methods: {
            announce: async (call) => {
                // announce each usage alert once per billing period
                for (const alert of call.input.alerts) {
                    await Customer.announce(call.input.accountId, alert, call);
                }

                return {};
            },
        },
    });
}
