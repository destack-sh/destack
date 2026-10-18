import { Table } from "@destack/db";
import { Stack } from "@destack/object/server";
import { Digest, type JsonObject, JsonValue } from "@destack/schema";
import { price, Price, product } from "../object/index.ts";
import type { Metering } from "./metering.ts";

/** The hex digits of a price's terms digest its declaration name carries: 48 bits. */
const TERMS_DIGEST_LENGTH = 12;

/** Implement prices: their terms are checked, and new declared terms make a new price. */
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
            after: [product],
            // key each price by its terms, so new terms make a new price and archive the old
            collect: async (document) => {
                const named = Stack.nested(price, product, document).map(
                    async ([name, value]): Promise<[string, JsonObject]> => {
                        // digest the declared price's terms
                        const declared = JsonValue.object(value);
                        const terms = Price.terms(Table.decode(price.table, declared));
                        const digest = await Digest.json(terms);

                        return [`${name}-${digest.slice(0, TERMS_DIGEST_LENGTH)}`, declared];
                    },
                );

                return Object.fromEntries(await Promise.all(named));
            },
            // offer a declared price again unless it says otherwise, its create checking its terms
            resolve: (_name, declared) =>
                Promise.resolve({ active: true, ...JsonValue.object(declared) }),
            retire: "archived",
        },
    });
}
