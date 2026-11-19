import { schema } from "@destack/schema";

/** One card per object, showing its cover. */
export const GalleryLayout = schema.object({
    /** The kind. */
    kind: schema.literal("gallery"),
});
/** One card per object, showing its cover. */
export type GalleryLayout = schema.Infer<typeof GalleryLayout>;
