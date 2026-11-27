import { Icon } from "../icon/index.ts";
import { Button } from "../button/index.ts";
import plusIcon from "@destack/icon/phosphor/plus";
import settingsIcon from "@destack/icon/phosphor/sliders-horizontal";
import { t } from "@destack/locale";
import { schema } from "@destack/schema";
import { Dimension, Shadow, ShadowStack } from "@destack/style";
import * as style from "@destack/style";
import { For, type JSX, useLocale } from "@destack/view";
import { Popover, PopoverContent, PopoverTrigger } from "../popover/index.ts";
import { Label } from "../label/index.ts";
import { Switch } from "../switch/index.ts";
import { choiceType, jsonType, ValueStack, ValuePart, ValueSet, valueStyles } from "./group.tsx";
import { type NamedValueInputProperties, readValue } from "./value-field.tsx";
import { DimensionPart } from "./dimension.tsx";
import { ColorPart } from "./color.tsx";

/** Whether a shadow falls outside its box or inside it, as design tools name the two. */
const ShadowKind = schema.enum(["drop", "inner"]);

/** The types of a shadow's parts, and of each shadow of a stack. */
const TYPES = {
    shadow: jsonType(Shadow),
    kind: choiceType(ShadowKind.options, {
        drop: { title: t.context("option")`Drop shadow` },
        inner: { title: t.context("option")`Inner shadow` },
    }),
};

/** The styles of a shadow's popover. */
const styles = style.create({
    popover: {
        width: "18rem",
    },
});

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
export function ShadowInput(properties: NamedValueInputProperties): JSX.Element {
    // read the shadow, a soft drop where none is given
    const locale = useLocale();
    const shadow = (): Shadow => readValue(properties.value, Shadow) ?? START_SHADOW;
    const change = (next: Partial<Shadow>): void =>
        properties.onValueChange({ ...shadow(), ...next });

    return (
        <ValueSet label={properties.label} aria-label={properties["aria-label"]}>
            {/* Color, and whether it falls inside */}
            <div {...style.attrs(valueStyles.row)}>
                <ColorPart
                    label={locale.render(t.context("label")`Color`)}
                    disabled={properties.disabled}
                    value={shadow().color}
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
        </ValueSet>
    );
}

/** Render the input of shadows: a row for each, first on top, its fields in a popover. */
export function ShadowStackInput(properties: NamedValueInputProperties): JSX.Element {
    // read the shadows, none where none are given
    const locale = useLocale();
    const shadows = (): readonly Shadow[] => readValue(properties.value, ShadowStack) ?? [];
    const write = (next: readonly Shadow[]): void => properties.onValueChange([...next]);

    return (
        <ValueSet
            actions={
                <Button
                    variant="ghost"
                    size="sm"
                    isIconOnly
                    aria-label={locale.render(t.context("action")`Add shadow`)}
                    disabled={properties.disabled}
                    onClick={() => write([START_SHADOW, ...shadows()])}
                >
                    <Icon icon={plusIcon} />
                </Button>
            }
            label={properties.label}
            aria-label={properties["aria-label"]}
        >
            {/* Shadows, first on top */}
            <ValueStack
                items={shadows()}
                label={(index) => locale.render(t.context("label")`Shadow ${index + 1}`)}
                disabled={properties.disabled}
                onValueChange={write}
            >
                {(shadow, index) => {
                    const label = () => locale.render(t.context("label")`Shadow ${index + 1}`);
                    const change = (next: Partial<Shadow>): void =>
                        write(shadows().with(index, { ...shadow(), ...next }));

                    return (
                        <>
                            {/* Its fields, in a popover */}
                            <Popover>
                                <PopoverTrigger
                                    render={(attributes) => (
                                        <Button
                                            {...attributes}
                                            variant="ghost"
                                            size="sm"
                                            isIconOnly
                                            aria-label={label()}
                                            disabled={properties.disabled}
                                        >
                                            <Icon icon={settingsIcon} />
                                        </Button>
                                    )}
                                />
                                <PopoverContent aria-label={label()} xstyle={styles.popover}>
                                    <ValuePart
                                        type={TYPES.shadow}
                                        schema={Shadow}
                                        label={t.context("label")`Shadow ${index + 1}`}
                                        value={shadow()}
                                        disabled={properties.disabled}
                                        onValueChange={(changed) =>
                                            write(shadows().with(index, changed))
                                        }
                                    />
                                </PopoverContent>
                            </Popover>

                            {/* Outside the box, or inside it */}
                            <ValuePart
                                type={TYPES.kind}
                                schema={ShadowKind}
                                label={t.context("label")`Kind`}
                                value={shadow().inset ? "inner" : "drop"}
                                disabled={properties.disabled}
                                onValueChange={(kind) => change({ inset: kind === "inner" })}
                            />
                        </>
                    );
                }}
            </ValueStack>
        </ValueSet>
    );
}
