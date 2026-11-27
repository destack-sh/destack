import opacityIcon from "@destack/icon/phosphor/drop-half";
import { Opacity } from "@destack/style";
import type { JSX } from "@destack/view";
import { FieldGroup } from "../field/field.tsx";
import { Icon } from "../icon/index.ts";
import { Slider } from "../slider/index.ts";
import { NumberPart } from "./dimension.tsx";
import { ValueSet, valueStyles } from "./group.tsx";
import { type NamedValueInputProperties, readValue } from "./value-field.tsx";

/** The steps of an opacity, one percent each. */
const STEP = 0.01;

/** Render the input of an opacity: a slider from none to all, and its percent typed beside it. */
export function OpacityInput(properties: NamedValueInputProperties): JSX.Element {
    // read the opacity, opaque where none is given
    const opacity = (): Opacity => readValue(properties.value, Opacity) ?? Opacity.initial;
    const label = (): string => properties.label ?? properties["aria-label"] ?? "";

    return (
        <ValueSet label={properties.label} aria-label={properties["aria-label"]}>
            <FieldGroup columns={3}>
                {/* Slider across two cells, the percent in the third */}
                <Slider
                    xstyle={valueStyles.wide}
                    aria-label={label()}
                    min={0}
                    max={1}
                    step={STEP}
                    format={{ style: "percent" }}
                    disabled={properties.disabled}
                    value={[opacity()]}
                    onValueChange={([next]) => {
                        if (next !== undefined) {
                            properties.onValueChange(next);
                        }
                    }}
                />
                <NumberPart
                    label={label()}
                    start={<Icon icon={opacityIcon} />}
                    min={0}
                    max={1}
                    step={STEP}
                    format={{ style: "percent" }}
                    disabled={properties.disabled}
                    value={opacity()}
                    onValueChange={(next) => properties.onValueChange(next)}
                />
            </FieldGroup>
        </ValueSet>
    );
}
