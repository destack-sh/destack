import { expect, test } from "@destack/test";
import { Currency, Money } from "./money.ts";

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

test("read the digits a currency writes past its major unit, none for a custom one", () => {
    expect(
        ["EUR", "JPY", "BHD", "x-credits"].map((code) => ({
            code,
            isCustom: Currency.isCustom(code),
            digits: Currency.digits(code),
        })),
    ).toEqual([
        { code: "EUR", isCustom: false, digits: 2 },
        { code: "JPY", isCustom: false, digits: 0 },
        { code: "BHD", isCustom: false, digits: 3 },
        { code: "x-credits", isCustom: true, digits: 0 },
    ]);
});
