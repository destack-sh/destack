import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { createSignal } from "@destack/view";
import { Field, FieldDescription, FieldGroup, FieldLabel } from "../field/index.ts";
import { InputGroup, InputGroupAddon } from "../input-group/index.ts";
import {
    NumberField,
    NumberFieldDecrement,
    NumberFieldIncrement,
    NumberFieldInput,
    NumberFieldPrefix,
    NumberFieldScrubArea,
} from "./number-field.tsx";

/** The styles of the examples. */
const styles = style.create({
    narrow: {
        width: "100%",
        maxWidth: "16rem",
    },
    wide: {
        display: "grid",
        gap: space[4],
        width: "100%",
        maxWidth: "20rem",
    },
    centered: {
        textAlign: "center",
    },
});

/** Numbers as amounts, shares, measures and counts. */
export const numberFieldFormats = defineExample({
    of: NumberField,
    name: "formats",
    description: t`Numbers as amounts, shares, measures and counts.`,
    render: (properties) => (
        <FieldGroup xstyle={styles.wide}>
            <Field>
                <FieldLabel>Price</FieldLabel>
                <NumberField
                    defaultValue={1234.5}
                    min={0}
                    step={0.5}
                    format={{ style: "currency", currency: "EUR" }}
                    {...properties}
                />
            </Field>
            <Field>
                <FieldLabel>Discount</FieldLabel>
                <NumberField
                    defaultValue={0.15}
                    min={0}
                    max={1}
                    step={0.05}
                    format={{ style: "percent" }}
                    {...properties}
                />
            </Field>
            <Field>
                <FieldLabel>Distance</FieldLabel>
                <NumberField
                    defaultValue={42}
                    min={0}
                    format={{ style: "unit", unit: "kilometer" }}
                    {...properties}
                />
            </Field>
            <Field>
                <FieldLabel>Guests</FieldLabel>
                <NumberField defaultValue={2} min={1} max={8} {...properties} />
                <FieldDescription>One to eight.</FieldDescription>
            </Field>
        </FieldGroup>
    ),
});

/** A number between its buttons, from the parts. */
export const numberFieldParts = defineExample({
    of: NumberField,
    name: "parts",
    description: t`A number between its buttons, from the parts.`,
    render: (properties) => (
        <NumberField defaultValue={100} min={10} max={400} step={10} snapOnStep {...properties}>
            <InputGroup xstyle={styles.narrow}>
                <InputGroupAddon>
                    <NumberFieldDecrement isIconOnly size="sm">
                        −
                    </NumberFieldDecrement>
                </InputGroupAddon>
                <NumberFieldInput aria-label="Zoom" xstyle={styles.centered} />
                <InputGroupAddon align="inline-end">
                    <NumberFieldIncrement isIconOnly size="sm">
                        +
                    </NumberFieldIncrement>
                </InputGroupAddon>
            </InputGroup>
        </NumberField>
    ),
});

/** A number its label scrubs, with what it settled on. */
export const numberFieldScrub = defineExample({
    of: NumberField,
    name: "scrub",
    description: t`A number its label scrubs, with what it settled on.`,
    render: (properties) => {
        const [width, setWidth] = createSignal<number | undefined>(320);

        return (
            <div {...style.attrs(styles.wide)}>
                <NumberField
                    {...properties}
                    value={width()}
                    onValueChange={setWidth}
                    min={0}
                    max={1000}
                >
                    <NumberFieldScrubArea>Width</NumberFieldScrubArea>
                    <InputGroup>
                        <NumberFieldInput aria-label="Width" />
                    </InputGroup>
                </NumberField>
                <output>{width() ?? "none"}</output>
            </div>
        );
    },
});

/** A width in pixels, rem or percent, typed as a sum or scrubbed by its mark. */
export const numberFieldUnits = defineExample({
    of: NumberField,
    name: "units",
    description: t`A width in pixels, rem or percent, typed as a sum or scrubbed by its mark.`,
    render: (properties) => {
        const [width, setWidth] = createSignal<number | undefined>(320);
        const [unit, setUnit] = createSignal("px");

        return (
            <div {...style.attrs(styles.wide)}>
                <NumberField
                    {...properties}
                    value={width()}
                    onValueChange={setWidth}
                    units={["px", "rem", "%"]}
                    unit={unit()}
                    onUnitChange={setUnit}
                >
                    <InputGroup>
                        <NumberFieldPrefix label="Width">W</NumberFieldPrefix>
                        <NumberFieldInput aria-label="Width" />
                    </InputGroup>
                </NumberField>
                <output>{`${String(width() ?? "none")} ${unit()}`}</output>
            </div>
        );
    },
});
