import { schema } from "@destack/schema";
import { Subject } from "@destack/sync";
import { TextSchema } from "./schema.ts";

/** How deep a list item or todo nests, zero at the outermost level. */
const INDENT = schema.number().int().nonnegative();

/** The built-in mark and node types of rich text. */
export const TextTypes = {
    /** Bold text, growing as people type after it. */
    bold: TextSchema.mark("bold"),
    /** Italic text, growing as people type after it. */
    italic: TextSchema.mark("italic"),
    /** Struck-through text, growing as people type after it. */
    strike: TextSchema.mark("strike"),
    /** Inline code. */
    code: TextSchema.mark("code", { expand: "none", excludes: ["bold", "italic", "strike"] }),
    /** A link to a URL. */
    link: TextSchema.mark("link", {
        expand: "none",
        value: schema.object({ href: schema.url() }),
    }),

    /** A paragraph starting at its marker. */
    paragraph: TextSchema.node("paragraph", { isBlock: true }),
    /** A heading of a level from one to six. */
    heading: TextSchema.node("heading", {
        isBlock: true,
        value: schema.object({ level: schema.number().int().min(1).max(6) }),
    }),
    /** An item of a bulleted or numbered list, nested by its indent. */
    list: TextSchema.node("list", {
        isBlock: true,
        value: schema.object({ style: schema.enum(["bullet", "number"]), indent: INDENT }),
    }),
    /** A todo item, checked or not, nested by its indent. */
    todo: TextSchema.node("todo", {
        isBlock: true,
        value: schema.object({ isChecked: schema.boolean(), indent: INDENT }),
    }),
    /** A quotation. */
    quote: TextSchema.node("quote", { isBlock: true }),
    /** A block of code in a language, absent for plain text. */
    codeBlock: TextSchema.node("codeBlock", {
        isBlock: true,
        value: schema.object({ language: schema.string().min(1).exactOptional() }),
    }),

    /** A mention of a person, a group's members, or any object, as each is a subject. */
    mention: TextSchema.node("mention", { value: Subject, isReferencing: true }),
};

/** The rich text types: styles, links, blocks, mentions and embedded objects. */
export const RICH_TEXT = Object.values(TextTypes);
