import { t, type Message } from "@destack/locale";
import {
    Constraint,
    ConstraintKind,
    HORIZONTAL_CONSTRAINTS,
    Position,
    POSITION_KINDS,
    PositionKind,
    VERTICAL_CONSTRAINTS,
} from "@destack/style";
import { type JSX, Show, useLocale } from "@destack/view";
import { DimensionField } from "../dimension-field/index.ts";
import { NumberField } from "../number-field/index.ts";
import { choiceType, ValueGroup, ValuePart, valueStyles } from "./group.tsx";
import type { NamedValueFieldProperties } from "./value-field.tsx";

/** The types of a position's parts. */
const TYPES = {
    kind: choiceType(PositionKind.options, POSITION_KINDS),
    x: choiceType(ConstraintKind.options, HORIZONTAL_CONSTRAINTS),
    y: choiceType(ConstraintKind.options, VERTICAL_CONSTRAINTS),
};

/** The position an input starts at, in flow and pinned to the top left. */
const START_POSITION: Position = {
    kind: "static",
    x: Constraint.of("start"),
    y: Constraint.of("start"),
};

/** Render the input of a position: how a box is placed, and how it follows its container. */
export function PositionInput(properties: NamedValueFieldProperties): JSX.Element {
    // read the position, in flow where none is given
    const position = (): Position => Position.safeParse(properties.value).data ?? START_POSITION;
    const change = (next: Partial<Position>): void =>
        properties.onValueChange({ ...position(), ...next });

    return (
        <ValueGroup aria-label={properties["aria-label"]} xstyle={valueStyles.group}>
            {/* How it is placed */}
            <ValuePart
                type={TYPES.kind}
                schema={PositionKind}
                label={t.context("label")`Placement`}
                value={position().kind}
                disabled={properties.disabled}
                onValueChange={(kind) => change({ kind })}
            />

            {/* How it keeps to its container across and down */}
            <ConstraintParts
                label={t.context("label")`Across`}
                kinds={TYPES.x}
                value={position().x}
                disabled={properties.disabled}
                onValueChange={(x) => change({ x })}
            />
            <ConstraintParts
                label={t.context("label")`Down`}
                kinds={TYPES.y}
                value={position().y}
                disabled={properties.disabled}
                onValueChange={(y) => change({ y })}
            />
        </ValueGroup>
    );
}

/** Render a constraint along one axis: its kind, and its offset, insets or shares. */
function ConstraintParts(properties: {
    /** What the axis is called. */
    readonly label: Message;
    /** The kinds it offers, titled for the axis. */
    readonly kinds: ReturnType<typeof choiceType<ConstraintKind>>;
    /** The constraint. */
    readonly value: Constraint;
    /** Whether it takes no input. */
    readonly disabled: boolean | undefined;
    /** Handle another constraint. */
    readonly onValueChange: (value: Constraint) => void;
}): JSX.Element {
    // read the kind's numbers, none for the other kinds
    const locale = useLocale();
    const offset = () => ("offset" in properties.value ? properties.value : undefined);
    const insets = () => (properties.value.kind === "both" ? properties.value : undefined);
    const shares = () => (properties.value.kind === "scale" ? properties.value : undefined);

    return (
        <ValueGroup aria-label={locale.render(properties.label)} xstyle={valueStyles.row}>
            {/* Kind */}
            <ValuePart
                type={properties.kinds}
                schema={ConstraintKind}
                label={t.context("label")`Constraint`}
                value={properties.value.kind}
                disabled={properties.disabled}
                onValueChange={(kind) => properties.onValueChange(Constraint.of(kind))}
            />

            {/* An offset from an edge or the middle */}
            <Show when={offset()}>
                {(pinned) => (
                    <DimensionField
                        aria-label={locale.render(t.context("label")`Offset`)}
                        disabled={properties.disabled === true}
                        value={pinned().offset}
                        onValueChange={(next) =>
                            properties.onValueChange({ ...pinned(), offset: next })
                        }
                    />
                )}
            </Show>

            {/* Insets from both edges */}
            <Show when={insets()}>
                {(both) => (
                    <>
                        <DimensionField
                            aria-label={locale.render(t.context("label")`Start`)}
                            disabled={properties.disabled === true}
                            value={both().start}
                            onValueChange={(start) =>
                                properties.onValueChange({ ...both(), start })
                            }
                        />
                        <DimensionField
                            aria-label={locale.render(t.context("label")`End`)}
                            disabled={properties.disabled === true}
                            value={both().end}
                            onValueChange={(end) => properties.onValueChange({ ...both(), end })}
                        />
                    </>
                )}
            </Show>

            {/* Shares of the container */}
            <Show when={shares()}>
                {(scale) => (
                    <>
                        <NumberField
                            aria-label={locale.render(t.context("label")`Start`)}
                            min={0}
                            max={1}
                            step={0.01}
                            format={{ style: "percent" }}
                            disabled={properties.disabled}
                            value={scale().start}
                            onValueChange={(start) =>
                                start !== undefined &&
                                properties.onValueChange({ ...scale(), start })
                            }
                        />
                        <NumberField
                            aria-label={locale.render(t.context("label")`End`)}
                            min={0}
                            max={1}
                            step={0.01}
                            format={{ style: "percent" }}
                            disabled={properties.disabled}
                            value={scale().end}
                            onValueChange={(end) =>
                                end !== undefined && properties.onValueChange({ ...scale(), end })
                            }
                        />
                    </>
                )}
            </Show>
        </ValueGroup>
    );
}
