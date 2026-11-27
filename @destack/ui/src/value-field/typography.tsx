import spacingIcon from "@destack/icon/phosphor/arrows-horizontal";
import lineHeightIcon from "@destack/icon/phosphor/arrows-vertical";
import sizeIcon from "@destack/icon/phosphor/text-aa";
import { t } from "@destack/locale";
import { present } from "@destack/schema";
import {
    Dimension,
    FontFamily,
    FontWeight,
    Reference,
    type Typeface,
    Typography,
} from "@destack/style";
import { type JSX, Show, useLocale } from "@destack/view";
import { FieldGroup } from "../field/field.tsx";
import { Icon } from "../icon/index.ts";
import { jsonType, ValueNone, ValuePart, ValueSet } from "./group.tsx";
import { TypefaceCatalog, TypefaceContext } from "./typeface.ts";
import { type NamedValueInputProperties, readValue } from "./value-field.tsx";
import { DimensionPart, NumberPart } from "./dimension.tsx";

/** The types of a typography's parts. */
const TYPES = {
    fontFamily: jsonType(FontFamily.bindable),
    fontWeight: jsonType(FontWeight.bindable),
};

/** The typography a person adds, the browser's body text. */
const NEW_TYPOGRAPHY: Typography = {
    fontFamily: ["sans-serif"],
    fontSize: { value: 1, unit: "rem" },
    fontWeight: 400,
    letterSpacing: Dimension.zero,
    lineHeight: 1.5,
};

/** Render the input of typography: its fonts, size, weight, letter spacing and line height. */
export function TypographyInput(properties: NamedValueInputProperties): JSX.Element {
    // read the typography, the browser's body text where none is given
    const locale = useLocale();
    const typography = (): Typography | undefined => readValue(properties.value, Typography);
    const change = (next: Partial<Typography>): void =>
        properties.onValueChange({ ...present(typography(), "the value changed"), ...next });

    // find the typeface of the first font set, whose faces the weight lists
    const catalog = new TypefaceCatalog(document);
    const typeface = (): Typeface | undefined => {
        const families = typography()?.fontFamily;

        return families === undefined || Reference.is(families)
            ? undefined
            : catalog.find(families[0] ?? "");
    };

    return (
        <Show
            when={typography()}
            fallback={
                <ValueNone
                    label={properties.label}
                    aria-label={properties["aria-label"]}
                    disabled={properties.disabled}
                    onAdd={() => properties.onValueChange(NEW_TYPOGRAPHY)}
                />
            }
        >
            {(current) => (
                <TypefaceContext value={typeface}>
                    <ValueSet label={properties.label} aria-label={properties["aria-label"]}>
                        {/* Fonts */}
                        <FieldGroup columns={1}>
                            <ValuePart
                                type={TYPES.fontFamily}
                                schema={FontFamily.bindable}
                                label={t.context("label")`Font family`}
                                value={current().fontFamily}
                                disabled={properties.disabled}
                                onValueChange={(fontFamily) => change({ fontFamily })}
                            />
                        </FieldGroup>

                        {/* Weight and size */}
                        <FieldGroup columns={2}>
                            <ValuePart
                                type={TYPES.fontWeight}
                                schema={FontWeight.bindable}
                                label={t.context("label")`Weight`}
                                value={current().fontWeight}
                                disabled={properties.disabled}
                                onValueChange={(fontWeight) => change({ fontWeight })}
                            />
                            <DimensionPart
                                label={locale.render(t.context("label")`Size`)}
                                start={<Icon icon={sizeIcon} />}
                                units={["px", "rem", "em"]}
                                disabled={properties.disabled === true}
                                value={current().fontSize}
                                onValueChange={(fontSize) => change({ fontSize })}
                            />
                        </FieldGroup>

                        {/* Line height and letter spacing */}
                        <FieldGroup columns={2}>
                            <NumberPart
                                label={locale.render(t.context("label")`Line height`)}
                                start={<Icon icon={lineHeightIcon} />}
                                min={0.5}
                                max={4}
                                step={0.05}
                                disabled={properties.disabled}
                                value={current().lineHeight}
                                onValueChange={(lineHeight) => change({ lineHeight })}
                            />
                            <DimensionPart
                                label={locale.render(t.context("label")`Letter spacing`)}
                                start={<Icon icon={spacingIcon} />}
                                units={["px", "em"]}
                                disabled={properties.disabled === true}
                                value={current().letterSpacing}
                                onValueChange={(letterSpacing) => change({ letterSpacing })}
                            />
                        </FieldGroup>
                    </ValueSet>
                </TypefaceContext>
            )}
        </Show>
    );
}
