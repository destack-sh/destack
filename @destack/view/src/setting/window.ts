import { schema } from "@destack/schema";
import { defineSetting } from "@destack/setting/declare";

/** A link under a release channel's scheme, such as `destack://notes--personal--florian/home`. */
const LinkText = schema.string().regex(/^destack(?:-[a-z]+)?:\/\/\S+$/u);

/** The link a new window opens, Home's start view by default. */
export const start = defineSetting({
    name: "start",
    title: "Start link",
    description: "The view or object a new window opens.",
    schema: LinkText.nullable(),
    default: null,
    scope: "user",
    overrides: ["client"],
    apply: "immediate",
});
