import { charge, PAGE_EVENTS, usage } from "../object/index.ts";
import type { Metering } from "./metering.ts";

/** Implement charges: rating the next page of an account's usage into its lines. */
export function implementCharge(metering: Metering): typeof charge {
    return charge.implement({
        methods: {
            rate: async (call) => {
                // read the account's next page of uses past where it stands rated, and rate it
                const { accountId, after } = call.input;
                const page = await call
                    .events(usage.eventKind())
                    .changes({ scopes: [accountId] }, after, PAGE_EVENTS);
                await metering.rate(accountId, page.events, call);

                return { sequence: page.sequence, uses: page.events.length };
            },
        },
    });
}
