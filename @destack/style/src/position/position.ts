import { type Message, t } from "@destack/locale";
import { schema } from "@destack/schema";
import { Dimension } from "../dimension/index.ts";

/** How a box is placed, as CSS's `position` names it. */
export const PositionKind = schema.enum(["static", "relative", "absolute", "fixed", "sticky"]);
/** How a box is placed. */
export type PositionKind = schema.Infer<typeof PositionKind>;

/** How a box is placed, titled and described. */
export const POSITION_KINDS = {
    static: {
        title: t.context("option")`In flow`,
        description: t`Where its container lays it out.`,
    },
    relative: {
        title: t.context("option")`Relative`,
        description: t`Moved from where its container lays it out.`,
    },
    absolute: {
        title: t.context("option")`Absolute`,
        description: t`Pinned to its container's edges, out of its layout.`,
    },
    fixed: { title: t.context("option")`Fixed`, description: t`Pinned to the screen's edges.` },
    sticky: {
        title: t.context("option")`Sticky`,
        description: t`In flow, then held at an edge as the page scrolls.`,
    },
} satisfies Record<PositionKind, { readonly title: Message; readonly description: Message }>;

/** How a box follows its container along one axis, as design tools constrain it. */
export const ConstraintKind = schema.enum(["start", "end", "both", "center", "scale"]);
/** How a box follows its container along one axis. */
export type ConstraintKind = schema.Infer<typeof ConstraintKind>;

/** How a box follows its container across, titled. */
export const HORIZONTAL_CONSTRAINTS = {
    start: { title: t.context("option")`Left` },
    end: { title: t.context("option")`Right` },
    both: { title: t.context("option")`Left and right` },
    center: { title: t.context("option")`Center` },
    scale: { title: t.context("option")`Scale` },
} satisfies Record<ConstraintKind, { readonly title: Message }>;

/** How a box follows its container down, titled. */
export const VERTICAL_CONSTRAINTS = {
    start: { title: t.context("option")`Top` },
    end: { title: t.context("option")`Bottom` },
    both: { title: t.context("option")`Top and bottom` },
    center: { title: t.context("option")`Center` },
    scale: { title: t.context("option")`Scale` },
} satisfies Record<ConstraintKind, { readonly title: Message }>;

/** A share of a container's length, from 0 to 1. */
const SHARE = schema.number().min(0).max(1);

/** How a box follows its container along one axis: pinned to an edge, both, centered or scaled. */
const constraintJson = schema.discriminatedUnion("kind", [
    schema.object({
        /** Pinned to the start, the left or the top. */
        kind: schema.literal("start"),
        /** How far it stands from the start. */
        offset: Dimension.bindable,
    }),
    schema.object({
        /** Pinned to the end, the right or the bottom. */
        kind: schema.literal("end"),
        /** How far it stands from the end. */
        offset: Dimension.bindable,
    }),
    schema.object({
        /** Pinned to both edges, growing with its container. */
        kind: schema.literal("both"),
        /** How far it stands from the start. */
        start: Dimension.bindable,
        /** How far it stands from the end. */
        end: Dimension.bindable,
    }),
    schema.object({
        /** Kept at its container's middle. */
        kind: schema.literal("center"),
        /** How far it stands past the middle, toward the end. */
        offset: Dimension.bindable,
    }),
    schema.object({
        /** Kept at shares of its container, growing with it. */
        kind: schema.literal("scale"),
        /** The share of the container before its start. */
        start: SHARE,
        /** The share of the container before its end. */
        end: SHARE,
    }),
]);
/** How a box follows its container along one axis. */
export type Constraint = schema.Infer<typeof constraintJson>;

/** How a box follows its container along one axis: pinned to an edge, both, centered or scaled. */
export const Constraint = Object.assign(constraintJson, {
    /** Start a constraint of a kind at its container's edges, the start's for an offset. */
    of(kind: ConstraintKind): Constraint {
        const zero = Dimension.zero;
        if (kind === "both") {
            return { kind, start: zero, end: zero };
        } else if (kind === "scale") {
            return { kind, start: 0, end: 1 };
        } else {
            return { kind, offset: zero };
        }
    },

    /** Place a box along one axis as CSS's insets and auto margins of its two edges. */
    css(constraint: Constraint): ConstraintStyle {
        // pin the edges the constraint names, leaving the others free
        const auto = { start: "auto", end: "auto", margin: "0" };
        if (constraint.kind === "start") {
            return { ...auto, start: Dimension.css(constraint.offset) };
        } else if (constraint.kind === "end") {
            return { ...auto, end: Dimension.css(constraint.offset) };
        } else if (constraint.kind === "both") {
            return {
                ...auto,
                start: Dimension.css(constraint.start),
                end: Dimension.css(constraint.end),
            };
        } else if (constraint.kind === "scale") {
            return {
                ...auto,
                start: percentOf(constraint.start),
                end: percentOf(1 - constraint.end),
            };
        }
        // center the box between doubled insets, so it moves by its offset
        else {
            const offset = Dimension.css(constraint.offset);

            return {
                start: `max(0px, 2 * ${offset})`,
                end: `max(0px, -2 * ${offset})`,
                margin: "auto",
            };
        }
    },
});

/** A constraint as CSS: the insets of its two edges and the margins between. */
export interface ConstraintStyle {
    /** The inset of the start edge. */
    readonly start: string;
    /** The inset of the end edge. */
    readonly end: string;
    /** The margins of both edges, auto to center. */
    readonly margin: string;
}

/** Where a box is placed: how, and how it follows its container across and down. */
const positionJson = schema
    .object({
        /** How it is placed. */
        kind: PositionKind,
        /** How it follows its container across. */
        x: Constraint,
        /** How it follows its container down. */
        y: Constraint,
    })
    .meta({ id: "style.position", title: "Position" });
/** Where a box is placed. */
export type Position = schema.Infer<typeof positionJson>;

/** Where a box is placed: how, and how it follows its container across and down. */
export const Position = Object.assign(positionJson, {
    /** Place a box as CSS's `position`, insets and margins, centering a fixed or fit size. */
    css(position: Position): PositionStyle {
        const x = Constraint.css(position.x);
        const y = Constraint.css(position.y);

        return {
            position: position.kind,
            top: y.start,
            right: x.end,
            bottom: y.end,
            left: x.start,
            marginInline: x.margin,
            marginBlock: y.margin,
        };
    },
});

/** A position as CSS properties. */
export interface PositionStyle {
    /** How it is placed. */
    readonly position: string;
    /** The top inset. */
    readonly top: string;
    /** The right inset. */
    readonly right: string;
    /** The bottom inset. */
    readonly bottom: string;
    /** The left inset. */
    readonly left: string;
    /** The margins across, auto to center. */
    readonly marginInline: string;
    /** The margins down, auto to center. */
    readonly marginBlock: string;
}

/** Write a share of a container as a CSS percentage. */
function percentOf(share: number): string {
    return `${String(share * 100)}%`;
}
