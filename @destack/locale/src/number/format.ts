import { schema } from "@destack/schema";

/** How a number is written: a share, an amount, a measure, and its digits, as Intl's options. */
export const NumberFormat = schema
    .object({
        /** What the number is, a plain one by default. */
        style: schema.enum(["decimal", "percent", "currency", "unit"]).exactOptional(),
        /** The currency of an amount, by its ISO 4217 code. */
        currency: schema.string().length(3).exactOptional(),
        /** How a currency is written, by its symbol by default. */
        currencyDisplay: schema.enum(["symbol", "narrowSymbol", "code", "name"]).exactOptional(),
        /** The unit of a measure, such as `kilometer`. */
        unit: schema.string().min(1).exactOptional(),
        /** How a unit is written, shortened by default. */
        unitDisplay: schema.enum(["short", "narrow", "long"]).exactOptional(),
        /** The fewest digits after the decimal separator. */
        minimumFractionDigits: schema.number().int().min(0).max(20).exactOptional(),
        /** The most digits after the decimal separator. */
        maximumFractionDigits: schema.number().int().min(0).max(20).exactOptional(),
        /** Whether digits are grouped, as in 1,000. */
        useGrouping: schema.boolean().exactOptional(),
    })
    .refine(isWritten, { message: "a number format has a currency or a unit to write" });
/** How a number is written. */
export type NumberFormat = schema.Infer<typeof NumberFormat>;

/** Report whether languages write a format, such as one of a known unit. */
function isWritten(format: Intl.NumberFormatOptions): boolean {
    // refuse what Intl does: a style without its currency or unit, and units it has none of
    try {
        return new Intl.NumberFormat("en", format).format(0) !== "";
    } catch {
        return false;
    }
}
