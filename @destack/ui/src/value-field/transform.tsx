import { t } from "@destack/locale";
import { Dimension, Transform } from "@destack/style";
import { type JSX, useLocale } from "@destack/view";
import { NumberField } from "../number-field/index.ts";
import { ValueGroup, valueStyles } from "./group.tsx";
import type { NamedValueFieldProperties } from "./value-field.tsx";
import { DimensionPart } from "./dimension.tsx";

/** The transform an input starts at, which leaves the box as it is. */
const START_TRANSFORM: Transform = {
    x: Dimension.zero,
    y: Dimension.zero,
    rotate: 0,
    scaleX: 1,
    scaleY: 1,
};

/** Render the input of a transform: its move, turn and scale. */
export function TransformInput(properties: NamedValueFieldProperties): JSX.Element {
    // read the transform, none where none is given
    const locale = useLocale();
    const transform = (): Transform =>
        Transform.safeParse(properties.value).data ?? START_TRANSFORM;
    const change = (next: Partial<Transform>): void =>
        properties.onValueChange({ ...transform(), ...next });

    return (
        <ValueGroup aria-label={properties["aria-label"]} xstyle={valueStyles.grid}>
            <DimensionPart
                label={locale.render(t.context("label")`Across`)}
                disabled={properties.disabled === true}
                value={transform().x}
                onValueChange={(x) => change({ x })}
            />
            <DimensionPart
                label={locale.render(t.context("label")`Down`)}
                disabled={properties.disabled === true}
                value={transform().y}
                onValueChange={(y) => change({ y })}
            />
            <NumberField
                aria-label={locale.render(t.context("label")`Rotation`)}
                min={-360}
                max={360}
                format={{ style: "unit", unit: "degree" }}
                disabled={properties.disabled}
                value={transform().rotate}
                onValueChange={(rotate) => rotate !== undefined && change({ rotate })}
            />
            <NumberField
                aria-label={locale.render(t.context("label")`Width scale`)}
                step={0.1}
                disabled={properties.disabled}
                value={transform().scaleX}
                onValueChange={(scaleX) => scaleX !== undefined && change({ scaleX })}
            />
            <NumberField
                aria-label={locale.render(t.context("label")`Height scale`)}
                step={0.1}
                disabled={properties.disabled}
                value={transform().scaleY}
                onValueChange={(scaleY) => scaleY !== undefined && change({ scaleY })}
            />
        </ValueGroup>
    );
}
