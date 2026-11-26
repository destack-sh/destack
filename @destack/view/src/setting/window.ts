import { schema } from "@destack/schema";
import { defineSetting } from "@destack/setting/declare";
import { t } from "@destack/locale";

/** A link under a release channel's scheme, such as `destack://notes--personal--florian/home`. */
const LinkText = schema.string().regex(/^destack(?:-[a-z]+)?:\/\/\S+$/u);

/** The link a new window opens, none for the desktop's own start view. */
export const start = defineSetting({
    name: "start",
    title: t.context("title")`Start link`,
    description: t`The view or object a new window opens.`,
    schema: LinkText.nullable(),
    default: null,
    scope: "user",
    overrides: ["device"],
});
