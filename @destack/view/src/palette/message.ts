import { CommandReference } from "@destack/object";
import { ObjectReference } from "@destack/sync";
import { schema } from "@destack/schema";
import { ActionEntry } from "./entry.ts";

/** A message between a page and the palette framed over it. */
export const PaletteMessage = schema.discriminatedUnion("action", [
    schema.object({
        /** Close the palette, from its frame. */
        action: schema.literal("palette.close"),
    }),
    schema.object({
        /** Ask the page for the actions it handles, from the frame. */
        action: schema.literal("actions.list"),
    }),
    schema.object({
        /** Answer the frame with the actions the page handles. */
        action: schema.literal("actions.listed"),
        /** The actions, as the palette offers them. */
        actions: schema.array(ActionEntry),
        /** The objects the person selected in the page, which commands on their types act on. */
        selection: schema.array(ObjectReference),
    }),
    schema.object({
        /** Run an action the page handles, from the frame. */
        action: schema.literal("action.run"),
        /** The action. */
        command: CommandReference,
    }),
]);
/** A message between a page and the palette framed over it. */
export type PaletteMessage = schema.Infer<typeof PaletteMessage>;
