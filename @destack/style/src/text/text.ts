import { type Message, t } from "@destack/locale";
import { schema } from "@destack/schema";
import { Color } from "../color/index.ts";
import { Dimension } from "../dimension/index.ts";

/** Where lines of text sit across their box, as CSS's `text-align` names it. */
export const TextAlign = schema.enum(["start", "center", "end", "justify"]);
/** Where lines of text sit across their box. */
export type TextAlign = schema.Infer<typeof TextAlign>;

/** How text is cased, as CSS's `text-transform` names it. */
export const TextCase = schema.enum(["none", "uppercase", "lowercase", "capitalize"]);
/** How text is cased. */
export type TextCase = schema.Infer<typeof TextCase>;

/** The line drawn with text, as CSS's `text-decoration-line` names it. */
export const TextDecoration = schema.enum(["none", "underline", "line-through", "overline"]);
/** The line drawn with text. */
export type TextDecoration = schema.Infer<typeof TextDecoration>;

/** How text breaks into lines, as CSS's `text-wrap` names it. */
export const TextWrap = schema.enum(["wrap", "nowrap", "balance", "pretty"]);
/** How text breaks into lines. */
export type TextWrap = schema.Infer<typeof TextWrap>;

/** Where lines of text sit across their box, titled. */
export const TEXT_ALIGNS = {
    start: { title: t.context("option")`Start` },
    center: { title: t.context("option")`Center` },
    end: { title: t.context("option")`End` },
    justify: { title: t.context("option")`Justify` },
} satisfies Record<TextAlign, { readonly title: Message }>;

/** How text is cased, titled. */
export const TEXT_CASES = {
    none: { title: t.context("option")`As typed` },
    uppercase: { title: t.context("option")`Uppercase` },
    lowercase: { title: t.context("option")`Lowercase` },
    capitalize: { title: t.context("option")`Capitalize` },
} satisfies Record<TextCase, { readonly title: Message }>;

/** The line drawn with text, titled. */
export const TEXT_DECORATIONS = {
    none: { title: t.context("option")`None` },
    underline: { title: t.context("option")`Underline` },
    "line-through": { title: t.context("option")`Strikethrough` },
    overline: { title: t.context("option")`Overline` },
} satisfies Record<TextDecoration, { readonly title: Message }>;

/** How text breaks into lines, titled and described. */
export const TEXT_WRAPS = {
    wrap: { title: t.context("option")`Wrap`, description: t`Breaks where a line is full.` },
    nowrap: { title: t.context("option")`No wrap`, description: t`Keeps to one line.` },
    balance: {
        title: t.context("option")`Balance`,
        description: t`Evens out the lines' lengths, as for headings.`,
    },
    pretty: {
        title: t.context("option")`Pretty`,
        description: t`Keeps a last line from standing alone, as for paragraphs.`,
    },
} satisfies Record<TextWrap, { readonly title: Message; readonly description: Message }>;

/** Whether text shows in full, or is cut with an ellipsis after some lines. */
export const TruncationKind = schema.enum(["none", "lines"]);
/** Whether text shows in full, or is cut after some lines. */
export type TruncationKind = schema.Infer<typeof TruncationKind>;

/** Whether text shows in full, or is cut after some lines, titled. */
export const TRUNCATION_KINDS = {
    none: { title: t.context("option")`In full` },
    lines: { title: t.context("option")`Cut after lines` },
} satisfies Record<TruncationKind, { readonly title: Message }>;

/** How text past its lines ends: shown in full, or cut with an ellipsis after some lines. */
export const Truncation = schema.discriminatedUnion("kind", [
    schema.object({
        /** Shown in full. */
        kind: schema.literal("none"),
    }),
    schema.object({
        /** Cut with an ellipsis. */
        kind: schema.literal("lines"),
        /** The lines it shows. */
        count: schema.number().int().positive(),
    }),
]);
/** How text past its lines ends. */
export type Truncation = schema.Infer<typeof Truncation>;

/** How text is laid out in its box: aligned, cased, decorated, wrapped and truncated. */
const textFormatJson = schema
    .object({
        /** Where its lines sit. */
        align: TextAlign,
        /** How it is cased. */
        case: TextCase,
        /** The line drawn with it. */
        decoration: TextDecoration,
        /** How it breaks into lines. */
        wrap: TextWrap,
        /** How it ends past its lines. */
        truncation: Truncation,
    })
    .meta({ id: "style.textFormat", title: "Text format" });
/** How text is laid out in its box. */
export type TextFormat = schema.Infer<typeof textFormatJson>;

/** How text is laid out in its box: aligned, cased, decorated, wrapped and truncated. */
export const TextFormat = Object.assign(textFormatJson, {
    /** Lay text out as CSS's text properties, clamping its lines where it truncates. */
    css(format: TextFormat): TextFormatStyle {
        // cut text past its lines with an ellipsis, as CSS clamps lines
        const truncation =
            format.truncation.kind === "lines"
                ? {
                      overflow: "hidden",
                      display: "-webkit-box",
                      WebkitBoxOrient: "vertical",
                      WebkitLineClamp: String(format.truncation.count),
                  }
                : {};

        return {
            textAlign: format.align,
            textTransform: format.case,
            textDecorationLine: format.decoration,
            textWrap: format.wrap,
            ...truncation,
        };
    },
});

/** A text format as CSS properties. */
export interface TextFormatStyle {
    /** The alignment. */
    readonly textAlign: string;
    /** The casing. */
    readonly textTransform: string;
    /** The decoration. */
    readonly textDecorationLine: string;
    /** The wrapping. */
    readonly textWrap: string;
    /** The overflow, hidden where it truncates. */
    readonly overflow?: string;
    /** The display, a clamped box where it truncates. */
    readonly display?: string;
    /** The box's direction, vertical where it truncates. */
    readonly WebkitBoxOrient?: string;
    /** The lines it shows where it truncates. */
    readonly WebkitLineClamp?: string;
}

/** A line around each letter of text: its color and width, as CSS's `-webkit-text-stroke`. */
const textStrokeJson = schema
    .object({
        /** The line's color. */
        color: Color.schema,
        /** The line's width. */
        width: Dimension,
    })
    .meta({ id: "style.textStroke", title: "Text stroke" });
/** A line around each letter of text. */
export type TextStroke = schema.Infer<typeof textStrokeJson>;

/** A line around each letter of text: its color and width, as CSS's `-webkit-text-stroke`. */
export const TextStroke = Object.assign(textStrokeJson, {
    /** Write the line as CSS's `-webkit-text-stroke`. */
    css(stroke: TextStroke): string {
        return `${Dimension.css(stroke.width)} ${stroke.color}`;
    },
});
