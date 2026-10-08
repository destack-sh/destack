import * as style from "@destack/style";
import { color, radius, stroke } from "@destack/theme/tokens.stylex";
import { createControllableSignal, createSignal, type JSX, omit, Show } from "@destack/view";
import { Slider, SliderThumb } from "../slider/index.ts";

/** The step an arrow key moves a cover's position by: a twentieth of its height. */
const KEY_STEP = 0.05;

/** The step Page Up and Page Down move a cover's position by: a fifth of its height. */
const PAGE_STEP = 0.2;

/** The properties of a cover image, the native element's attributes included. */
export interface CoverImageProperties extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> {
    /** The image's address. */
    readonly source: string;
    /** The vertical point the banner centres on, from 0 at the top to 1 at the bottom, which makes it controlled. */
    readonly position?: number;
    /** The point the banner centres on at first while uncontrolled, the middle by default. */
    readonly defaultPosition?: number;
    /** Handle a new position while a person drags the image or moves it with the keyboard, which makes the cover repositionable. */
    readonly onPositionChange?: (position: number) => void;
    /** The accessible name of the repositioning handle, required when the cover is repositionable. */
    readonly label?: string;
    /** The StyleX styles applied after the cover's styles. */
    readonly xstyle?: style.Styles;
}

/** A drag of a cover's handle under way. */
interface CoverDrag {
    /** The height the pointer started at. */
    readonly y: number;
    /** The position the drag started from. */
    readonly position: number;
    /** The height of the handle, which a drag moves the position across from end to end. */
    readonly height: number;
}

/** The styles of a cover and its image. */
const styles = style.create({
    cover: {
        position: "relative",
        overflow: "hidden",
        width: "100%",
        aspectRatio: "4 / 1",
        borderRadius: radius[2],
        backgroundColor: color.muted,
    },
    image: {
        width: "100%",
        height: "100%",
        objectFit: "cover",
        userSelect: "none",
        pointerEvents: "none",
    },
    repositionable: {
        cursor: { default: "grab", ":active": "grabbing" },
        outlineStyle: { default: "none", ":has(:focus-visible)": "solid" },
        outlineWidth: stroke.ring,
        outlineColor: color.ring,
        outlineOffset: `calc(-1 * ${stroke.ring})`,
        touchAction: "none",
    },
    slider: {
        position: "absolute",
        inset: 0,
        width: "100%",
        height: "100%",
        opacity: 0,
        pointerEvents: "none",
    },
});

/** The point an image's banner crop centres on. */
const crops = style.create({
    center: (position: number) => ({
        objectPosition: `center ${String(Math.round(position * 1000) / 10)}%`,
    }),
});

/** Keep a position between the top and the bottom of the image. */
function clamp(position: number): number {
    return Math.min(1, Math.max(0, position));
}

/** Read the position a key moves a cover to, undefined for a key that moves nothing. */
function keyed(key: string, position: number): number | undefined {
    switch (key) {
        case "ArrowUp":
            return clamp(position - KEY_STEP);
        case "ArrowDown":
            return clamp(position + KEY_STEP);
        case "PageUp":
            return clamp(position - PAGE_STEP);
        case "PageDown":
            return clamp(position + PAGE_STEP);
        case "Home":
            return 0;
        case "End":
            return 1;
        default:
            return undefined;
    }
}

/** Render an image as a banner centred on its position, repositioned by dragging or by the keys of a vertical slider when its owner listens. */
export function CoverImage(properties: CoverImageProperties): JSX.Element {
    // follow the controlled position or the cover's own
    const rest = omit(
        properties,
        "source",
        "position",
        "defaultPosition",
        "onPositionChange",
        "label",
        "xstyle",
        "style",
    );
    const [position, setPosition] = createControllableSignal({
        isControlled: () => properties.position !== undefined,
        value: () => properties.position ?? 0.5,
        defaultValue: properties.defaultPosition ?? 0.5,
        onChange: (next) => properties.onPositionChange?.(next),
    });
    const isRepositionable = (): boolean => properties.onPositionChange !== undefined;
    const percent = (): number => Math.round(position() * 100);

    // keep the point and position a drag started from, and the height it moves across
    const [drag, setDrag] = createSignal<CoverDrag | undefined>(undefined);

    return (
        <div
            data-slot="cover-image"
            {...rest}
            onPointerDown={(event) => {
                // start a drag of a repositionable cover from the pointer and the current position
                if (isRepositionable()) {
                    event.currentTarget.setPointerCapture(event.pointerId);
                    setDrag({
                        y: event.clientY,
                        position: position(),
                        height: event.currentTarget.clientHeight,
                    });
                }
            }}
            onPointerMove={(event) => {
                // move the position against the pointer, a full height of drag moving it end to end
                const started = drag();
                if (started !== undefined) {
                    setPosition(dragged(started, event.clientY));
                }
            }}
            onPointerUp={() => setDrag(undefined)}
            onPointerCancel={() => setDrag(undefined)}
            {...style.attributes(
                [styles.cover, isRepositionable() && styles.repositionable, properties.xstyle],
                properties.style,
            )}
        >
            <img
                src={properties.source}
                alt=""
                {...style.attributes([styles.image, crops.center(position())])}
            />
            <Show when={isRepositionable()}>
                <Slider
                    orientation="vertical"
                    min={0}
                    max={100}
                    value={[percent()]}
                    onValueChange={([value]) => {
                        // take a value the slider reports as a position
                        if (value !== undefined) {
                            setPosition(value / 100);
                        }
                    }}
                    xstyle={styles.slider}
                >
                    <SliderThumb
                        aria-label={properties.label}
                        aria-valuetext={`${String(percent())}%`}
                        data-slot="cover-image-handle"
                        onKeyDown={(event) => {
                            // nudge the position a step or a page, or move it to the top or the bottom
                            const next = keyed(event.key, position());
                            if (next !== undefined) {
                                event.preventDefault();
                                setPosition(next);
                            }
                        }}
                    />
                </Slider>
            </Show>
        </div>
    );
}

/** Read the position a drag moves a cover to as the pointer reaches a height. */
function dragged(drag: CoverDrag, y: number): number {
    return clamp(drag.position - (y - drag.y) / drag.height);
}
