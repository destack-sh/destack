import * as style from "@destack/style";
import { color, radius, size, space, stroke } from "@destack/theme/tokens.stylex";
import {
    type Accessor,
    createContext,
    createSignal,
    createUniqueId,
    type JSX,
    merge,
    omit,
    onCleanup,
    type Setter,
    Show,
    useContext,
    useLocale,
} from "@destack/view";

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
    share: (share: number) => ({
        flexGrow: share,
    }),
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
    /** Whether a drag below half the smallest share collapses the panel. */
    readonly collapsible: boolean;
    /** The share of the collapsed panel. */
    readonly collapsedSize: number;
}

/** The panels of a resizable group in order and the shares a handle moves between neighbours. */
export class ResizableControl {
    /** The direction the panels line up in. */
    readonly direction: () => "horizontal" | "vertical";
    /** The panels in document order. */
    readonly panels: Accessor<readonly ResizablePanelState[]>;
    /** The properties of the group, read for its handlers and saved layout. */
    readonly #properties: ResizablePanelGroupProperties;
    /** The shares the group saved, by panel index. */
    readonly #saved: readonly number[];
    /** Replace the panels. */
    readonly #setPanels: Setter<readonly ResizablePanelState[]>;
    /** The number of handles joined so far, which numbers the next handle. */
    #handles: number;
    /** The number of panels joined so far, which numbers the next panel. */
    #count: number;

    /** Create a group without panels, restoring the layout it saved under its id. */
    constructor(
        direction: () => "horizontal" | "vertical",
        properties: ResizablePanelGroupProperties,
    ) {
        // start without panels or handles on the saved layout
        const [panels, setPanels] = createSignal<readonly ResizablePanelState[]>([], {
            ownedWrite: true,
        });
        this.direction = direction;
        this.panels = panels;
        this.#properties = properties;
        this.#saved = properties.autoSaveId === undefined ? [] : loadLayout(properties.autoSaveId);
        this.#setPanels = setPanels;
        this.#handles = 0;
        this.#count = 0;
    }

    /** Add a panel until it unmounts, answering the share the group saved for it. */
    register(panel: ResizablePanelState): number | undefined {
        // add the panel and number it
        this.#setPanels((panels) => [...panels, panel]);
        onCleanup(() => this.#setPanels((panels) => panels.filter((entry) => entry !== panel)));
        this.#count += 1;

        return this.#saved[this.#count - 1];
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

        // split what the panels with shares leave evenly
        const panels = this.panels();
        const taken = panels.reduce((sum, entry) => sum + (entry.size() ?? 0), 0);
        const open = panels.filter((entry) => entry.size() === undefined).length;

        return (100 - taken) / open;
    }

    /** Read every panel's share in order. */
    layout(): number[] {
        return this.panels().map((panel) => this.sizeOf(panel));
    }

    /** Set every panel's share in order. */
    apply(layout: readonly number[]): void {
        this.panels().forEach((panel, index) => panel.resize(layout[index]));
    }

    /** Move the boundary after a handle's panel by a share, within both neighbours' limits, collapsing a collapsible neighbour dragged below half its smallest share. */
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
        const step =
            collapseStep(before, sizeBefore, delta) ??
            collapseStep(after, sizeAfter, -delta, -1) ??
            Math.min(Math.max(delta, lowest), highest);
        before.resize(sizeBefore + step);
        after.resize(sizeAfter - step);

        // report the layout the move makes
        this.report(
            this.panels().map((panel) =>
                panel === before
                    ? sizeBefore + step
                    : panel === after
                      ? sizeAfter - step
                      : this.sizeOf(panel),
            ),
        );
    }

    /** Collapse a collapsible panel, or restore it to its smallest share, taking the share from its neighbour. */
    toggle(handle: number): void {
        // move a collapsed panel to its smallest share and any other to its collapsed share
        const before = this.panels()[handle];
        if (before === undefined || !before.collapsible) {
            return;
        }
        const share = this.sizeOf(before);
        this.move(
            handle,
            share <= before.collapsedSize ? before.minimum - share : before.collapsedSize - share,
        );
    }

    /** Tell the layout handler of a layout and save it under the group's id. */
    report(layout: readonly number[]): void {
        this.#properties.onLayout?.(layout);
        if (this.#properties.autoSaveId !== undefined) {
            saveLayout(this.#properties.autoSaveId, layout);
        }
    }
}

/** The step that collapses a collapsible panel dragged below half its smallest share, else undefined. */
function collapseStep(
    panel: ResizablePanelState,
    share: number,
    delta: number,
    sign = 1,
): number | undefined {
    const isCollapsing =
        panel.collapsible && share + delta < panel.minimum / 2 && share > panel.collapsedSize;

    return isCollapsing ? sign * (panel.collapsedSize - share) : undefined;
}

/** The storage key prefix of saved layouts. */
const LAYOUT_KEY = "destack-resizable:";

/** Read the layout a group saved under its id, none when storage refuses. */
function loadLayout(id: string): readonly number[] {
    try {
        const saved: unknown = JSON.parse(localStorage.getItem(`${LAYOUT_KEY}${id}`) ?? "[]");

        return Array.isArray(saved) && saved.every((entry) => typeof entry === "number")
            ? saved
            : [];
    } catch {
        return [];
    }
}

/** Save a group's layout under its id, skipping storage that refuses. */
function saveLayout(id: string, layout: readonly number[]): void {
    try {
        localStorage.setItem(`${LAYOUT_KEY}${id}`, JSON.stringify(layout));
    } catch {
        // keep the layout in memory alone where storage refuses
    }
}

/** The properties of a panel group, the native element's attributes included. */
export interface ResizablePanelGroupProperties extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class"
> {
    /** The direction the panels line up in, horizontal by default. */
    readonly direction?: "horizontal" | "vertical";
    /** Handle the panels' shares changing, in order. */
    readonly onLayout?: (layout: readonly number[]) => void;
    /** The id the group saves its layout under in the browser, restoring it on the next visit. */
    readonly autoSaveId?: string;
    /** The StyleX styles applied after the group's styles. */
    readonly xstyle?: style.Styles;
}

/** The properties of a panel, the native element's attributes included. */
export interface ResizablePanelProperties extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "id"
> {
    /** The panel's share of the group at first in percent, an even part by default. */
    readonly defaultSize?: number;
    /** The smallest share in percent, 0 by default. */
    readonly minSize?: number;
    /** The largest share in percent, 100 by default. */
    readonly maxSize?: number;
    /** Whether a drag below half the smallest share, or Enter on its handle, collapses the panel. */
    readonly collapsible?: boolean;
    /** The share of the collapsed panel in percent, 0 by default. */
    readonly collapsedSize?: number;
    /** The StyleX styles applied after the panel's styles. */
    readonly xstyle?: style.Styles;
}

/** The properties of a handle between two panels, the native element's attributes included. */
export interface ResizableHandleProperties extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "onKeyDown" | "onPointerDown" | "onPointerMove" | "onPointerUp"
> {
    /** Whether the handle shows a grip to aim at. */
    readonly withHandle?: boolean;
    /** The StyleX styles applied after the handle's styles. */
    readonly xstyle?: style.Styles;
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
    const control = new ResizableControl(() => group.direction, properties);
    const rest = omit(group, "direction", "onLayout", "autoSaveId", "xstyle", "style");

    return (
        <ResizableContext value={control}>
            <div
                data-slot="resizable-panel-group"
                data-panel-group-direction={group.direction}
                {...rest}
                {...style.attributes(
                    [styles.group, directions[group.direction], group.xstyle],
                    group.style,
                )}
            />
        </ResizableContext>
    );
}

/** Render a panel that takes its share of the group. */
export function ResizablePanel(properties: ResizablePanelProperties): JSX.Element {
    // join the group with the panel's share and limits
    const control = useResizable();
    const [share, resize] = createSignal(properties.defaultSize, { ownedWrite: true });
    const panel: ResizablePanelState = {
        id: createUniqueId(),
        size: share,
        resize,
        minimum: properties.minSize ?? 0,
        maximum: properties.maxSize ?? 100,
        collapsible: properties.collapsible === true,
        collapsedSize: properties.collapsedSize ?? 0,
    };
    const saved = control.register(panel);
    if (saved !== undefined) {
        resize(saved);
    }
    const rest = omit(
        properties,
        "defaultSize",
        "minSize",
        "maxSize",
        "collapsible",
        "collapsedSize",
        "xstyle",
        "style",
    );

    return (
        <div
            id={panel.id}
            data-slot="resizable-panel"
            data-state={
                panel.collapsible
                    ? control.sizeOf(panel) <= panel.collapsedSize
                        ? "collapsed"
                        : "expanded"
                    : undefined
            }
            {...rest}
            {...style.attributes(
                [styles.panel, styles.share(control.sizeOf(panel)), properties.xstyle],
                properties.style,
            )}
        />
    );
}

/** Render the window splitter between two panels, which pointers drag and arrow keys, Home and End move. */
export function ResizableHandle(properties: ResizableHandleProperties): JSX.Element {
    // read the group, this handle's place and the panel before it
    const control = useResizable();
    const locale = useLocale();
    const handle = control.nextHandle();
    const rest = omit(properties, "withHandle", "xstyle", "style");
    const before = (): ResizablePanelState | undefined => control.panels()[handle];
    let start:
        | { readonly position: number; readonly extent: number; readonly layout: number[] }
        | undefined;

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
                // collapse or restore the panel before on Enter
                if (event.key === "Enter") {
                    event.preventDefault();
                    control.toggle(handle);
                    return;
                }

                // move by a step, or to the panel's limits on Home and End
                const delta = keyDelta(event.key, control, before(), locale.direction);
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
                    layout: control.layout(),
                };
            }}
            onPointerMove={(event) => {
                // move by the pointer's share of the group since it pressed
                if (start === undefined || start.extent === 0) {
                    return;
                }
                const position =
                    control.direction() === "horizontal" ? event.clientX : event.clientY;
                control.apply(start.layout);
                control.move(handle, ((position - start.position) / start.extent) * 100);
            }}
            onPointerUp={() => (start = undefined)}
            {...style.attributes(
                [styles.handle, handles[control.direction()], properties.xstyle],
                properties.style,
            )}
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
