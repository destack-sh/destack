import { t } from "@destack/locale";
import {
    Constraint,
    ConstraintKind,
    HORIZONTAL_CONSTRAINTS,
    Position,
    POSITION_KINDS,
    PositionKind,
    VERTICAL_CONSTRAINTS,
} from "@destack/style";
import * as style from "@destack/style";
import { type JSX, Show, useLocale } from "@destack/view";
import { FieldGroup } from "../field/field.tsx";
import { ConstraintField } from "../constraint-field/index.ts";
import { DimensionPart, NumberPart } from "./dimension.tsx";
import { choiceType, ValueSet, ValuePart, valueStyles } from "./group.tsx";
import { type NamedValueInputProperties, readValue } from "./value-field.tsx";

/** The types of a position's parts. */
const TYPES = {
    kind: choiceType(PositionKind.options, POSITION_KINDS),
    x: choiceType(ConstraintKind.options, HORIZONTAL_CONSTRAINTS),
    y: choiceType(ConstraintKind.options, VERTICAL_CONSTRAINTS),
};

/** The two edges and the middle of each axis, each with its mark, as design tools letter them. */
const EDGES = {
    x: {
        start: { mark: "L", name: t.context("label")`Left` },
        end: { mark: "R", name: t.context("label")`Right` },
        center: { mark: "±", name: t.context("label")`Offset across` },
    },
    y: {
        start: { mark: "T", name: t.context("label")`Top` },
        end: { mark: "B", name: t.context("label")`Bottom` },
        center: { mark: "±", name: t.context("label")`Offset down` },
    },
} as const;

/** Render the input of a position: its placement, its constraints, and their offsets. */
export function PositionInput(properties: NamedValueInputProperties): JSX.Element {
    // read the position, the one CSS starts from where none is given
    const position = (): Position => readValue(properties.value, Position) ?? Position.initial;
    const change = (next: Partial<Position>): void =>
        properties.onValueChange({ ...position(), ...next });

    // start an axis at a kind's own offsets, keeping the axis as it is while its kind stays
    const constrain = (axis: "x" | "y", kind: ConstraintKind): Constraint => {
        const current = position()[axis];

        return current.kind === kind ? current : Constraint.of(kind);
    };

    return (
        <ValueSet label={properties.label} aria-label={properties["aria-label"]}>
            {/* How it is placed */}
            <FieldGroup columns={1}>
                <ValuePart
                    type={TYPES.kind}
                    schema={PositionKind}
                    label={t.context("label")`Placement`}
                    value={position().kind}
                    disabled={properties.disabled}
                    onValueChange={(kind) => change({ kind })}
                />
            </FieldGroup>

            {/* The constraint of each axis, beside the box that pins its edges */}
            <FieldGroup columns={2}>
                <div {...style.attrs(valueStyles.group)}>
                    <ValuePart
                        type={TYPES.x}
                        schema={ConstraintKind}
                        label={t.context("label")`Across`}
                        value={position().x.kind}
                        disabled={properties.disabled}
                        onValueChange={(kind) => change({ x: constrain("x", kind) })}
                    />
                    <ValuePart
                        type={TYPES.y}
                        schema={ConstraintKind}
                        label={t.context("label")`Down`}
                        value={position().y.kind}
                        disabled={properties.disabled}
                        onValueChange={(kind) => change({ y: constrain("y", kind) })}
                    />
                </div>
                <ConstraintField
                    aria-label={properties["aria-label"]}
                    disabled={properties.disabled}
                    value={{ x: position().x.kind, y: position().y.kind }}
                    onValueChange={(next) =>
                        change({ x: constrain("x", next.x), y: constrain("y", next.y) })
                    }
                />
            </FieldGroup>

            {/* The offsets of both axes, flowing two to a row */}
            <FieldGroup columns={2}>
                <OffsetParts
                    axis="x"
                    value={position().x}
                    disabled={properties.disabled}
                    onValueChange={(x) => change({ x })}
                />
                <OffsetParts
                    axis="y"
                    value={position().y}
                    disabled={properties.disabled}
                    onValueChange={(y) => change({ y })}
                />
            </FieldGroup>
        </ValueSet>
    );
}

/** Render the offsets of one axis: from an edge, both edges, the middle, or shares. */
function OffsetParts(properties: {
    /** The axis. */
    readonly axis: "x" | "y";
    /** The axis's constraint. */
    readonly value: Constraint;
    /** Whether it takes no input. */
    readonly disabled: boolean | undefined;
    /** Handle the offsets changing. */
    readonly onValueChange: (value: Constraint) => void;
}): JSX.Element {
    // read the constraint by its kind
    const locale = useLocale();
    const name = (key: "start" | "end" | "center"): string =>
        locale.render(EDGES[properties.axis][key].name);
    const mark = (key: "start" | "end" | "center"): string => EDGES[properties.axis][key].mark;
    const pinned = () => {
        const constraint = properties.value;

        return constraint.kind === "start" || constraint.kind === "end" ? constraint : undefined;
    };
    const centered = () => (properties.value.kind === "center" ? properties.value : undefined);
    const both = () => (properties.value.kind === "both" ? properties.value : undefined);
    const shares = () => (properties.value.kind === "scale" ? properties.value : undefined);

    return (
        <>
            {/* An offset from one edge */}
            <Show when={pinned()}>
                {(edge) => (
                    <DimensionPart
                        label={name(edge().kind)}
                        start={mark(edge().kind)}
                        disabled={properties.disabled}
                        value={edge().offset}
                        onValueChange={(offset) => properties.onValueChange({ ...edge(), offset })}
                    />
                )}
            </Show>

            {/* An offset from the middle */}
            <Show when={centered()}>
                {(middle) => (
                    <DimensionPart
                        label={name("center")}
                        start={mark("center")}
                        disabled={properties.disabled}
                        value={middle().offset}
                        onValueChange={(offset) =>
                            properties.onValueChange({ ...middle(), offset })
                        }
                    />
                )}
            </Show>

            {/* Insets from both edges */}
            <Show when={both()}>
                {(insets) => (
                    <>
                        <DimensionPart
                            label={name("start")}
                            start={mark("start")}
                            disabled={properties.disabled}
                            value={insets().start}
                            onValueChange={(start) =>
                                properties.onValueChange({ ...insets(), start })
                            }
                        />
                        <DimensionPart
                            label={name("end")}
                            start={mark("end")}
                            disabled={properties.disabled}
                            value={insets().end}
                            onValueChange={(end) => properties.onValueChange({ ...insets(), end })}
                        />
                    </>
                )}
            </Show>

            {/* Shares of the container before each edge */}
            <Show when={shares()}>
                {(scale) => (
                    <>
                        <NumberPart
                            label={name("start")}
                            start={mark("start")}
                            min={0}
                            max={1}
                            step={0.01}
                            format={{ style: "percent" }}
                            disabled={properties.disabled}
                            value={scale().start}
                            onValueChange={(start) =>
                                properties.onValueChange({ ...scale(), start })
                            }
                        />
                        <NumberPart
                            label={name("end")}
                            start={mark("end")}
                            min={0}
                            max={1}
                            step={0.01}
                            format={{ style: "percent" }}
                            disabled={properties.disabled}
                            value={scale().end}
                            onValueChange={(end) => properties.onValueChange({ ...scale(), end })}
                        />
                    </>
                )}
            </Show>
        </>
    );
}
