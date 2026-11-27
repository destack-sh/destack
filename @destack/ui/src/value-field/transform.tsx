import rotationIcon from "@destack/icon/phosphor/arrow-clockwise";
import { t } from "@destack/locale";
import { Transform } from "@destack/style";
import { type JSX, useLocale } from "@destack/view";
import { FieldGroup } from "../field/field.tsx";
import { Icon } from "../icon/index.ts";
import { ValueSet } from "./group.tsx";
import { type NamedValueInputProperties, readValue } from "./value-field.tsx";
import { DimensionPart, NumberPart } from "./dimension.tsx";

/** Render the input of a transform: its move, turn and scale. */
export function TransformInput(properties: NamedValueInputProperties): JSX.Element {
    // read the transform, none where none is given
    const locale = useLocale();
    const transform = (): Transform => readValue(properties.value, Transform) ?? Transform.initial;
    const change = (next: Partial<Transform>): void =>
        properties.onValueChange({ ...transform(), ...next });

    return (
        <ValueSet label={properties.label} aria-label={properties["aria-label"]}>
            {/* Position and rotation */}
            <FieldGroup columns={3}>
                <DimensionPart
                    label={locale.render(t.context("label")`Across`)}
                    start="X"
                    disabled={properties.disabled === true}
                    value={transform().x}
                    onValueChange={(x) => change({ x })}
                />
                <DimensionPart
                    label={locale.render(t.context("label")`Down`)}
                    start="Y"
                    disabled={properties.disabled === true}
                    value={transform().y}
                    onValueChange={(y) => change({ y })}
                />
                <NumberPart
                    label={locale.render(t.context("label")`Rotation`)}
                    start={<Icon icon={rotationIcon} />}
                    min={-360}
                    max={360}
                    format={{ style: "unit", unit: "degree", unitDisplay: "narrow" }}
                    disabled={properties.disabled}
                    value={transform().rotate}
                    onValueChange={(rotate) => change({ rotate })}
                />
            </FieldGroup>

            {/* Scale */}
            <ValueSet label={locale.render(t.context("label")`Scale`)}>
                <FieldGroup columns={2}>
                    <NumberPart
                        label={locale.render(t.context("label")`Width scale`)}
                        start="W"
                        step={0.1}
                        disabled={properties.disabled}
                        value={transform().scaleX}
                        onValueChange={(scaleX) => change({ scaleX })}
                    />
                    <NumberPart
                        label={locale.render(t.context("label")`Height scale`)}
                        start="H"
                        step={0.1}
                        disabled={properties.disabled}
                        value={transform().scaleY}
                        onValueChange={(scaleY) => change({ scaleY })}
                    />
                </FieldGroup>
            </ValueSet>
        </ValueSet>
    );
}
