import dotsSix from "@destack/icon/phosphor/dots-six-vertical";
import eyeIcon from "@destack/icon/phosphor/eye";
import eyeSlashIcon from "@destack/icon/phosphor/eye-slash";
import minusIcon from "@destack/icon/phosphor/minus";
import plusIcon from "@destack/icon/phosphor/plus";
import { t } from "@destack/locale";
import { BLEND_MODES, Fill, FillStack, Gradient, Paint, PAINT_KINDS } from "@destack/style";
import * as style from "@destack/style";
import { font, space } from "@destack/theme/tokens.stylex";
import { For, type JSX, useLocale } from "@destack/view";
import { Button } from "../button/index.ts";
import { Icon } from "../icon/index.ts";
import { Popover, PopoverContent, PopoverTrigger } from "../popover/index.ts";
import { Select, SelectItem } from "../select/index.ts";
import {
    Sortable,
    SortableContainer,
    SortableHandle,
    SortableItem,
    type SortableMove,
} from "../sortable/index.ts";
import { Toggle } from "../toggle/index.ts";
import type { NamedValueField } from "../value-field/index.ts";
import { GradientPicker, PaintPicker, PaintSwatch } from "./paint-picker.tsx";

/** The paint a new layer starts at, black as design tools add one. */
const NEW_LAYER: Fill = {
    paint: { kind: "solid", color: "#000000" },
    blend: "normal",
    isVisible: true,
};

/** The styles of the inputs of paints. */
const styles = style.create({
    trigger: {
        width: "100%",
        justifyContent: "flex-start",
        gap: space[2],
    },
    code: {
        fontFamily: font.code,
    },
    picker: {
        width: "20rem",
    },
    stack: {
        display: "flex",
        flexDirection: "column",
        gap: space[1],
    },
    layer: {
        display: "flex",
        alignItems: "center",
        gap: space[1],
    },
    grow: {
        flex: 1,
        minWidth: 0,
    },
});

/** The properties of the input of a named JSON value. */
type NamedProperties = Parameters<NamedValueField>[0];

/** Render the input of a paint: its swatch and kind, which open a picker. */
export function PaintInput(properties: NamedProperties): JSX.Element {
    // read the paint, none while it is not one
    const locale = useLocale();
    const paint = (): Paint | undefined => Paint.safeParse(properties.value).data;

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
                        xstyle={styles.trigger}
                    >
                        <PaintSwatch paint={paint()} />
                        {locale.render(PAINT_KINDS[paint()?.kind ?? "solid"].title)}
                    </Button>
                )}
            />
            <PopoverContent aria-label={properties["aria-label"]} xstyle={styles.picker}>
                <PaintPicker
                    value={paint()}
                    onValueChange={(next) => properties.onValueChange(next)}
                />
            </PopoverContent>
        </Popover>
    );
}

/** Render the input of a gradient: its swatch, which opens a picker. */
export function GradientInput(properties: NamedProperties): JSX.Element {
    // read the gradient, none while it is not one, and paint its swatch with it
    const gradient = () => Gradient.schema.safeParse(properties.value).data;
    const paint = (): Paint | undefined => {
        const value = gradient();

        return value === undefined ? undefined : { kind: "gradient", gradient: value };
    };

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
                        xstyle={styles.trigger}
                    >
                        <PaintSwatch paint={paint()} />
                    </Button>
                )}
            />
            <PopoverContent aria-label={properties["aria-label"]} xstyle={styles.picker}>
                <GradientPicker
                    value={gradient()}
                    onValueChange={(next) => properties.onValueChange(next)}
                />
            </PopoverContent>
        </Popover>
    );
}

/** Render a box's fill: its layers top first, each reordered, picked, blended or hidden. */
export function FillStackInput(properties: NamedProperties): JSX.Element {
    // key each layer by its place, as layers repeat, and write the stack back changed
    const locale = useLocale();
    const stack = (): readonly Fill[] => FillStack.safeParse(properties.value).data ?? [];
    const keyed = () => stack().map((fill, index) => ({ key: String(index), fill }));
    const write = (next: readonly Fill[]): void => properties.onValueChange([...next]);
    const update = (index: number, change: Partial<Fill>): void => {
        // change one layer, leaving the others
        const fill = stack()[index];
        if (fill !== undefined) {
            write(stack().with(index, { ...fill, ...change }));
        }
    };

    // reorder the layers by a drag
    const move = ({ id, previous }: SortableMove): void => {
        // place the moved layer after the one it lands after
        const rest = keyed().filter((entry) => entry.key !== id);
        const moved = keyed().filter((entry) => entry.key === id);
        const index =
            previous === undefined ? 0 : rest.findIndex((entry) => entry.key === previous) + 1;
        write([...rest.slice(0, index), ...moved, ...rest.slice(index)].map((entry) => entry.fill));
    };
    const isDisabled = (): boolean => properties.disabled === true;

    return (
        <div
            role="group"
            aria-label={properties["aria-label"]}
            aria-describedby={properties["aria-describedby"]}
            {...style.attrs(styles.stack)}
        >
            {/* Layers, top first */}
            <Sortable onMove={move}>
                <SortableContainer id="layers" xstyle={styles.stack}>
                    <For each={keyed()}>
                        {(entry, index) => (
                            <SortableItem
                                id={entry.key}
                                disabled={isDisabled()}
                                label={locale.render(t.context("label")`Layer ${index() + 1}`)}
                                xstyle={styles.layer}
                            >
                                <SortableHandle>
                                    <Icon icon={dotsSix} />
                                </SortableHandle>

                                {/* Paint, picked in a popover */}
                                <Popover>
                                    <PopoverTrigger
                                        render={(attributes) => (
                                            <Button
                                                {...attributes}
                                                variant="ghost"
                                                disabled={isDisabled()}
                                                xstyle={[styles.trigger, styles.grow]}
                                            >
                                                <PaintSwatch paint={entry.fill.paint} />
                                                {locale.render(
                                                    PAINT_KINDS[entry.fill.paint.kind].title,
                                                )}
                                            </Button>
                                        )}
                                    />
                                    <PopoverContent
                                        aria-label={locale.render(
                                            t.context("label")`Layer ${index() + 1}`,
                                        )}
                                        xstyle={styles.picker}
                                    >
                                        <PaintPicker
                                            value={entry.fill.paint}
                                            onValueChange={(paint) => update(index(), { paint })}
                                        />
                                    </PopoverContent>
                                </Popover>

                                {/* Blend */}
                                <Select
                                    aria-label={locale.render(t.context("label")`Blend mode`)}
                                    disabled={isDisabled()}
                                    value={entry.fill.blend}
                                    onValueChange={(key) => {
                                        const blend = Fill.shape.blend.safeParse(key).data;
                                        if (blend !== undefined) {
                                            update(index(), { blend });
                                        }
                                    }}
                                >
                                    <For each={Object.entries(BLEND_MODES)}>
                                        {([key, mode]) => (
                                            <SelectItem value={key}>
                                                {locale.render(mode.title)}
                                            </SelectItem>
                                        )}
                                    </For>
                                </Select>

                                {/* Shown or hidden, and removed */}
                                <Toggle
                                    disabled={isDisabled()}
                                    pressed={entry.fill.isVisible}
                                    aria-label={locale.render(t.context("action")`Show layer`)}
                                    onPressedChange={(isVisible) => update(index(), { isVisible })}
                                >
                                    <Icon icon={entry.fill.isVisible ? eyeIcon : eyeSlashIcon} />
                                </Toggle>
                                <Button
                                    variant="ghost"
                                    isIconOnly
                                    disabled={isDisabled()}
                                    aria-label={locale.render(t.context("action")`Remove layer`)}
                                    onClick={() => write(stack().toSpliced(index(), 1))}
                                >
                                    <Icon icon={minusIcon} />
                                </Button>
                            </SortableItem>
                        )}
                    </For>
                </SortableContainer>
            </Sortable>

            {/* Add a layer on top */}
            <Button
                variant="ghost"
                disabled={isDisabled()}
                onClick={() => write([NEW_LAYER, ...stack()])}
            >
                <Icon icon={plusIcon} />
                {locale.render(t.context("action")`Add layer`)}
            </Button>
        </div>
    );
}
