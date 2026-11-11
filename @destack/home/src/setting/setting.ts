import { schema } from "@destack/schema";
import { defineSetting } from "@destack/setting/declare";
import { shortcut } from "@destack/view/palette";
import { appearance, contrast, density, motion, textSize } from "@destack/view/setting";
import { t } from "@destack/locale";

/** A link under a release channel's scheme, such as `destack://notes--personal--florian/home`. */
const LinkText = schema.string().regex(/^destack(?:-[a-z]+)?:\/\/\S+$/u);

/** The link a new window opens, Home's start view by default. */
export const start = defineSetting({
    name: "start",
    title: t`Start link`,
    description: t`The view or object a new window opens.`,
    schema: LinkText.nullable(),
    default: null,
    scope: "user",
    overrides: ["device"],
});

/** The settings the person sets in their own scope, which the settings view edits. */
export const PERSON_SETTINGS = [start, shortcut, appearance, textSize, density, contrast, motion];
