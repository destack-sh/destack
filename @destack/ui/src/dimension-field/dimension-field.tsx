import { t } from "@destack/locale";
import { field } from "@destack/object";
import { Dimension, DimensionUnit } from "@destack/style";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import {
    createControllableSignal,
    For,
    type JSX,
    omit,
    Show,
    untrack,
    useLocale,
} from "@destack/view";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";
import { InputGroup, InputGroupAddon } from "../input-group/index.ts";
import { NumberField, NumberFieldInput, NumberFieldPrefix } from "../number-field/index.ts";
import { type ElementPartProperties, renderPart } from "../part/index.ts";
import { Select, SelectItem } from "../select/index.ts";

/** The units a dimension field offers by default: pixels, the root's font size and a share. */
const UNITS: readonly DimensionUnit[] = ["px", "rem", "%"];

/** The properties people change in a dimension field's controls and inspector. */
const DIMENSION_FIELD_PROPERTIES = {
    disabled: field
        .boolean()
        .labeled(t.context("label")`Disabled`)
        .described(t`Refuses the number and the unit, so the length cannot change now.`)
        .optional(),
};

/** The styles of a dimension field. */
const styles = style.create({
    dimension: {
        display: "flex",
        gap: space[1],
        minWidth: 0,
    },
    number: {
        flex: 1,
        minWidth: "4rem",
    },
});

/** The properties of a dimension field besides its declared ones: its length and units. */
export type DimensionFieldAttributes = Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "children" | "onChange"
> &
    ElementPartProperties & {
        /** The length, which makes it controlled. */
        readonly value?: Dimension | undefined;
        /** The length at first while uncontrolled, none by default. */
        readonly defaultValue?: Dimension;
        /** Handle each change of the length. */
        readonly onValueChange?: (value: Dimension) => void;
        /** The units it offers, pixels, rem and percent by default. */
        readonly units?: readonly DimensionUnit[] | undefined;
        /** What it is called, where no label beside it says so. */
        readonly "aria-label"?: string | undefined;
        /** The mark at the field's start, a letter or an icon that scrubs it, such as `W`. */
        readonly start?: JSX.Element;
        /** What follows the number at the field's end, such as a button to bind a token. */
        readonly end?: JSX.Element;
    };

/** Render a length or a percentage: its number, and its unit picked beside it. */
export const DimensionField = defineComponent({
    name: "dimensionField",
    properties: DIMENSION_FIELD_PROPERTIES,
    render(
        properties: RenderProperties<DimensionFieldAttributes, typeof DIMENSION_FIELD_PROPERTIES>,
    ): JSX.Element {
        // follow the controlled length, or the field's while uncontrolled
        const [value, setValue] = createControllableSignal<Dimension>({
            isControlled: () => "value" in properties,
            value: () => properties.value ?? Dimension.zero,
            defaultValue: untrack(() => properties.defaultValue) ?? Dimension.zero,
            onChange: (next) => properties.onValueChange?.(next),
        });
        const units = (): readonly DimensionUnit[] => {
            const offered = properties.units ?? UNITS;

            return offered.includes(value().unit) ? offered : [...offered, value().unit];
        };
        const rest = omit(
            properties,
            "value",
            "defaultValue",
            "onValueChange",
            "units",
            "disabled",
            "aria-label",
            "start",
            "end",
        );

        // type the number and its unit in one input, as design tools do

        return renderPart("div", "dimension-field", rest, styles.dimension, {
            get children() {
                return (
                    <NumberField
                        aria-label={properties["aria-label"]}
                        disabled={properties.disabled}
                        units={units()}
                        unit={value().unit}
                        value={value().value}
                        onUnitChange={(next) =>
                            setValue({ value: value().value, unit: DimensionUnit.parse(next) })
                        }
                        onValueChange={(next) => {
                            if (next !== undefined) {
                                setValue({ value: next, unit: value().unit });
                            }
                        }}
                    >
                        <InputGroup>
                            <Show when={properties.start}>
                                {(mark) => (
                                    <NumberFieldPrefix label={properties["aria-label"] ?? ""}>
                                        {mark()}
                                    </NumberFieldPrefix>
                                )}
                            </Show>
                            <NumberFieldInput />
                            <Show when={properties.end}>
                                {(after) => (
                                    <InputGroupAddon align="inline-end">{after()}</InputGroupAddon>
                                )}
                            </Show>
                        </InputGroup>
                    </NumberField>
                );
            },
        });
    },
});

/** The properties of a dimension field, its length included. */
export type DimensionFieldProperties = ComponentProperties<typeof DimensionField>;

/** Render a number in a unit, such as a length or a time, its unit offered among the others. */
export function UnitParts<Unit extends string>(properties: {
    /** The units it offers. */
    readonly units: readonly Unit[];
    /** Write a unit as a person reads it, as CSS writes it by default. */
    readonly title?: (unit: Unit) => string;
    /** The look of the unit's menu, such as its width in a grid. */
    readonly unitStyle?: style.Styles;
    /** The number and its unit. */
    readonly value: { readonly value: number; readonly unit: Unit };
    /** Whether it takes no input. */
    readonly disabled: boolean;
    /** Handle the number or the unit changing. */
    readonly onValueChange: (value: { value: number; unit: Unit }) => void;
}): JSX.Element {
    // offer the value's unit too, where the units leave it out
    const locale = useLocale();
    const units = (): readonly Unit[] =>
        properties.units.includes(properties.value.unit)
            ? properties.units
            : [...properties.units, properties.value.unit];

    return (
        <>
            {/* Number */}
            <NumberField
                aria-label={locale.render(t.context("label")`Amount`)}
                xstyle={styles.number}
                value={properties.value.value}
                disabled={properties.disabled}
                onValueChange={(next) =>
                    next !== undefined &&
                    properties.onValueChange({ value: next, unit: properties.value.unit })
                }
            />

            {/* Unit */}
            <Select
                aria-label={locale.render(t.context("label")`Unit`)}
                xstyle={properties.unitStyle}
                disabled={properties.disabled}
                value={properties.value.unit}
                onValueChange={(key) => {
                    const unit = units().find((each) => each === key);
                    if (unit !== undefined) {
                        properties.onValueChange({ value: properties.value.value, unit });
                    }
                }}
            >
                <For each={units()}>
                    {(unit) => (
                        <SelectItem value={unit}>{properties.title?.(unit) ?? unit}</SelectItem>
                    )}
                </For>
            </Select>
        </>
    );
}
