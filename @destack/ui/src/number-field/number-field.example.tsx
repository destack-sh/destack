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
    render: () => (
        <FieldGroup xstyle={styles.wide}>
            <Field>
                <FieldLabel>Price</FieldLabel>
                <NumberField
                    defaultValue={1234.5}
                    min={0}
                    step={0.5}
                    format={{ style: "currency", currency: "EUR" }}
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
                />
            </Field>
            <Field>
                <FieldLabel>Distance</FieldLabel>
                <NumberField
                    defaultValue={42}
                    min={0}
                    format={{ style: "unit", unit: "kilometer" }}
                />
            </Field>
            <Field>
                <FieldLabel>Guests</FieldLabel>
                <NumberField defaultValue={2} min={1} max={8} />
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
    render: () => (
        <NumberField defaultValue={100} min={10} max={400} step={10} snapOnStep>
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
    render: () => {
        const [width, setWidth] = createSignal<number | undefined>(320);

        return (
            <div {...style.attrs(styles.wide)}>
                <NumberField value={width()} onValueChange={setWidth} min={0} max={1000}>
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

/** A number that takes no input, and one shown alone. */
export const numberFieldDisabled = defineExample({
    of: NumberField,
    name: "disabled",
    description: t`A number that takes no input, and one shown alone.`,
    render: () => (
        <div {...style.attrs(styles.wide)}>
            <NumberField aria-label="Disabled" defaultValue={3} disabled />
            <NumberField aria-label="Read only" defaultValue={3} readOnly />
        </div>
    ),
});
