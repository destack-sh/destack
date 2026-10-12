import { schema } from "@destack/schema";
import { defineSetting } from "@destack/setting/declare";

/** The payment provider of a finance installation: none, for default products alone, or Stripe. */
export const FinanceProvider = schema.enum(["none", "stripe"]);
/** The payment provider a finance installation charges buyers through. */
export type FinanceProvider = schema.Infer<typeof FinanceProvider>;

/** The finance installation's payment provider, which its stack places. */
export const provider = defineSetting({
    name: "provider",
    title: "Payment provider",
    description: "The payment provider finance charges buyers through.",
    schema: FinanceProvider,
    default: "none",
    scope: "space",
    overrides: ["installation"],
    apply: "restart",
});

/** The distribution tag of each package's current release, whose catalog finance sells. */
export const tag = defineSetting({
    name: "tag",
    title: "Release tag",
    description: "The distribution tag of the release of each package whose catalog finance sells.",
    schema: schema.string().min(1),
    default: "latest",
    scope: "space",
    overrides: ["installation"],
    apply: "restart",
});
