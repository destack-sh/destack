import { type Message, t } from "@destack/locale";
import { schema } from "@destack/schema";
import { Dimension } from "../dimension/index.ts";

/** How a box sizes along one side: as layout decides, by a length, around content, or across. */
export const SizingKind = schema.enum(["auto", "fixed", "fit", "fill"]);
/** How a box sizes along one side. */
export type SizingKind = schema.Infer<typeof SizingKind>;

/** How a box sizes along one side, titled and described. */
export const SIZING_KINDS = {
    auto: {
        title: t.context("option")`Auto`,
        description: t`As the layout around it decides, as CSS's auto.`,
    },
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
    schema
        .discriminatedUnion("kind", [
            schema.object({
                /** As the layout around decides. */
                kind: schema.literal("auto"),
            }),
            schema.object({
                /** A fixed length. */
                kind: schema.literal("fixed"),
                /** The length. */
                length: Dimension.bindable,
            }),
            schema.object({
                /** Around its content. */
                kind: schema.literal("fit"),
            }),
            schema.object({
                /** Across its container. */
                kind: schema.literal("fill"),
            }),
        ])
        .meta({ id: "style.sizing", title: "Sizing" }),
    {
        /** Write a sizing as CSS's width or height: auto, a length, fit-content or stretch. */
        css(sizing: Sizing): string {
            switch (sizing.kind) {
                case "auto":
                    return "auto";
                case "fixed":
                    return Dimension.css(sizing.length);
                case "fit":
                    return "fit-content";
                case "fill":
                    return "stretch";
            }
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
            length: Dimension.bindable,
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
    /** The size CSS starts from: as the layout decides along each side, within no limits. */
    initial: {
        width: { kind: "auto" },
        height: { kind: "auto" },
        minWidth: { kind: "none" },
        maxWidth: { kind: "none" },
        minHeight: { kind: "none" },
        maxHeight: { kind: "none" },
    } satisfies Size,

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
