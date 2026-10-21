import { charge, usage } from "../object/index.ts";
import type { Metering } from "./metering.ts";

/** Implement charges: rating a claimed page of an account's usage into its lines. */
export function implementCharge(metering: Metering): typeof charge {
    return charge.implement({
        methods: {
            rate: async (call) => {
                // read the account's claimed page of uses, and rate it
                const { accountId, after, through } = call.input;
                const page = await call
                    .events(usage.eventKind())
                    .range({ scopes: [accountId] }, after, through);
                await metering.rate(accountId, page, call);

                return {};
            },
        },
    });
}
