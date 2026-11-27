import { Button } from "../button/index.ts";
import positionIcon from "@destack/icon/phosphor/arrows-horizontal";
import minusIcon from "@destack/icon/phosphor/minus";
import plusIcon from "@destack/icon/phosphor/plus";
import { t } from "@destack/locale";
import { present } from "@destack/schema";
import { MOTION_PROPERTIES, MotionTrack, TimingFunction, type TrackPlace } from "@destack/style";
import * as style from "@destack/style";
import { color, radius, size, space, stroke } from "@destack/theme/tokens.stylex";
import { createEffect, createSignal, For, type JSX, Show, useLocale } from "@destack/view";
import { Field, FieldLabel } from "../field/field.tsx";
import { Icon } from "../icon/index.ts";
import { NumberPart } from "../value-field/dimension.tsx";
import { jsonType, ValuePart, ValueSet } from "../value-field/group.tsx";
import { ValueField } from "../value-field/value-field.tsx";

/** How far an arrow moves a keyframe along its track, and with Shift held. */
const STEPS = { small: 0.01, large: 0.1 } as const;

/** The hundredths a keyframe's offset keeps. */
const PRECISION = 100;

/** The type of a keyframe's easing. */
const EASING_TYPE = jsonType(TimingFunction);

/** The styles of a keyframe track. */
const styles = style.create({
    track: {
        position: "relative",
        height: size[1],
        marginInline: `calc(${space[3]} / 2)`,
        borderRadius: radius.full,
        backgroundColor: color.muted,
        touchAction: "none",
    },
    place: (left: string) => ({ left }),
    keyframe: {
        position: "absolute",
        top: "50%",
        width: space[3],
        height: space[3],
        marginInlineStart: `calc(${space[3]} / -2)`,
        marginBlockStart: `calc(${space[3]} / -2)`,
        padding: 0,
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.foreground,
        rotate: "45deg",
        backgroundColor: color.background,
        cursor: "grab",
    },
    selected: {
        backgroundColor: color.foreground,
    },
});

/** The styles of a track's field. */
const fieldStyles = style.create({
    keyframe: {
        display: "flex",
        flexDirection: "column",
        gap: space[2],
    },
    row: {
        display: "flex",
        alignItems: "center",
        gap: space[1],
    },
});

/** The properties of a keyframe track. */
export interface KeyframeTrackProperties {
    /** What the track animates, which names its keyframes. */
    readonly label: string;
    /** Where each keyframe falls along the track's time, from 0 to 1. */
    readonly offsets: readonly number[];
    /** The index of the selected keyframe. */
    readonly selected: number;
    /** Whether it takes no input. */
    readonly disabled: boolean | undefined;
    /** Select a keyframe. */
    readonly onSelect: (index: number) => void;
    /** Move a keyframe to another offset, in hundredths from 0 to 1. */
    readonly onMove: (index: number, offset: number) => void;
    /** Remove a keyframe, as Delete or Backspace asks where the track has another. */
    readonly onRemove: (index: number) => void;
}

/** Render the keyframes of one track along its time, selected, dragged or moved by arrows. */
export function KeyframeTrack(properties: KeyframeTrackProperties): JSX.Element {
    // move a keyframe within the track's time, in hundredths
    const locale = useLocale();
    let track: HTMLElement | undefined;
    const move = (index: number, offset: number): void =>
        properties.onMove(
            index,
            Math.round(Math.min(Math.max(offset, 0), 1) * PRECISION) / PRECISION,
        );

    // read where a pointer drags a keyframe as a share of the track
    const drag = (index: number, event: PointerEvent): void => {
        const box = present(track, "a keyframe track").getBoundingClientRect();
        move(index, (event.clientX - box.left) / box.width);
    };

    // keep focus on the selected keyframe as moves reorder the keyframes beneath it
    createEffect(
        () => properties.selected,
        (index) => {
            const keyframes = present(track, "a keyframe track").querySelectorAll<HTMLElement>(
                "[data-slot=keyframe-track-keyframe]",
            );
            if (track?.contains(document.activeElement) === true) {
                keyframes[index]?.focus();
            }
        },
    );

    // move a keyframe by an arrow, further with Shift held, or remove it, leaving other keys
    const keys = (index: number, offset: number, event: KeyboardEvent): void => {
        const step = event.shiftKey ? STEPS.large : STEPS.small;

        // move it later
        if (event.key === "ArrowRight") {
            event.preventDefault();
            properties.onSelect(index);
            move(index, offset + step);
        }
        // move it earlier
        else if (event.key === "ArrowLeft") {
            event.preventDefault();
            properties.onSelect(index);
            move(index, offset - step);
        }
        // remove it, where the track has another
        else if (
            (event.key === "Delete" || event.key === "Backspace") &&
            properties.offsets.length > 1
        ) {
            event.preventDefault();
            properties.onRemove(index);
        }
    };

    return (
        <div
            role="group"
            aria-label={properties.label}
            data-slot="keyframe-track"
            ref={(element) => (track = element)}
            {...style.attrs(styles.track)}
        >
            <For each={properties.offsets} keyed={false}>
                {(offset, index) => (
                    <button
                        type="button"
                        data-slot="keyframe-track-keyframe"
                        aria-label={locale.render(
                            t.context(
                                "label",
                            )`Keyframe at ${locale.number(offset(), { style: "percent" })}`,
                        )}
                        aria-pressed={properties.selected === index ? "true" : "false"}
                        disabled={properties.disabled}
                        onClick={() => properties.onSelect(index)}
                        onPointerDown={(event) => {
                            properties.onSelect(index);
                            event.currentTarget.setPointerCapture(event.pointerId);
                        }}
                        onPointerMove={(event) => {
                            if (event.currentTarget.hasPointerCapture(event.pointerId)) {
                                drag(index, event);
                            }
                        }}
                        onKeyDown={(event) => keys(index, offset(), event)}
                        {...style.attrs(
                            styles.keyframe,
                            styles.place(`${String(offset() * 100)}%`),
                            properties.selected === index && styles.selected,
                        )}
                    />
                )}
            </For>
        </div>
    );
}

/** Render a track's field: its keyframes along its time, then the selected keyframe's values. */
export function TrackField(properties: {
    /** The track. */
    readonly value: MotionTrack;
    /** Whether it takes no input. */
    readonly disabled: boolean | undefined;
    /** Handle the track changing. */
    readonly onValueChange: (value: MotionTrack) => void;
    /** Remove the track. */
    readonly onRemove: () => void;
}): JSX.Element {
    // select a keyframe, the first at first, following it as changes reorder the keyframes
    const locale = useLocale();
    const [selected, setSelected] = createSignal(0);
    const title = (): string => locale.render(MOTION_PROPERTIES[properties.value.property].title);
    const keyframe = () => properties.value.keyframes[selected()];
    const isLast = (): boolean => selected() === properties.value.keyframes.length - 1;
    const place = (next: TrackPlace): void => {
        setSelected(next.index);
        properties.onValueChange(next.track);
    };
    const remove = (index: number): void => {
        setSelected(Math.max(index - 1, 0));
        properties.onValueChange(MotionTrack.remove(properties.value, index));
    };

    return (
        <ValueSet
            label={title()}
            actions={
                <>
                    <Button
                        variant="ghost"
                        size="sm"
                        isIconOnly
                        aria-label={locale.render(t.context("action")`Add keyframe`)}
                        disabled={properties.disabled}
                        onClick={() => place(MotionTrack.split(properties.value, selected()))}
                    >
                        <Icon icon={plusIcon} />
                    </Button>
                    <Button
                        variant="ghost"
                        size="sm"
                        isIconOnly
                        aria-label={locale.render(t.context("action")`Remove track`)}
                        disabled={properties.disabled}
                        onClick={() => properties.onRemove()}
                    >
                        <Icon icon={minusIcon} />
                    </Button>
                </>
            }
        >
            {/* The keyframes along the track's time */}
            <KeyframeTrack
                label={title()}
                offsets={properties.value.keyframes.map((each) => each.offset)}
                selected={selected()}
                disabled={properties.disabled}
                onSelect={setSelected}
                onMove={(index, offset) =>
                    place(MotionTrack.set(properties.value, index, { offset }))
                }
                onRemove={remove}
            />

            {/* The selected keyframe: where it falls, its value, and how it eases into the next */}
            <Show when={keyframe()}>
                {(current) => (
                    <div {...style.attrs(fieldStyles.keyframe)}>
                        <Field orientation="horizontal">
                            <FieldLabel>{locale.render(t.context("label")`Offset`)}</FieldLabel>
                            <div {...style.attrs(fieldStyles.row)}>
                                <NumberPart
                                    label={locale.render(t.context("label")`Offset`)}
                                    start={<Icon icon={positionIcon} />}
                                    min={0}
                                    max={1}
                                    step={0.01}
                                    format={{ style: "percent" }}
                                    disabled={properties.disabled}
                                    value={current().offset}
                                    onValueChange={(offset) =>
                                        place(
                                            MotionTrack.set(properties.value, selected(), {
                                                offset,
                                            }),
                                        )
                                    }
                                />
                                <Button
                                    variant="ghost"
                                    size="sm"
                                    isIconOnly
                                    aria-label={locale.render(t.context("action")`Remove keyframe`)}
                                    disabled={
                                        properties.disabled === true ||
                                        properties.value.keyframes.length === 1
                                    }
                                    onClick={() => remove(selected())}
                                >
                                    <Icon icon={minusIcon} />
                                </Button>
                            </div>
                        </Field>
                        <ValueField
                            label={locale.render(t.context("label")`Value`)}
                            orientation="horizontal"
                            type={jsonType(MOTION_PROPERTIES[properties.value.property].schema)}
                            value={current().value}
                            required
                            disabled={properties.disabled}
                            onValueChange={(value) =>
                                place(
                                    MotionTrack.set(properties.value, selected(), {
                                        value: present(value, "a keyframe's value"),
                                    }),
                                )
                            }
                        />
                        <Show when={!isLast()}>
                            <ValuePart
                                type={EASING_TYPE}
                                schema={TimingFunction}
                                label={t.context("label")`Easing`}
                                isLabelShown
                                value={current().easing}
                                disabled={properties.disabled}
                                onValueChange={(easing) =>
                                    place(MotionTrack.set(properties.value, selected(), { easing }))
                                }
                            />
                        </Show>
                    </div>
                )}
            </Show>
        </ValueSet>
    );
}
