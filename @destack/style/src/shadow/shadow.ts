import { type Message, t } from "@destack/locale";
import { schema } from "@destack/schema";
import { Color } from "../color/index.ts";
import { Dimension } from "../dimension/index.ts";

/** A shadow a box casts, or one inside it, as CSS's box-shadow draws one. */
export const Shadow = schema.object({
    /** The shadow's color. */
    color: Color.schema,
    /** How far it falls to the right. */
    x: Dimension,
    /** How far it falls down. */
    y: Dimension,
    /** How far it blurs. */
    blur: Dimension,
    /** How far it grows past the box before it blurs. */
    spread: Dimension,
    /** Whether it falls inside the box. */
    isInset: schema.boolean(),
});
/** A shadow a box casts, or one inside it. */
export type Shadow = schema.Infer<typeof Shadow>;

/** The shadows of a box, the first on top, as CSS lists them. */
export const ShadowStack = Object.assign(
    schema.array(Shadow).meta({ id: "shadowStack", title: "Shadows" }),
    {
        /** Write the shadows as CSS's `box-shadow`, none for no shadow. */
        css(stack: readonly Shadow[]): string {
            const shadows = stack.map((shadow) =>
                [
                    ...(shadow.isInset ? ["inset"] : []),
                    shadow.x,
                    shadow.y,
                    shadow.blur,
                    shadow.spread,
                    shadow.color,
                ].join(" "),
            );

            return shadows.length === 0 ? "none" : shadows.join(", ");
        },
    },
);

/** What a blur softens: the box itself, or what shows through behind it. */
export const BLUR_KINDS = {
    layer: { title: t.context("option")`Layer`, description: t`The box and what it holds.` },
    backdrop: {
        title: t.context("option")`Backdrop`,
        description: t`What shows through behind the box, as frosted glass does.`,
    },
} satisfies Record<string, { readonly title: Message; readonly description: Message }>;

/** A blur of a box or of what lies behind it, as CSS's filter and backdrop filter draw it. */
const blurJson = schema
    .object({
        /** What it softens. */
        kind: schema.enum(["layer", "backdrop"]),
        /** How far it softens. */
        radius: Dimension,
    })
    .meta({ id: "blur", title: "Blur" });
/** A blur of a box or of what lies behind it. */
export type Blur = schema.Infer<typeof blurJson>;

/** A blur of a box or of what lies behind it, as CSS's filter and backdrop filter draw it. */
export const Blur = Object.assign(blurJson, {
    /** Draw a blur as CSS's `filter`, or `backdrop-filter` for what lies behind. */
    css(blur: Blur): { readonly filter: string } | { readonly backdropFilter: string } {
        const filter = `blur(${blur.radius})`;

        return blur.kind === "layer" ? { filter } : { backdropFilter: filter };
    },
});
