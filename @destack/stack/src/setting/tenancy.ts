import { Tenancy } from "@destack/account/object";
import { defineSetting } from "@destack/setting/declare";

/** Where a space's work runs: the scheduler places the space on a machine of its tenancy. */
export const tenancy = defineSetting({
    name: "tenancy",
    title: "Tenancy",
    description: "Where the space's work runs: shared or dedicated cloud machines, or your own.",
    schema: Tenancy,
    default: "shared",
    scope: "space",
    overrides: [],
    apply: "restart",
});
