import caretLeft from "@destack/icon/phosphor/caret-left";
import caretRight from "@destack/icon/phosphor/caret-right";
import minusIcon from "@destack/icon/phosphor/minus";
import plusIcon from "@destack/icon/phosphor/plus";
import { t } from "@destack/locale";
import { type JsonObject, LocalReference, schema } from "@destack/schema";
import { type Bindable, type Reference } from "@destack/style";
import * as style from "@destack/style";
import { radius, space } from "@destack/theme/tokens.stylex";
import { createSignal, For, type JSX, Show, useLocale } from "@destack/view";
import { Button } from "../button/index.ts";
import {
    ColorArea,
    ColorEyeDropper,
    ColorField,
    ColorPicker,
    ColorSlider,
    ColorSwatch,
} from "../color-picker/index.ts";
import { FieldContext } from "../field/control.ts";
import { Icon } from "../icon/index.ts";
import { Popover, PopoverContent, PopoverTrigger } from "../popover/index.ts";
import { Sortable, SortableContainer, SortableItem, type SortableMove } from "../sortable/index.ts";
import { Tabs, TabsContent, TabsList, TabsTitle, TabsTrigger } from "../tabs/index.ts";
import { TokenBinding, TokenList } from "./bindable.tsx";
import { ValueSet } from "./group.tsx";
import { type NamedValueInputProperties, readValue } from "./value-field.tsx";

/** The color a list of colors adds first, before it has one to repeat. */
const FIRST_COLOR = "#ffffff";

/** The styles of the inputs of colors. */
const styles = style.create({
    picker: {
        display: "flex",
        flexDirection: "column",
        gap: space[2],
    },
    palette: {
        display: "flex",
        flexDirection: "column",
        gap: space[2],
    },
    blend: (backgroundImage: string) => ({
        height: space[2],
        borderRadius: radius.full,
        backgroundImage,
    }),
    row: {
        display: "flex",
        flexDirection: "row",
        alignItems: "center",
        gap: space[1],
    },
    opacity: {
        width: "5.5rem",
        flexShrink: 0,
    },
});

/** Render the input of a color: a swatch that opens a picker, its text and its opacity. */
export function ColorInput(properties: NamedValueInputProperties): JSX.Element {
    return (
        <ColorRow
            label={properties["aria-label"]}
            disabled={properties.disabled}
            value={typeof properties.value === "string" ? properties.value : undefined}
            onValueChange={(next) => properties.onValueChange(next)}
        />
    );
}

/** Render a color of a value, or a reference to a color token in its place. */
export function ColorPart(properties: {
    /** What the color is called. */
    readonly label: string | undefined;
    /** Whether it takes no input. */
    readonly disabled: boolean | undefined;
    /** The color, or its token. */
    readonly value: Bindable<string>;
    /** Handle the color or its token changing. */
    readonly onValueChange: (value: Bindable<string>) => void;
    /** What the picker's own tab shows in place of the color's, such as a paint's picker. */
    readonly custom?: JSX.Element;
}): JSX.Element {
    return (
        <TokenBinding
            type="color"
            label={properties.label}
            disabled={properties.disabled}
            value={properties.value}
            onValueChange={properties.onValueChange}
            detach={(probe, reference) => probe.color(reference)}
            css={(color) => color}
        >
            {(color, bind) => (
                <ColorRow
                    label={properties.label}
                    disabled={properties.disabled}
                    value={color()}
                    end={bind}
                    custom={properties.custom}
                    onValueChange={properties.onValueChange}
                    onBind={properties.onValueChange}
                />
            )}
        </TokenBinding>
    );
}

/** Render a color in a row: a swatch that opens a picker, its text and its opacity. */
function ColorRow(properties: {
    /** What the color is called. */
    readonly label: string | undefined;
    /** Whether it takes no input. */
    readonly disabled: boolean | undefined;
    /** The color, none while none is picked. */
    readonly value: string | undefined;
    /** What follows the text inside the field, such as a button to bind a token. */
    readonly end?: JSX.Element;
    /** What the picker shows in place of the color's own, such as a paint's picker. */
    readonly custom?: JSX.Element | undefined;
    /** Handle another color being picked. */
    readonly onValueChange: (value: string) => void;
    /** Bind the color to a token picked in the picker, which shows a tab of tokens for it. */
    readonly onBind?: (value: Reference) => void;
}): JSX.Element {
    // pick the color, or a token in its own tab where the color binds
    const locale = useLocale();
    const [isOpen, setOpen] = createSignal(false);
    const bind = (reference: Reference): void => {
        properties.onBind?.(reference);
        setOpen(false);
    };

    // show the app's own picker, else the kit's
    const own = (): JSX.Element =>
        properties.custom ?? (
            <ColorChooser value={properties.value} onValueChange={properties.onValueChange} />
        );

    return (
        <ColorPicker
            aria-label={properties.label}
            disabled={properties.disabled}
            value={properties.value}
            onValueChange={properties.onValueChange}
            xstyle={styles.row}
        >
            <ColorField
                aria-label={properties.label}
                opacity="apart"
                start={
                    <Popover open={isOpen()} onOpenChange={setOpen}>
                        <PopoverTrigger
                            render={(attributes) => (
                                <Button
                                    {...attributes}
                                    variant="ghost"
                                    size="sm"
                                    isIconOnly
                                    disabled={properties.disabled}
                                    aria-label={locale.render(t.context("action")`Pick a color`)}
                                >
                                    <ColorSwatch color={properties.value} />
                                </Button>
                            )}
                        />
                        <PopoverContent aria-label={properties.label} xstyle={styles.picker}>
                            <Show when={properties.onBind} fallback={own()}>
                                <Tabs defaultValue="custom">
                                    <TabsList aria-label={locale.render(t.context("title")`Color`)}>
                                        <TabsTrigger value="custom">
                                            <TabsTitle>
                                                {locale.render(t.context("title")`Custom`)}
                                            </TabsTitle>
                                        </TabsTrigger>
                                        <TabsTrigger value="tokens">
                                            <TabsTitle>
                                                {locale.render(t.context("title")`Tokens`)}
                                            </TabsTitle>
                                        </TabsTrigger>
                                    </TabsList>
                                    <TabsContent value="custom">{own()}</TabsContent>
                                    <TabsContent value="tokens">
                                        <TokenList
                                            type="color"
                                            root={undefined}
                                            isShown={isOpen()}
                                            onPick={bind}
                                        />
                                    </TabsContent>
                                </Tabs>
                            </Show>
                        </PopoverContent>
                    </Popover>
                }
                end={properties.end}
            />
            <FieldContext value={null}>
                <ColorField channel="alpha" xstyle={styles.opacity} />
            </FieldContext>
        </ColorPicker>
    );
}

/** Render a palette: swatches a drag reorders, each picked or removed in its popover. */
export function PaletteInput(properties: NamedValueInputProperties): JSX.Element {
    // key each color by its value and how often it came before, as colors repeat
    const locale = useLocale();
    const colors = (): readonly string[] =>
        readValue(properties.value, schema.array(schema.string())) ?? [];
    const entries = (): readonly { readonly key: string; readonly color: string }[] => {
        const seen = new Map<string, number>();

        return colors().map((each) => {
            const count = seen.get(each) ?? 0;
            seen.set(each, count + 1);

            return { key: `${each}#${String(count)}`, color: each };
        });
    };

    // write the colors back changed, within the fewest and the most
    const write = (next: readonly string[]): void => properties.onValueChange([...next]);
    const isFewest = (): boolean =>
        colors().length <= limitOf(properties.type.schema, "minItems", 0);
    const isMost = (): boolean =>
        colors().length >= limitOf(properties.type.schema, "maxItems", Infinity);
    const move = ({ id, previous }: SortableMove): void => {
        // place the moved color after the one it lands after
        const rest = entries().filter((entry) => entry.key !== id);
        const moved = entries().filter((entry) => entry.key === id);
        const index =
            previous === undefined ? 0 : rest.findIndex((entry) => entry.key === previous) + 1;
        write(
            [...rest.slice(0, index), ...moved, ...rest.slice(index)].map((entry) => entry.color),
        );
    };

    return (
        <ValueSet
            aria-label={properties["aria-label"]}
            aria-describedby={properties["aria-describedby"]}
            xstyle={styles.palette}
        >
            {/* Swatches */}
            <div {...style.attrs(styles.row)}>
                <Sortable onMove={move} indicator="gap">
                    <SortableContainer id="colors" orientation="horizontal" xstyle={styles.row}>
                        <For each={entries()}>
                            {(entry, index) => (
                                <SortableItem
                                    id={entry.key}
                                    activator="item"
                                    sensors={["pointer"]}
                                    disabled={properties.disabled === true}
                                    label={entry.color}
                                >
                                    <Popover>
                                        <PopoverTrigger
                                            render={(attributes) => (
                                                <Button
                                                    {...attributes}
                                                    variant="outline"
                                                    isIconOnly
                                                    disabled={properties.disabled}
                                                    aria-label={locale.render(
                                                        t.context("label")`Color ${index() + 1}`,
                                                    )}
                                                >
                                                    <ColorSwatch color={entry.color} />
                                                </Button>
                                            )}
                                        />
                                        <PopoverContent
                                            aria-label={locale.render(
                                                t.context("label")`Color ${index() + 1}`,
                                            )}
                                            xstyle={styles.picker}
                                        >
                                            <ColorChooser
                                                value={entry.color}
                                                onValueChange={(next) =>
                                                    write(colors().with(index(), next))
                                                }
                                            />
                                            <div {...style.attrs(styles.row)}>
                                                <Button
                                                    variant="ghost"
                                                    isIconOnly
                                                    disabled={index() === 0}
                                                    aria-label={locale.render(
                                                        t.context("action")`Move earlier`,
                                                    )}
                                                    onClick={() =>
                                                        write(swap(colors(), index() - 1))
                                                    }
                                                >
                                                    <Icon
                                                        icon={
                                                            locale.direction === "rtl"
                                                                ? caretRight
                                                                : caretLeft
                                                        }
                                                    />
                                                </Button>
                                                <Button
                                                    variant="ghost"
                                                    isIconOnly
                                                    disabled={index() === colors().length - 1}
                                                    aria-label={locale.render(
                                                        t.context("action")`Move later`,
                                                    )}
                                                    onClick={() => write(swap(colors(), index()))}
                                                >
                                                    <Icon
                                                        icon={
                                                            locale.direction === "rtl"
                                                                ? caretLeft
                                                                : caretRight
                                                        }
                                                    />
                                                </Button>
                                                <Button
                                                    variant="ghost"
                                                    disabled={isFewest()}
                                                    onClick={() =>
                                                        write(colors().toSpliced(index(), 1))
                                                    }
                                                >
                                                    <Icon icon={minusIcon} />
                                                    {locale.render(
                                                        t.context("action")`Remove color`,
                                                    )}
                                                </Button>
                                            </div>
                                        </PopoverContent>
                                    </Popover>
                                </SortableItem>
                            )}
                        </For>
                    </SortableContainer>
                </Sortable>
                <Button
                    variant="ghost"
                    isIconOnly
                    disabled={properties.disabled === true || isMost()}
                    aria-label={locale.render(t.context("action")`Add color`)}
                    onClick={() => write([...colors(), colors().at(-1) ?? FIRST_COLOR])}
                >
                    <Icon icon={plusIcon} />
                </Button>
            </div>

            {/* Blend */}
            <div {...style.attrs(styles.blend(blendOf(colors())))} />
        </ValueSet>
    );
}

/** Render a color's area, hue, opacity, text and the screen's dropper. */
function ColorChooser(properties: {
    /** The color, undefined while none is picked. */
    readonly value: string | undefined;
    /** Handle another color being picked. */
    readonly onValueChange: (value: string) => void;
}): JSX.Element {
    return (
        <ColorPicker value={properties.value} onValueChange={properties.onValueChange}>
            <ColorArea />
            <ColorSlider channel="hue" />
            <ColorSlider channel="alpha" />
            <div {...style.attrs(styles.row)}>
                <ColorEyeDropper />
                <ColorField />
            </div>
        </ColorPicker>
    );
}

/** Swap a color with the one after it. */
function swap(colors: readonly string[], index: number): readonly string[] {
    return [
        ...colors.slice(0, index),
        ...colors.slice(index, index + 2).toReversed(),
        ...colors.slice(index + 2),
    ];
}

/** Write colors as the CSS gradient that blends them evenly, flat for one alone. */
function blendOf(colors: readonly string[]): string {
    const stops = colors.length === 1 ? [...colors, ...colors] : colors;

    return `linear-gradient(to right in oklab, ${stops.join(", ")})`;
}

/** Read a limit of a schema, such as its most items, at its root or in its named definition. */
function limitOf(description: JsonObject, keyword: "minItems" | "maxItems", none: number): number {
    // take the root's own, then the named definition's
    const name = LocalReference.root(description);
    const definitions = schema
        .record(schema.string(), schema.jsonObject())
        .safeParse(description["$defs"]).data;
    const definition = name === null ? undefined : definitions?.[name];
    const found = description[keyword] ?? definition?.[keyword];

    return typeof found === "number" ? found : none;
}
