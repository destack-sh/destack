import { schema } from "@destack/schema";
import { defineSetting } from "@destack/setting/declare";

/** The region the fixture serves, which every start reads. */
export const region = defineSetting({
    name: "region",
    title: "Region",
    description: "The region the fixture serves.",
    schema: schema.string().min(1),
    scope: "space",
    overrides: ["installation"],
    apply: "restart",
    required: true,
});
