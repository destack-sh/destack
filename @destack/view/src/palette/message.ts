import { DeclarationName, PackageId } from "@destack/package";
import { ObjectReference } from "@destack/sync";
import { schema } from "@destack/schema";
import { RegisteredCommand } from "./registry.ts";

/** A message between a page and the palette framed over it. */
export const PaletteMessage = schema.discriminatedUnion("action", [
    schema.object({
        /** Close the palette, from its frame. */
        action: schema.literal("palette.close"),
    }),
    schema.object({
        /** Ask the page for the commands its view registers, from the frame. */
        action: schema.literal("commands.list"),
    }),
    schema.object({
        /** Answer the frame with the commands the view registers. */
        action: schema.literal("commands.listed"),
        /** The package serving the view. */
        packageId: PackageId,
        /** The commands. */
        commands: schema.array(RegisteredCommand),
        /** The objects the person selected in the page, which commands on their types act on. */
        selection: schema.array(ObjectReference),
    }),
    schema.object({
        /** Run a command the view registers, from the frame. */
        action: schema.literal("command.run"),
        /** The command's name. */
        name: DeclarationName,
        /** The input the palette asked for. */
        input: schema.jsonObject(),
    }),
]);
/** A message between a page and the palette framed over it. */
export type PaletteMessage = schema.Infer<typeof PaletteMessage>;
