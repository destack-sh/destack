import * as style from "@destack/style";
import { color, radius, size, space, stroke } from "@destack/theme/tokens.stylex";
import { useLocale } from "@destack/locale/solid";
import type { JSX } from "@solidjs/web";
import {
    createContext,
    createSignal,
    createUniqueId,
    merge,
    omit,
    onCleanup,
    Show,
    useContext,
    type Accessor,
    type Setter,
} from "solid-js";
import { directionOf } from "../focus/index.ts";

/** The share a keyboard step moves a handle by, in percent of the group. */
const KEYBOARD_STEP = 10;

/** The direction of a panel group that sets none. */
const DEFAULTS: Required<Pick<ResizablePanelGroupProperties, "direction">> = {
    direction: "horizontal",
};

/** The panel group of the nearest group, null outside one. */
const ResizableContext = createContext<ResizableControl | null>(null);

/** The styles of a panel group, its panels and handles. */
const styles = style.create({
    group: {
        display: "flex",
        width: "100%",
        height: "100%",
    },
    panel: {
        overflow: "hidden",
        flexBasis: 0,
        flexShrink: 1,
    },
    handle: {
        position: "relative",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        flexShrink: 0,
        backgroundColor: color.border,
        touchAction: "none",
        outlineStyle: { default: "none", ":focus-visible": "solid" },
        outlineWidth: stroke.ring,
        outlineColor: `color-mix(in oklab, ${color.ring} 50%, transparent)`,
    },
    grip: {
        zIndex: 1,
        borderRadius: radius[2],
        borderStyle: "solid",
        borderWidth: stroke.border,
        borderColor: color.border,
        backgroundColor: color.border,
    },
});

/** The layout of a group, its handles and grips in each direction. */
const directions = style.create({
    horizontal: { flexDirection: "row" },
    vertical: { flexDirection: "column" },
});

/** The thin side and the cursor of a handle in each direction. */
const handles = style.create({
    horizontal: { width: stroke.border, cursor: "col-resize" },
    vertical: { height: stroke.border, cursor: "row-resize" },
});

/** The shape of a handle's grip in each direction. */
const grips = style.create({
    horizontal: { width: space[2], height: size[1] },
    vertical: { width: size[1], height: space[2] },
});

/** A panel of a group, with its share of the group and its limits in percent. */
export interface ResizablePanelState {
    /** The id of the panel element. */
    readonly id: string;
    /** The panel's share when set, undefined while it takes an even part of what the others leave. */
    readonly size: Accessor<number | undefined>;
    /** Set the panel's share. */
    readonly resize: Setter<number | undefined>;
    /** The smallest share. */
    readonly minimum: number;
    /** The largest share. */
    readonly maximum: number;
}

/** The panels of a resizable group in order and the shares a handle moves between neighbours. */
export class ResizableControl {
    /** The direction the panels line up in. */
    readonly direction: () => "horizontal" | "vertical";
    /** The panels in document order. */
    readonly panels: Accessor<readonly ResizablePanelState[]>;
    /** Replace the panels. */
    readonly #setPanels: Setter<readonly ResizablePanelState[]>;
    /** The number of handles joined so far, which numbers the next handle. */
    #handles: number;

    /** Create a group without panels. */
    constructor(direction: () => "horizontal" | "vertical") {
        // start without panels or handles
        const [panels, setPanels] = createSignal<readonly ResizablePanelState[]>([], {
            ownedWrite: true,
        });
        this.direction = direction;
        this.panels = panels;
        this.#setPanels = setPanels;
        this.#handles = 0;
    }

    /** Add a panel until it unmounts. */
    register(panel: ResizablePanelState): void {
        this.#setPanels((panels) => [...panels, panel]);
        onCleanup(() => this.#setPanels((panels) => panels.filter((entry) => entry !== panel)));
    }

    /** Number the next handle, the one after that many panels. */
    nextHandle(): number {
        this.#handles += 1;

        return this.#handles - 1;
    }

    /** Read a panel's share: its own, else an even part of what the panels with shares leave. */
    sizeOf(panel: ResizablePanelState): number {
        // take the panel's own share when set
        const own = panel.size();
        if (own !== undefined) {
            return own;
        }
        const panels = this.panels();
        const taken = panels.reduce((sum, entry) => sum + (entry.size() ?? 0), 0);
        const open = panels.filter((entry) => entry.size() === undefined).length;

        return (100 - taken) / open;
    }

    /** Move the boundary after a handle's panel by a share, within both neighbours' limits. */
    move(handle: number, delta: number): void {
        // read the neighbours of the handle
        const before = this.panels()[handle];
        const after = this.panels()[handle + 1];
        if (before === undefined || after === undefined) {
            return;
        }

        // keep both shares within their limits and their sum
        const sizeBefore = this.sizeOf(before);
        const sizeAfter = this.sizeOf(after);
        const lowest = Math.max(before.minimum - sizeBefore, sizeAfter - after.maximum);
        const highest = Math.min(before.maximum - sizeBefore, sizeAfter - after.minimum);
        const step = Math.min(Math.max(delta, lowest), highest);
        before.resize(sizeBefore + step);
        after.resize(sizeAfter - step);
    }
}

/** The properties of a panel group, the native element's attributes included. */
export interface ResizablePanelGroupProperties extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "style"
> {
    /** The direction the panels line up in, horizontal by default. */
    readonly direction?: "horizontal" | "vertical";
    /** The StyleX styles applied after the group's styles. */
    readonly style?: style.Styles;
}

/** The properties of a panel, the native element's attributes included. */
export interface ResizablePanelProperties extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "style" | "id"
> {
    /** The panel's share of the group at first in percent, an even part by default. */
    readonly defaultSize?: number;
    /** The smallest share in percent, 0 by default. */
    readonly minSize?: number;
    /** The largest share in percent, 100 by default. */
    readonly maxSize?: number;
    /** The StyleX styles applied after the panel's styles. */
    readonly style?: style.Styles;
}

/** The properties of a handle between two panels, the native element's attributes included. */
export interface ResizableHandleProperties extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "style" | "onKeyDown" | "onPointerDown" | "onPointerMove" | "onPointerUp"
> {
    /** Whether the handle shows a grip to aim at. */
    readonly withHandle?: boolean;
    /** The StyleX styles applied after the handle's styles. */
    readonly style?: style.Styles;
}

/** Read the panel group around an element, refusing elements outside one. */
function useResizable(): ResizableControl {
    const control = useContext(ResizableContext);
    if (control === null) {
        throw new TypeError("resizable panels and handles need a panel group around them");
    }

    return control;
}

/** Render a row or column of panels whose shares the handles between them change. */
export function ResizablePanelGroup(properties: ResizablePanelGroupProperties): JSX.Element {
    // share one group with its panels and handles
    const group = merge(DEFAULTS, properties);
    const control = new ResizableControl(() => group.direction);
    const rest = omit(group, "direction", "style");

    return (
        <ResizableContext value={control}>
            <div
                data-slot="resizable-panel-group"
                data-panel-group-direction={group.direction}
                {...rest}
                {...style.attrs(styles.group, directions[group.direction], group.style)}
            />
        </ResizableContext>
    );
}

/** Render a panel that takes its share of the group. */
export function ResizablePanel(properties: ResizablePanelProperties): JSX.Element {
    // join the group with the panel's share and limits
    const control = useResizable();
    const [share, resize] = createSignal(properties.defaultSize);
    const panel: ResizablePanelState = {
        id: createUniqueId(),
        size: share,
        resize,
        minimum: properties.minSize ?? 0,
        maximum: properties.maxSize ?? 100,
    };
    control.register(panel);
    const rest = omit(properties, "defaultSize", "minSize", "maxSize", "style");

    return (
        <div
            id={panel.id}
            data-slot="resizable-panel"
            {...rest}
            {...style.attrs(styles.panel, properties.style)}
            style={{ "flex-grow": String(control.sizeOf(panel)) }}
        />
    );
}

/** Render the window splitter between two panels, which pointers drag and arrow keys, Home and End move. */
export function ResizableHandle(properties: ResizableHandleProperties): JSX.Element {
    // read the group, this handle's place and the panel before it
    const control = useResizable();
    const locale = useLocale();
    const handle = control.nextHandle();
    const rest = omit(properties, "withHandle", "style");
    const before = (): ResizablePanelState | undefined => control.panels()[handle];
    let start: { readonly position: number; readonly extent: number } | undefined;

    return (
        <div
            role="separator"
            tabindex={0}
            aria-orientation={control.direction() === "horizontal" ? "vertical" : "horizontal"}
            aria-controls={before()?.id}
            aria-valuenow={Math.round(sizeOrZero(control, before()))}
            aria-valuemin={before()?.minimum ?? 0}
            aria-valuemax={before()?.maximum ?? 100}
            data-slot="resizable-handle"
            {...rest}
            onKeyDown={(event) => {
                // move by a step, or to the panel's limits on Home and End
                const delta = keyDelta(event.key, control, before(), directionOf(locale.tag));
                if (delta !== undefined) {
                    event.preventDefault();
                    control.move(handle, delta);
                }
            }}
            onPointerDown={(event) => {
                // follow the pointer from where it pressed, measured against the group
                const group = event.currentTarget.parentElement?.getBoundingClientRect();
                const isHorizontal = control.direction() === "horizontal";
                event.currentTarget.setPointerCapture(event.pointerId);
                start = {
                    position: isHorizontal ? event.clientX : event.clientY,
                    extent: (isHorizontal ? group?.width : group?.height) ?? 0,
                };
            }}
            onPointerMove={(event) => {
                // move by the pointer's travel as a share of the group
                if (start === undefined || start.extent === 0) {
                    return;
                }
                const position =
                    control.direction() === "horizontal" ? event.clientX : event.clientY;
                control.move(handle, ((position - start.position) / start.extent) * 100);
                start = { ...start, position };
            }}
            onPointerUp={() => (start = undefined)}
            {...style.attrs(styles.handle, handles[control.direction()], properties.style)}
        >
            <Show when={properties.withHandle === true}>
                <div aria-hidden="true" {...style.attrs(styles.grip, grips[control.direction()])} />
            </Show>
        </div>
    );
}

/** Read a panel's share, or zero before the panel mounts. */
function sizeOrZero(control: ResizableControl, panel: ResizablePanelState | undefined): number {
    return panel === undefined ? 0 : control.sizeOf(panel);
}

/** Read the share a key moves a handle by, mirroring left and right in right-to-left text. */
function keyDelta(
    key: string,
    control: ResizableControl,
    panel: ResizablePanelState | undefined,
    direction: "ltr" | "rtl",
): number | undefined {
    // a step along the group's axis
    const isHorizontal = control.direction() === "horizontal";
    const grow = isHorizontal ? (direction === "rtl" ? "ArrowLeft" : "ArrowRight") : "ArrowDown";
    const shrink = isHorizontal ? (direction === "rtl" ? "ArrowRight" : "ArrowLeft") : "ArrowUp";
    if (key === grow) {
        return KEYBOARD_STEP;
    } else if (key === shrink) {
        return -KEYBOARD_STEP;
    }
    // the panel's limits
    else if (panel !== undefined && key === "Home") {
        return panel.minimum - control.sizeOf(panel);
    } else if (panel !== undefined && key === "End") {
        return panel.maximum - control.sizeOf(panel);
    } else {
        return undefined;
    }
}
