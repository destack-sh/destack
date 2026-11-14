import type { Interaction } from "@destack/package/declare";
import { schema } from "@destack/schema";

/** A step of a desktop world: the desktop starting on a person's laptop, and quitting. */
export const DesktopStep = schema.object({
    /** Start the desktop until it opens its first window, or quit it until it ends. */
    action: schema.enum(["start", "quit"]),
});
/** A step of a desktop world. */
export type DesktopStep = schema.Infer<typeof DesktopStep>;

/** An observation of a desktop world: the windows the desktop opened. */
export const DesktopObservation = schema.object({
    /** Count the windows the desktop opened. */
    count: schema.literal("window"),
});
/** An observation of a desktop world. */
export type DesktopObservation = schema.Infer<typeof DesktopObservation>;

/** The interaction of desktop worlds, which the desktop's driver plays. */
export const desktopInteraction: Interaction<
    DesktopStep,
    DesktopObservation,
    Record<string, never>
> = Object.freeze({
    name: "desktop",
    step: DesktopStep,
    observation: DesktopObservation,
    globals: schema.object({}),
});
