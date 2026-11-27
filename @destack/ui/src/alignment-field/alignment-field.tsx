import { type Message, t } from "@destack/locale";
import { present, schema } from "@destack/schema";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, radius, size, space, stroke } from "@destack/theme/tokens.stylex";
import { field } from "@destack/object";
import { createControllableSignal, For, type JSX, omit, untrack, useLocale } from "@destack/view";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";
import { useFieldControl } from "../field/control.ts";
import { type ElementPartProperties, renderPart } from "../part/index.ts";

/** A place along one side of a box: its start, its center or its end. */
export const AlignmentPlace = schema.enum(["start", "center", "end"]);
/** A place along one side of a box. */
export type AlignmentPlace = schema.Infer<typeof AlignmentPlace>;

/** A place in a box, across and down, as a 3×3 grid picks it. */
export interface Alignment {
    /** The place across, from the start of a line of text. */
    readonly x: AlignmentPlace;
    /** The place down. */
    readonly y: AlignmentPlace;
}

/** The names of the nine places, down then across. */
const PLACES: Readonly<Record<AlignmentPlace, Readonly<Record<AlignmentPlace, Message>>>> = {
    start: {
        start: t.context("option")`Top start`,
        center: t.context("option")`Top center`,
        end: t.context("option")`Top end`,
    },
    center: {
        start: t.context("option")`Middle start`,
        center: t.context("option")`Middle center`,
        end: t.context("option")`Middle end`,
    },
    end: {
        start: t.context("option")`Bottom start`,
        center: t.context("option")`Bottom center`,
        end: t.context("option")`Bottom end`,
    },
};

/** The nine cells of the grid in reading order. */
const CELLS: readonly Alignment[] = AlignmentPlace.options.flatMap((y) =>
    AlignmentPlace.options.map((x) => ({ x, y })),
);

/** The styles of an alignment field. */
const styles = style.create({
    field: {
        display: "grid",
        gridTemplateColumns: "repeat(3, 1fr)",
        gridAutoRows: "1fr",
        width: "100%",
        minWidth: `calc(3 * ${size[1]})`,
        height: "100%",
        padding: space[1],
        borderRadius: radius[2],
        backgroundColor: color.muted,
    },
    cell: {
        display: "grid",
        placeItems: "center",
        minHeight: space[5],
        padding: 0,
        borderWidth: 0,
        borderRadius: radius[1],
        backgroundColor: {
            default: "transparent",
            ":hover": { default: null, [media.hover]: color.background },
        },
        color: color.mutedForeground,
        cursor: "pointer",
        outlineOffset: `calc(-1 * ${stroke.ring})`,
    },
    dot: {
        width: space[1],
        height: space[1],
        borderRadius: radius.full,
        backgroundColor: "currentColor",
    },
    chosen: {
        width: space[3],
        height: space[1],
        borderRadius: radius.full,
        backgroundColor: color.foreground,
    },
    disabled: {
        opacity: 0.5,
        pointerEvents: "none",
    },
});

/** The properties people change in an alignment field's controls and inspector. */
const ALIGNMENT_FIELD_PROPERTIES = {
    disabled: field
        .boolean()
        .labeled(t.context("label")`Disabled`)
        .described(t`Refuses choices where the place cannot change now.`)
        .default(false),
};

/** The properties of an alignment field besides its declared ones: its place. */
export type AlignmentFieldAttributes = Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "onChange"
> &
    ElementPartProperties & {
        /** The place, which makes it controlled, none while none of the nine is chosen. */
        readonly value?: Alignment | undefined;
        /** The place at first while uncontrolled, none by default. */
        readonly defaultValue?: Alignment;
        /** Handle a person choosing a place. */
        readonly onValueChange?: (value: Alignment) => void;
    };

/** Render a 3×3 grid that picks a place in a box, which arrow keys move across and down. */
export const AlignmentField = defineComponent({
    name: "alignmentField",
    properties: ALIGNMENT_FIELD_PROPERTIES,
    render(
        properties: RenderProperties<AlignmentFieldAttributes, typeof ALIGNMENT_FIELD_PROPERTIES>,
    ): JSX.Element {
        return <AlignmentGrid {...properties} />;
    },
});

/** The properties of an alignment field, its place included. */
export type AlignmentFieldProperties = ComponentProperties<typeof AlignmentField>;

/** Render the grid of an alignment field and follow its place. */
function AlignmentGrid(
    properties: RenderProperties<AlignmentFieldAttributes, typeof ALIGNMENT_FIELD_PROPERTIES>,
): JSX.Element {
    // follow the controlled place, else the field's own, the tab stop on it or the center
    const locale = useLocale();
    const fieldControl = useFieldControl();
    const [value, setValue] = createControllableSignal<Alignment | undefined>({
        isControlled: () => "value" in properties,
        value: () => properties.value,
        defaultValue: untrack(() => properties.defaultValue),
        onChange: (next) => next !== undefined && properties.onValueChange?.(next),
    });
    const rest = omit(properties, "value", "defaultValue", "onValueChange", "disabled");
    const current = (): Alignment => value() ?? { x: "center", y: "center" };
    const isChosen = (cell: Alignment): boolean => value()?.x === cell.x && value()?.y === cell.y;

    // move by one cell across or down, flipped across in right to left text, and choose it
    const keys = (event: KeyboardEvent): void => {
        // leave other keys to the page
        const steps = moveOf(event.key, locale.direction === "rtl");
        if (steps === undefined) {
            return;
        }

        // choose the cell moved to where it moved, and focus it
        event.preventDefault();
        const next = moved(current(), steps);
        if (!isChosen(next)) {
            setValue(next);
        }
        const grid = event.currentTarget instanceof HTMLElement ? event.currentTarget : undefined;
        grid?.querySelector<HTMLElement>(`[data-x="${next.x}"][data-y="${next.y}"]`)?.focus();
    };

    return renderPart(
        "div",
        "alignment-field",
        rest,
        () => [styles.field, properties.disabled && styles.disabled],
        {
            role: "radiogroup" as const,
            get id() {
                return fieldControl?.groupAttributes().id;
            },
            get "aria-labelledby"() {
                return fieldControl?.groupAttributes()["aria-labelledby"];
            },
            get "aria-disabled"() {
                return properties.disabled ? ("true" as const) : undefined;
            },
            onKeyDown: keys,
            get children() {
                return (
                    <For each={CELLS}>
                        {(cell) => (
                            <button
                                type="button"
                                role="radio"
                                data-slot="alignment-field-cell"
                                data-x={cell.x}
                                data-y={cell.y}
                                aria-checked={isChosen(cell) ? "true" : "false"}
                                aria-label={locale.render(PLACES[cell.y][cell.x])}
                                tabindex={current().x === cell.x && current().y === cell.y ? 0 : -1}
                                disabled={properties.disabled}
                                onClick={() => setValue(cell)}
                                {...style.attrs(styles.cell)}
                            >
                                <span
                                    {...style.attrs(isChosen(cell) ? styles.chosen : styles.dot)}
                                />
                            </button>
                        )}
                    </For>
                );
            },
        },
    );
}

/** Read the cells an arrow key moves across and down, none for other keys. */
function moveOf(key: string, isRightToLeft: boolean): readonly [number, number] | undefined {
    // move toward the end of a line, which is left in right to left text
    const forward = isRightToLeft ? -1 : 1;
    const moves: Readonly<Record<string, readonly [number, number]>> = {
        ArrowRight: [forward, 0],
        ArrowLeft: [-forward, 0],
        ArrowDown: [0, 1],
        ArrowUp: [0, -1],
    };

    return moves[key];
}

/** Move a place by some cells across and down, staying within the grid. */
function moved(place: Alignment, [across, down]: readonly [number, number]): Alignment {
    return { x: stepped(place.x, across), y: stepped(place.y, down) };
}

/** Move a place along one side by some cells, stopping at either end. */
function stepped(place: AlignmentPlace, by: number): AlignmentPlace {
    const index = Math.min(Math.max(AlignmentPlace.options.indexOf(place) + by, 0), 2);

    return present(AlignmentPlace.options[index], "a place along a side");
}
