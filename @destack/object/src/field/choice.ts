import { Hue } from "@destack/color";
import { IconName } from "@destack/icon/name";
import { Message } from "@destack/locale";
import { schema } from "@destack/schema";

/** How a value of a field looks wherever it shows. */
export const ChoiceDefinition = schema.object({
    /** The value as people read it. */
    title: Message,
    /** The icon, by its name in the icon set. */
    icon: IconName.exactOptional(),
    /** The color. */
    color: Hue.exactOptional(),
});
/** How a value of a field looks wherever it shows. */
export type ChoiceDefinition = schema.Infer<typeof ChoiceDefinition>;

/** A value a field stores one of, with how it looks where its declaration says. */
export const FieldChoice = schema.object({
    /** The value. */
    value: schema.string(),
    /** The value as people read it, absent to read it as stored. */
    title: Message.exactOptional(),
    /** The icon, by its name in the icon set. */
    icon: IconName.exactOptional(),
    /** The color. */
    color: Hue.exactOptional(),
});
/** A value a field stores one of, with how it looks where its declaration says. */
export type FieldChoice = schema.Infer<typeof FieldChoice>;

/** How each value of a field looks, by value. */
export type ChoiceDefinitions = Readonly<Record<string, ChoiceDefinition>>;
