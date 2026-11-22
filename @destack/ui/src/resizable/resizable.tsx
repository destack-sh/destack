import { state } from "@destack/theme/state";
import type { Axis } from "../position/index.ts";
import { schema } from "@destack/schema";
import { TelemetryScope } from "@destack/telemetry";
import { StoredItem } from "@destack/view/primitives/storage";
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
    onSettled,
    type Setter,
    Show,
    useContext,
    useEnvironment,
    useLocale,
} from "@destack/view";
import { createElementSize } from "@destack/view/primitives/resize-observer";
import { Move } from "../interaction/index.ts";
import { mergeProperties } from "../part/index.ts";

/** The storage key prefix of saved layouts. */
const LAYOUT_KEY = "destack-resizable:";

/** The shape of a saved layout: each panel's share. */
const SavedLayout = schema.array(schema.number());

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

/** A size in a group: a share in percent, or a length in pixels such as `"320px"`. */
export type ResizableSize = number | `${number}px`;

/** A panel of a group, with its share of the group in percent and its limits. */
export interface ResizablePanelState {
    /** The id of the panel element. */
    readonly id: string;
    /** The panel's share when set. */
    readonly size: Accessor<number | undefined>;
    /** Set the panel's share. */
    readonly resize: Setter<number | undefined>;
    /** The smallest size. */
    readonly minimum: ResizableSize;
    /** The largest size. */
    readonly maximum: ResizableSize;
    /** Whether a drag below half the smallest share collapses the panel. */
    readonly collapsible: boolean;
    /** The size of the collapsed panel. */
    readonly collapsedSize: ResizableSize;
}

/** The panels of a resizable group in order and the shares a handle moves between neighbours. */
export class ResizableControl {
    /** The direction the panels line up in. */
    readonly direction: () => Axis;
    /** The panels in document order. */
    readonly panels: Accessor<readonly ResizablePanelState[]>;
    /** The properties of the group, read for its handlers and saved layout. */
    readonly #properties: ResizablePanelGroupProperties;
    /** The layout the group saves under its id, absent without one. */
    readonly #stored: StoredItem | undefined;
    /** The package's telemetry, reporting garbled saved layouts. */
    readonly #telemetry: TelemetryScope;
    /** Replace the panels. */
    readonly #setPanels: Setter<readonly ResizablePanelState[]>;
    /** Replace the group element. */
    readonly #setElement: Setter<HTMLElement | undefined>;
    /** The group element's measured size. */
    readonly #size: { readonly width: number | null; readonly height: number | null };
    /** The number of handles joined so far, which numbers the next handle. */
    #handles: number;

    /** Create a group without panels, restoring the layout it saved under its id. */
    constructor(direction: () => Axis, properties: ResizablePanelGroupProperties) {
        // start without panels or handles on the saved layout
        const [panels, setPanels] = createSignal<readonly ResizablePanelState[]>([], {
            ownedWrite: true,
        });
        this.direction = direction;
        this.panels = panels;
        this.#properties = properties;
        const environment = useEnvironment();
        this.#telemetry = TelemetryScope.of(import.meta.destack.package, environment.telemetry);
        this.#stored =
            properties.autoSaveId === undefined
                ? undefined
                : new StoredItem(
                      `${LAYOUT_KEY}${properties.autoSaveId}`,
                      environment,
                      this.#telemetry,
                  );
        this.#setPanels = setPanels;
        this.#handles = 0;
        const [element, setElement] = createSignal<HTMLElement>();
        this.#setElement = setElement;
        this.#size = createElementSize(element);
    }

    /** Set the group element, which pixel sizes measure against. */
    setElement(element: HTMLElement): void {
        this.#setElement(element);
    }

    /** Report whether a size reads as a share now: a share always, pixels once measured. */
    isShareable(value: ResizableSize): boolean {
        return typeof value === "number" || (this.#length() ?? 0) > 0;
    }

    /** Read a size as a share of the group in percent, pixels by the group's measured length. */
    share(value: ResizableSize): number {
        // take a share as it is
        if (typeof value === "number") {
            return value;
        }

        // divide a length by the group's, refusing a group not yet measured
        const length = this.#length();
        if (length === null || length === 0) {
            throw new RangeError(
                `resizable size '${value}' is invalid: the group is not measured yet`,
            );
        }

        return (Number.parseFloat(value) / length) * 100;
    }

    /** Read the group's length along its direction, null before it is measured. */
    #length(): number | null {
        return this.direction() === "horizontal" ? this.#size.width : this.#size.height;
    }

    /** Add a panel until it unmounts. */
    register(panel: ResizablePanelState): void {
        // add the panel and number it
        this.#setPanels((panels) => [...panels, panel]);
        onCleanup(() => this.#setPanels((panels) => panels.filter((entry) => entry !== panel)));
    }

    /** Set the panels' shares to the saved layout, which the server never reads. */
    restore(): void {
        const saved = this.#load();
        if (saved.length > 0) {
            this.apply(saved);
        }
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

    /** Move the boundary after a handle's panel by a share, within both neighbours' limits. */
    move(handle: number, delta: number): void {
        // read the neighbours of the handle
        const before = this.panels()[handle];
        const after = this.panels()[handle + 1];
        if (before === undefined || after === undefined) {
            return;
        }

        // clamp both shares to their limits and their sum
        const sizeBefore = this.sizeOf(before);
        const sizeAfter = this.sizeOf(after);
        const limits = (panel: ResizablePanelState) => ({
            minimum: this.share(panel.minimum),
            maximum: this.share(panel.maximum),
            collapsed: this.share(panel.collapsedSize),
        });
        const [first, second] = [limits(before), limits(after)];
        const lowest = Math.max(first.minimum - sizeBefore, sizeAfter - second.maximum);
        const highest = Math.min(first.maximum - sizeBefore, sizeAfter - second.minimum);
        const step =
            collapseStep(before.collapsible, first, sizeBefore, delta) ??
            collapseStep(after.collapsible, second, sizeAfter, -delta, -1) ??
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

    /** Collapse a collapsible panel, or restore it to its smallest share. */
    toggle(handle: number): void {
        // move a collapsed panel to its smallest share and any other to its collapsed share
        const before = this.panels()[handle];
        if (before === undefined || !before.collapsible) {
            return;
        }
        const share = this.sizeOf(before);
        const collapsed = this.share(before.collapsedSize);
        this.move(
            handle,
            share <= collapsed ? this.share(before.minimum) - share : collapsed - share,
        );
    }

    /** Tell the layout handler of a layout and save it under the group's id. */
    report(layout: readonly number[]): void {
        this.#properties.onLayout?.(layout);
        this.#stored?.write(JSON.stringify(layout));
    }

    /** Read the saved layout, none where storage refuses or has something else. */
    #load(): readonly number[] {
        // read the saved shares, reporting a garbled layout
        const text = this.#stored?.read();
        try {
            return text === undefined ? [] : SavedLayout.parse(JSON.parse(text));
        } catch (error) {
            this.#telemetry.captureException(error, { level: "warn" });

            return [];
        }
    }
}

/** The step that collapses a collapsible panel dragged below half its smallest share. */
function collapseStep(
    isCollapsible: boolean,
    limits: { readonly minimum: number; readonly collapsed: number },
    share: number,
    delta: number,
    sign = 1,
): number | undefined {
    const isCollapsing =
        isCollapsible && share + delta < limits.minimum / 2 && share > limits.collapsed;

    return isCollapsing ? sign * (limits.collapsed - share) : undefined;
}

/** The properties of a panel group, the native element's attributes included. */
export interface ResizablePanelGroupProperties extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class"
> {
    /** The direction the panels line up in, horizontal by default. */
    readonly direction?: Axis;
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
    /** The smallest size, a share in percent or pixels such as `"320px"`, 0 by default. */
    readonly minSize?: ResizableSize;
    /** The largest size, a share in percent or pixels, 100 by default. */
    readonly maxSize?: ResizableSize;
    /** Whether a drag below half the smallest share or Enter on its handle collapses the panel. */
    readonly collapsible?: boolean;
    /** The size of the collapsed panel, a share in percent or pixels, 0 by default. */
    readonly collapsedSize?: ResizableSize;
    /** The StyleX styles applied after the panel's styles. */
    readonly xstyle?: style.Styles;
}

/** The properties of a handle between two panels, the native element's attributes included. */
export interface ResizableHandleProperties extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "onKeyDown" | "onPointerDown" | "onPointerMove" | "onPointerUp" | "onPointerCancel"
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

    // restore the saved layout once settled, so the server and the hydrating page render alike
    onSettled(() => control.restore());

    return (
        <ResizableContext value={control}>
            <div
                data-slot="resizable-panel-group"
                data-panel-group-direction={group.direction}
                {...rest}
                ref={(element) => control.setElement(element)}
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
    control.register(panel);
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
                panel.collapsible && control.isShareable(panel.collapsedSize)
                    ? control.sizeOf(panel) <= control.share(panel.collapsedSize)
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

/** Render the window splitter between two panels. */
export function ResizableHandle(properties: ResizableHandleProperties): JSX.Element {
    // read the group, this handle's place and the panel before it
    const control = useResizable();
    const locale = useLocale();
    const handle = control.nextHandle();
    const rest = omit(properties, "withHandle", "xstyle", "style");
    const before = (): ResizablePanelState | undefined => control.panels()[handle];
    let start: { readonly extent: number; readonly layout: number[] } = { extent: 0, layout: [] };

    // move by the pointer's share of the group since it pressed
    const move = new Move(
        {
            distance: 0,
            onMoveStart: (event) => {
                const group = event.target.parentElement?.getBoundingClientRect();
                const extent = control.direction() === "horizontal" ? group?.width : group?.height;
                start = { extent: extent ?? 0, layout: control.layout() };
            },
            onMove: (event) => {
                // step by an arrow key along the group's axis
                const isHorizontal = control.direction() === "horizontal";
                if (event.pointerType === "keyboard") {
                    const mirror = isHorizontal && locale.direction === "rtl" ? -1 : 1;
                    const steps = isHorizontal ? event.deltaX * mirror : event.deltaY;
                    control.move(handle, steps * KEYBOARD_STEP);

                    return;
                }

                // follow a pointer by its share of the group since it pressed
                const offset = isHorizontal ? event.offsetX : event.offsetY;
                if (start.extent > 0) {
                    control.apply(start.layout);
                    control.move(handle, (offset / start.extent) * 100);
                }
            },
        },
        useEnvironment().clock,
    );

    // collapse or restore the panel before on Enter, and move to its limits on Home and End
    const keys = {
        onKeyDown: (event: KeyboardEvent) => {
            const panel = before();
            if (event.key === "Enter") {
                event.preventDefault();
                control.toggle(handle);
            } else if (panel !== undefined && (event.key === "Home" || event.key === "End")) {
                event.preventDefault();
                const limit = event.key === "Home" ? panel.minimum : panel.maximum;
                control.move(handle, control.share(limit) - control.sizeOf(panel));
            }
        },
    };

    return (
        <div
            role="separator"
            tabindex={0}
            aria-orientation={control.direction() === "horizontal" ? "vertical" : "horizontal"}
            aria-controls={before()?.id}
            aria-valuenow={Math.round(sizeOrZero(control, before()))}
            aria-valuemin={limitOf(control, before()?.minimum ?? 0)}
            aria-valuemax={limitOf(control, before()?.maximum ?? 100)}
            data-slot="resizable-handle"
            {...rest}
            {...mergeProperties(move.attributes(), keys)}
            {...style.attributes(
                [state.focus, styles.handle, handles[control.direction()], properties.xstyle],
                properties.style,
            )}
        >
            <Show when={properties.withHandle === true}>
                <div aria-hidden="true" {...style.attrs(styles.grip, grips[control.direction()])} />
            </Show>
        </div>
    );
}

/** Read a handle's limit as a rounded share, none for pixels before the group is measured. */
function limitOf(control: ResizableControl, limit: ResizableSize): number | undefined {
    return control.isShareable(limit) ? Math.round(control.share(limit)) : undefined;
}

/** Read a panel's share, or zero before the panel mounts. */
function sizeOrZero(control: ResizableControl, panel: ResizablePanelState | undefined): number {
    return panel === undefined ? 0 : control.sizeOf(panel);
}
