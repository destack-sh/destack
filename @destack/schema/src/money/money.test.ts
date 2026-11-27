import { expect, test } from "@destack/test";
import { Money } from "./money.ts";

/** Read the messages a value of money is refused with, none for money. */
function refused(value: unknown): string[] {
    return Money.safeParse(value).error?.issues.map((issue) => issue.message) ?? [];
}

test("take whole minor units in a currency's code, refusing a fraction of a cent and a name", () => {
    expect([
        refused({ amount: 1250, currency: "EUR" }),
        refused({ amount: 1200, currency: "x-credits" }),
        refused({ amount: 12.5, currency: "euro" }),
    ]).toEqual([
        [],
        [],
        [
            "Invalid input: expected int, received number",
            "a currency is an ISO 4217 code such as EUR, or a custom one such as x-credits",
        ],
    ]);
});
