import * as schema from "../validate/index.ts";

/** A rating as stars give it, from 0 up to the most `max` narrows it to. */
export const Rating = schema.number().int().min(0).meta({ id: "schema.rating", title: "Rating" });
/** A rating. */
export type Rating = schema.Infer<typeof Rating>;
