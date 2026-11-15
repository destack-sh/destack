import { account } from "@destack/account/object";
import { RequestId } from "@destack/sync";
import { usage } from "@destack/stack/object";
import { charge } from "../object/index.ts";
import type { Metering } from "./metering.ts";

/** Implement charges: rating each claimed page of an account's usage into its lines. */
export function implementCharge(home: string, metering: Metering): typeof charge {
    return charge.implement({
        cursors: [
            // rate each payer's usage at the home, held thirty days past any rating outage
            usage.cursor(
                "usage",
                async (page, { store, transaction }) => {
                    // rate the claimed page in the cursor's commit
                    await store.mutate(
                        store.as(charge),
                        {
                            id: RequestId.create(store.environment),
                            calls: [
                                charge.call("rate", {
                                    ...charge.in(home),
                                    accountId: account.identifier(page.scope),
                                    after: page.after,
                                    through: page.through,
                                }),
                            ],
                        },
                        { transaction, scope: home },
                    );
                },
                { hold: { days: 30 } },
            ),
        ],
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
