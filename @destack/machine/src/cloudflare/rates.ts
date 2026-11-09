import { schema } from "@destack/schema";

/** What a cloud machine's Worker rates its spaces' use by, as the deployment binds it. */
export const RateCard = schema.object({
    /** The key of the SKU rating each meter at its attribute values, by their `Meter.key`. */
    skus: schema
        .record(schema.string(), schema.string())
        .transform((skus) => new Map(Object.entries(skus))),
    /** The memory each active Durable Object is billed at, in gigabytes. */
    gigabytes: schema.number().positive(),
});
/** What a cloud machine's Worker rates its spaces' use by. */
export type RateCard = schema.Infer<typeof RateCard>;
