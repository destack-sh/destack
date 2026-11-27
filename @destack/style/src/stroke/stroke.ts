import { type Message, t } from "@destack/locale";
import { schema } from "@destack/schema";
import { Color } from "../color/index.ts";
import { Dimension } from "../dimension/index.ts";

/** Where a stroke runs along a box's edge, as CSS's border and outline draw. */
export const STROKE_ALIGNS = {
    inside: { title: t.context("option")`Inside`, description: t`Within the box, as a border.` },
    outside: {
        title: t.context("option")`Outside`,
        description: t`Around the box, as an outline that takes no room.`,
    },
} satisfies Record<string, { readonly title: Message; readonly description: Message }>;

/** The lines a stroke draws, as CSS's border styles name them. */
export const STROKE_STYLES = {
    solid: { title: t.context("option")`Solid` },
    dashed: { title: t.context("option")`Dashed` },
    dotted: { title: t.context("option")`Dotted` },
} satisfies Record<string, { readonly title: Message }>;

/** A line along a box's edge: its color, width, where it runs and how it is drawn. */
const strokeJson = schema
    .object({
        /** The line's color. */
        color: Color.schema,
        /** The line's width. */
        width: Dimension,
        /** Where it runs: inside the box, or around it. */
        align: schema.enum(["inside", "outside"]),
        /** How it is drawn. */
        style: schema.enum(["solid", "dashed", "dotted"]),
    })
    .meta({ id: "stroke", title: "Stroke" });
/** A line along a box's edge. */
export type Stroke = schema.Infer<typeof strokeJson>;

/** A line along a box's edge: its color, width, where it runs and how it is drawn. */
export const Stroke = Object.assign(strokeJson, {
    /** Draw a stroke as CSS's border inside the box, or its outline around it. */
    css(stroke: Stroke): StrokeStyle {
        // draw inside as a border, and outside as an outline flush with the edge
        const { color, width, style } = stroke;

        return stroke.align === "inside"
            ? { borderColor: color, borderWidth: width, borderStyle: style }
            : {
                  outlineColor: color,
                  outlineWidth: width,
                  outlineStyle: style,
                  outlineOffset: "0",
              };
    },
});

/** A stroke as CSS properties: a border's, or an outline's. */
export type StrokeStyle =
    | {
          /** The border's color. */
          readonly borderColor: string;
          /** The border's width. */
          readonly borderWidth: string;
          /** The border's style. */
          readonly borderStyle: string;
      }
    | {
          /** The outline's color. */
          readonly outlineColor: string;
          /** The outline's width. */
          readonly outlineWidth: string;
          /** The outline's style. */
          readonly outlineStyle: string;
          /** How far the outline stands off the edge. */
          readonly outlineOffset: string;
      };
