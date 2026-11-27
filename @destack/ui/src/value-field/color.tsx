import caretLeft from "@destack/icon/phosphor/caret-left";
import caretRight from "@destack/icon/phosphor/caret-right";
import minusIcon from "@destack/icon/phosphor/minus";
import plusIcon from "@destack/icon/phosphor/plus";
import { t } from "@destack/locale";
import { type JsonObject, LocalReference, schema } from "@destack/schema";
import * as style from "@destack/style";
import { font, radius, space } from "@destack/theme/tokens.stylex";
import { For, type JSX, useLocale } from "@destack/view";
import { Button } from "../button/index.ts";
import {
    ColorArea,
    ColorField,
    ColorPicker,
    ColorSlider,
    ColorSwatch,
} from "../color-picker/index.ts";
import { Icon } from "../icon/index.ts";
import { Popover, PopoverContent, PopoverTrigger } from "../popover/index.ts";
import { Sortable, SortableContainer, SortableItem, type SortableMove } from "../sortable/index.ts";
import type { NamedValueFieldProperties } from "./value-field.tsx";

/** The color a list of colors adds first, before it has one to repeat. */
const FIRST_COLOR = "#ffffff";

/** The styles of the inputs of colors. */
const styles = style.create({
    control: {
        width: "100%",
        justifyContent: "flex-start",
        fontFamily: font.code,
    },
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
        gap: space[1],
    },
});

/** Render the input of a color: its swatch and text, which open a picker. */
export function ColorInput(properties: NamedValueFieldProperties): JSX.Element {
    const value = (): string | undefined =>
        typeof properties.value === "string" ? properties.value : undefined;

    return (
        <Popover>
            <PopoverTrigger
                render={(attributes) => (
                    <Button
                        {...attributes}
                        variant="outline"
                        aria-label={properties["aria-label"]}
                        aria-describedby={properties["aria-describedby"]}
                        disabled={properties.disabled}
                        xstyle={styles.control}
                    >
                        <ColorSwatch color={value()} />
                        <bdi>{value()}</bdi>
                    </Button>
                )}
            />
            <PopoverContent aria-label={properties["aria-label"]} xstyle={styles.picker}>
                <ColorChooser
                    value={value()}
                    onValueChange={(next) => properties.onValueChange(next)}
                />
            </PopoverContent>
        </Popover>
    );
}

/** Render a palette: swatches a drag reorders, each picked or removed in its popover. */
export function PaletteInput(properties: NamedValueFieldProperties): JSX.Element {
    // key each color by its value and how often it came before, as colors repeat
    const locale = useLocale();
    const colors = (): readonly string[] =>
        schema.array(schema.string()).safeParse(properties.value).data ?? [];
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
        <div
            role="group"
            aria-label={properties["aria-label"]}
            aria-describedby={properties["aria-describedby"]}
            {...style.attrs(styles.palette)}
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
        </div>
    );
}

/** Render a color's area, hue, opacity and text, picking one color. */
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
            <ColorField />
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
