import { Table } from "@destack/db";
import { price, Price, product } from "../object/index.ts";
import type { Metering } from "./metering.ts";

/** Implement prices: their terms are checked, and new declared terms replace the declared price. */
export function implementPrice(metering: Metering): typeof price {
    return price.implement({
        methods: {
            create: {
                authorize: (context) =>
                    Price.require(
                        context.input.parentId,
                        Price.terms(Table.decode(price.table, context.input)),
                        context,
                        metering.reader(context, context.self, context.scope),
                    ),
            },
        },
        declaration: {
            // apply after the product once a metered price's meter is published, and archive a price its stack stops declaring
            after: [product],
            resolve: async (_name, declared, plan) => {
                if (declared.recurring.usage === "metered") {
                    await metering.requirePublished(plan, declared.recurring.meter.packageId);
                }

                return declared;
            },
            retire: "archived",
        },
    });
}
