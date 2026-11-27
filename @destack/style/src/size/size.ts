import { type Message, t } from "@destack/locale";
import { schema } from "@destack/schema";
import { Dimension } from "../dimension/index.ts";

/** How a box sizes along one side: by a length, around its content, or across its container. */
export const SizingKind = schema.enum(["fixed", "fit", "fill"]);
/** How a box sizes along one side. */
export type SizingKind = schema.Infer<typeof SizingKind>;

/** How a box sizes along one side, titled and described. */
export const SIZING_KINDS = {
    fixed: { title: t.context("option")`Fixed`, description: t`A length of its own.` },
    fit: {
        title: t.context("option")`Fit`,
        description: t`Around its content, as CSS's fit-content.`,
    },
    fill: {
        title: t.context("option")`Fill`,
        description: t`Across its container, as CSS's stretch.`,
    },
} satisfies Record<SizingKind, { readonly title: Message; readonly description: Message }>;

/** How a box sizes along one side, as CSS's width or height. */
export const Sizing = Object.assign(
    schema.discriminatedUnion("kind", [
        schema.object({
            /** A fixed length. */
            kind: schema.literal("fixed"),
            /** The length. */
            length: Dimension,
        }),
        schema.object({
            /** Around its content. */
            kind: schema.literal("fit"),
        }),
        schema.object({
            /** Across its container. */
            kind: schema.literal("fill"),
        }),
    ]),
    {
        /** Write a sizing as CSS's width or height: a length, `fit-content` or `stretch`. */
        css(sizing: Sizing): string {
            return sizing.kind === "fixed"
                ? Dimension.css(sizing.length)
                : sizing.kind === "fit"
                  ? "fit-content"
                  : "stretch";
        },
    },
);
/** How a box sizes along one side. */
export type Sizing = schema.Infer<typeof Sizing>;

/** Whether a box stays within a least or greatest length along one side. */
export const SizeLimitKind = schema.enum(["none", "length"]);
/** Whether a box stays within a least or greatest length along one side. */
export type SizeLimitKind = schema.Infer<typeof SizeLimitKind>;

/** Whether a box stays within a least or greatest length, titled. */
export const SIZE_LIMIT_KINDS = {
    none: { title: t.context("option")`None` },
    length: { title: t.context("option")`Length` },
} satisfies Record<SizeLimitKind, { readonly title: Message }>;

/** A box's least or greatest length along one side, or none. */
export const SizeLimit = Object.assign(
    schema.discriminatedUnion("kind", [
        schema.object({
            /** No limit. */
            kind: schema.literal("none"),
        }),
        schema.object({
            /** The length. */
            kind: schema.literal("length"),
            /** The length. */
            length: Dimension,
        }),
    ]),
    {
        /** Write a limit as CSS writes it, `auto` for no least and `none` for no greatest. */
        css(limit: SizeLimit, none: "auto" | "none"): string {
            return limit.kind === "length" ? Dimension.css(limit.length) : none;
        },
    },
);
/** A box's least or greatest length along one side, or none. */
export type SizeLimit = schema.Infer<typeof SizeLimit>;

/** How big a box is: how it sizes along each side, and the lengths it stays within. */
const sizeJson = schema
    .object({
        /** How it sizes across. */
        width: Sizing,
        /** How it sizes down. */
        height: Sizing,
        /** The least width. */
        minWidth: SizeLimit,
        /** The greatest width. */
        maxWidth: SizeLimit,
        /** The least height. */
        minHeight: SizeLimit,
        /** The greatest height. */
        maxHeight: SizeLimit,
    })
    .meta({ id: "style.size", title: "Size" });
/** How big a box is. */
export type Size = schema.Infer<typeof sizeJson>;

/** How big a box is: how it sizes along each side, and the lengths it stays within. */
export const Size = Object.assign(sizeJson, {
    /** Size a box as CSS's width, height and their limits. */
    css(size: Size): SizeStyle {
        return {
            width: Sizing.css(size.width),
            height: Sizing.css(size.height),
            minWidth: SizeLimit.css(size.minWidth, "auto"),
            maxWidth: SizeLimit.css(size.maxWidth, "none"),
            minHeight: SizeLimit.css(size.minHeight, "auto"),
            maxHeight: SizeLimit.css(size.maxHeight, "none"),
        };
    },
});

/** A size as CSS properties. */
export interface SizeStyle {
    /** The width. */
    readonly width: string;
    /** The height. */
    readonly height: string;
    /** The least width. */
    readonly minWidth: string;
    /** The greatest width. */
    readonly maxWidth: string;
    /** The least height. */
    readonly minHeight: string;
    /** The greatest height. */
    readonly maxHeight: string;
}
