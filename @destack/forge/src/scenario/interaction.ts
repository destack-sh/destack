import type { Interaction } from "@destack/package/declare";
import { schema } from "@destack/schema";

/** A step of a forge world: the publishing account's owner working on a platform repository. */
export const ForgeStep = schema.union([
    schema.object({
        /** Create a platform repository, which the later steps work on. */
        action: schema.literal("create"),
    }),
    schema.object({
        /** Commit locally, ahead of the repository. */
        action: schema.literal("commit"),
        /** The commits to write. */
        count: schema.number().int().min(1),
    }),
    schema.object({
        /** Push the next local commit to the repository through a write lease. */
        action: schema.literal("push"),
    }),
    schema.object({
        /** Clone the repository through a read lease. */
        action: schema.literal("clone"),
    }),
    schema.object({
        /** Pull the repository into the next clone through a read lease. */
        action: schema.literal("pull"),
    }),
]);
/** A step of a forge world. */
export type ForgeStep = schema.Infer<typeof ForgeStep>;

/** An observation of a forge world: whether the last clone or pull reached the last push. */
export const ForgeObservation = schema.object({
    /** Read whether the checkout last cloned or pulled is at the commit last pushed. */
    read: schema.literal("synced"),
});
/** An observation of a forge world. */
export type ForgeObservation = schema.Infer<typeof ForgeObservation>;

/** The interaction of forge worlds, which the forge's driver plays. */
export const forgeInteraction: Interaction<
    ForgeStep,
    ForgeObservation,
    Record<string, never>
> = Object.freeze({
    name: "forge",
    step: ForgeStep,
    observation: ForgeObservation,
    globals: schema.object({}),
});
