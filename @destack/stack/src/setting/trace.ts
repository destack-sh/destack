import { schema } from "@destack/schema";
import { defineSetting } from "@destack/setting/declare";
import { t } from "@destack/locale";

/** The share of traces a space's workloads keep beside every failed or slow one. */
export const traceSampling = defineSetting({
    name: "telemetry.sampling",
    title: t`Trace sampling`,
    description: t`The share of traces kept, beside every failed or slow one.`,
    schema: schema.number().min(0).max(1),
    default: 1,
    scope: "space",
    overrides: ["installation"],
    apply: "restart",
});
