import type { Interaction } from "@destack/package/declare";
import { schema } from "@destack/schema";

/** A step of a terminal world: a command line run on the owner's home computer. */
export const TerminalStep = schema.object({
    /** Run a command line until it ends, refusing a failed one. */
    action: schema.literal("run"),
    /** The command line's words, such as `["spaces", "--json"]`. */
    words: schema.array(schema.string()).min(1),
});
/** A step of a terminal world. */
export type TerminalStep = schema.Infer<typeof TerminalStep>;

/** An observation of a terminal world: what the last command line printed. */
export const TerminalObservation = schema.object({
    /** Read what the last command line printed as JSON. */
    read: schema.literal("output"),
    /** The field to read of the printed object, or of each printed item, sorted. */
    field: schema.string().min(1),
});
/** An observation of a terminal world. */
export type TerminalObservation = schema.Infer<typeof TerminalObservation>;

/** The interaction of terminal worlds, which the command line's driver plays. */
export const terminalInteraction: Interaction<
    TerminalStep,
    TerminalObservation,
    Record<string, never>
> = Object.freeze({
    name: "terminal",
    step: TerminalStep,
    observation: TerminalObservation,
    globals: schema.object({}),
});
