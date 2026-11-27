import minusIcon from "@destack/icon/phosphor/minus";
import plusIcon from "@destack/icon/phosphor/plus";
import { t } from "@destack/locale";
import { Color, Dimension, Shadow, ShadowStack } from "@destack/style";
import * as style from "@destack/style";
import { For, type JSX, useLocale } from "@destack/view";
import { Button } from "../button/index.ts";
import { Icon } from "../icon/index.ts";
import { Label } from "../label/index.ts";
import { Switch } from "../switch/index.ts";
import { jsonType, ValueGroup, ValuePart, valueStyles } from "./group.tsx";
import type { NamedValueFieldProperties } from "./value-field.tsx";
import { DimensionPart } from "./dimension.tsx";

/** The types of a shadow's parts, and of each shadow of a stack. */
const TYPES = {
    color: jsonType(Color.bindable),
    shadow: jsonType(Shadow),
};

/** The shadow an input adds, a soft drop below the box. */
const START_SHADOW: Shadow = {
    color: "#00000033",
    offsetX: Dimension.zero,
    offsetY: { value: 4, unit: "px" },
    blur: { value: 12, unit: "px" },
    spread: Dimension.zero,
    inset: false,
};

/** The lengths of a shadow an input shows, with what each is called. */
const SHADOW_LENGTHS = [
    ["offsetX", t.context("label")`Across`],
    ["offsetY", t.context("label")`Down`],
    ["blur", t.context("label")`Blur`],
    ["spread", t.context("label")`Spread`],
] as const;

/** Render the input of a shadow: its color, whether it falls inside, and its four lengths. */
export function ShadowInput(properties: NamedValueFieldProperties): JSX.Element {
    // read the shadow, a soft drop where none is given
    const locale = useLocale();
    const shadow = (): Shadow => Shadow.safeParse(properties.value).data ?? START_SHADOW;
    const change = (next: Partial<Shadow>): void =>
        properties.onValueChange({ ...shadow(), ...next });

    return (
        <ValueGroup aria-label={properties["aria-label"]} xstyle={valueStyles.group}>
            {/* Color, and whether it falls inside */}
            <div {...style.attrs(valueStyles.row)}>
                <ValuePart
                    type={TYPES.color}
                    schema={Color.bindable}
                    label={t.context("label")`Color`}
                    value={shadow().color}
                    disabled={properties.disabled}
                    onValueChange={(color) => change({ color })}
                />
                <Label>
                    <Switch
                        disabled={properties.disabled}
                        checked={shadow().inset}
                        onCheckedChange={(inset) => change({ inset })}
                    />
                    {locale.render(t.context("label")`Inside`)}
                </Label>
            </div>

            {/* Offset, blur and spread */}
            <div {...style.attrs(valueStyles.grid)}>
                <For each={SHADOW_LENGTHS}>
                    {([key, label]) => (
                        <DimensionPart
                            label={locale.render(label)}
                            units={["px", "rem"]}
                            disabled={properties.disabled === true}
                            value={shadow()[key]}
                            onValueChange={(length) => change({ [key]: length })}
                        />
                    )}
                </For>
            </div>
        </ValueGroup>
    );
}

/** Render the input of shadows: an input for each, first on top. */
export function ShadowStackInput(properties: NamedValueFieldProperties): JSX.Element {
    // read the shadows, none where none are given
    const locale = useLocale();
    const shadows = (): readonly Shadow[] => ShadowStack.safeParse(properties.value).data ?? [];
    const write = (next: readonly Shadow[]): void => properties.onValueChange([...next]);

    return (
        <ValueGroup aria-label={properties["aria-label"]} xstyle={valueStyles.group}>
            {/* Shadows, first on top, each row kept by its place as it changes */}
            <For each={shadows()} keyed={false}>
                {(shadow, index) => (
                    <div {...style.attrs(valueStyles.item)}>
                        <ValuePart
                            type={TYPES.shadow}
                            schema={Shadow}
                            label={t.context("label")`Shadow ${index + 1}`}
                            value={shadow()}
                            disabled={properties.disabled}
                            onValueChange={(changed) => write(shadows().with(index, changed))}
                        />
                        <Button
                            variant="ghost"
                            isIconOnly
                            disabled={properties.disabled}
                            aria-label={locale.render(t.context("action")`Remove shadow`)}
                            onClick={() => write(shadows().toSpliced(index, 1))}
                        >
                            <Icon icon={minusIcon} />
                        </Button>
                    </div>
                )}
            </For>

            {/* Add one on top */}
            <Button
                variant="ghost"
                disabled={properties.disabled}
                onClick={() => write([START_SHADOW, ...shadows()])}
            >
                <Icon icon={plusIcon} />
                {locale.render(t.context("action")`Add shadow`)}
            </Button>
        </ValueGroup>
    );
}
