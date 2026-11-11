import { schema } from "@destack/schema";
import { defineSetting } from "@destack/setting/declare";
import { t } from "@destack/locale";

/** The region the fixture serves, which every start reads. */
export const region = defineSetting({
    name: "region",
    title: t`Region`,
    description: t`The region the fixture serves.`,
    schema: schema.string().min(1),
    scope: "space",
    overrides: ["installation"],
    apply: "restart",
    required: true,
});
