import { media } from "@destack/style/media.stylex";
import * as style from "@destack/style";
import { color, radius, shadow, stroke } from "@destack/theme/tokens.stylex";
import { type JSX, untrack, useEnvironment, useLocale } from "@destack/view";
import { Frame, type FrameBounds } from "../interaction/index.ts";
import { FrameSides } from "../floating-panel/index.ts";
import { LEAST, type PaneGroupControl, usePaneGroup } from "./control.ts";
import { PaneLayout, type PaneBox, type PaneFloat } from "./layout.ts";

/** The layers each float takes: its frame, its panes, their contents, their lines and its sides. */
export const LAYERS = 5;

/** The styles of a float's frame. */
const styles = style.create({
    frame: {
        position: "absolute",
        borderRadius: radius[3],
        backgroundColor: color.background,
        boxShadow: `0 0 0 ${stroke.border} ${color.border}, ${shadow.overlay}`,
        outlineWidth: { default: 0, [media.forcedColors]: stroke.border },
        outlineStyle: "solid",
        outlineColor: "CanvasText",
    },
    sides: {
        position: "absolute",
        pointerEvents: "none",
    },
    place: (start: string, top: string, width: string, height: string) => ({
        insetInlineStart: start,
        top,
        width,
        height,
    }),
    layer: (layer: number) => ({ zIndex: layer }),
});

/** Create the frame of a float, in the pixels of its group. */
export function frameOf(group: PaneGroupControl, float: () => PaneFloat | undefined): Frame {
    // convert between the shares of the group and its pixels
    const locale = useLocale();
    const room = () => {
        const bounds = group.element()?.getBoundingClientRect();

        return { width: bounds?.width ?? 0, height: bounds?.height ?? 0 };
    };

    return new Frame(
        {
            bounds: () => {
                const box = untrack(float)?.bounds;
                const { width, height } = room();

                return box === undefined
                    ? { start: 0, top: 0, width, height }
                    : {
                          start: box.start * width,
                          top: box.top * height,
                          width: box.width * width,
                          height: box.height * height,
                      };
            },
            room,
            least: () => {
                const root = untrack(float)?.root;

                return root === undefined
                    ? { width: LEAST.horizontal, height: LEAST.vertical }
                    : {
                          width: group.leastOf(root, "horizontal"),
                          height: group.leastOf(root, "vertical"),
                      };
            },
            onChange: (bounds) => {
                const id = untrack(float)?.id;
                if (id !== undefined) {
                    group.change((layout) => PaneLayout.place(layout, id, shareOf(bounds, room())));
                }
            },
            direction: () => locale.direction,
            canMove: ({ target }) =>
                untrack(float) !== undefined &&
                !(
                    target instanceof Element &&
                    target.closest("[role=tab], button, a[href], input") !== null
                ),
            isDisabled: () => group.isLocked(),
        },
        useEnvironment().clock,
    );
}

/** Render the frame of a float, which its sides and corners resize. */
export function PaneFrame(properties: {
    /** The float. */
    readonly float: PaneFloat;
    /** How high it floats. */
    readonly height: number;
}): JSX.Element {
    // draw it under its panes, and its sides over them
    const group = usePaneGroup();
    const frame = frameOf(group, () => properties.float);
    const place = (): style.Styles => {
        const { start, top, width, height } = properties.float.bounds;

        return styles.place(percent(start), percent(top), percent(width), percent(height));
    };

    return (
        <>
            <div
                data-slot="pane-frame"
                aria-hidden="true"
                {...style.attributes([
                    styles.frame,
                    place(),
                    styles.layer(properties.height * LAYERS),
                ])}
            />
            <div
                data-slot="pane-frame-sides"
                hidden={group.isLocked()}
                {...style.attributes([
                    styles.sides,
                    place(),
                    styles.layer(properties.height * LAYERS + LAYERS - 1),
                ])}
            >
                <FrameSides frame={frame} />
            </div>
        </>
    );
}

/** Write a share as a percentage. */
function percent(share: number): string {
    return `${String(share * 100)}%`;
}

/** Read bounds in pixels as shares of a room. */
function shareOf(
    bounds: FrameBounds,
    room: { readonly width: number; readonly height: number },
): PaneBox {
    return {
        start: bounds.start / room.width,
        top: bounds.top / room.height,
        width: bounds.width / room.width,
        height: bounds.height / room.height,
    };
}
