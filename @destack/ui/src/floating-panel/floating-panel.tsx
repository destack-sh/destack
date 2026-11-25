import { media } from "@destack/style/media.stylex";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { space, stroke, weight, color } from "@destack/theme/tokens.stylex";
import {
    createContext,
    createControllableSignal,
    type JSX,
    omit,
    untrack,
    useContext,
    useEnvironment,
    useLocale,
} from "@destack/view";
import {
    Dialog,
    type DialogButtonProperties,
    DialogClose,
    DialogContent,
    type DialogContentProperties,
    type DialogElementProperties,
    type DialogProperties,
    DialogTitle,
    DialogTrigger,
    type DialogTriggerProperties,
} from "../dialog/index.ts";
import { Frame, type FrameBounds } from "../interaction/index.ts";
import { mergeProperties, renderPart } from "../part/index.ts";
import { FrameSides } from "./sides.tsx";

/** The size of a panel without bounds, in pixels: a palette's. */
const SIZE = { width: 360, height: 280 };

/** The least size of a panel without one, in pixels. */
const LEAST = { width: 200, height: 120 };

/** The controls in a header that a press acts on. */
const CONTROLS = "button, a[href], input, select, textarea";

/** The styles of a panel. */
const styles = style.create({
    content: {
        position: "fixed",
        display: { default: "none", ":is([open])": "flex" },
        flexDirection: "column",
        gap: 0,
        margin: 0,
        padding: 0,
        maxWidth: "100vw",
        maxHeight: "100dvh",
        overflow: "visible",
    },
    place: (start: string, top: string, width: string, height: string) => ({
        insetInlineStart: start,
        insetInlineEnd: "auto",
        top,
        bottom: "auto",
        width,
        height,
    }),
    header: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
        flexShrink: 0,
        paddingBlock: space[2],
        paddingInline: space[3],
        borderBlockEndWidth: stroke.border,
        borderBlockEndColor: color.border,
        cursor: { default: "grab", ":is([data-moving])": "grabbing" },
        userSelect: "none",
        touchAction: "none",
    },
    title: {
        flexGrow: 1,
        minWidth: 0,
        margin: 0,
        fontWeight: weight.medium,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    body: {
        flexGrow: 1,
        minHeight: 0,
        padding: space[3],
        overflow: "auto",
        overscrollBehavior: { default: "contain", [media.print]: "auto" },
    },
});

/** The frame of the nearest panel, null outside one. */
const FrameContext = createContext<Frame | null>(null);

/** Hold the open state of a panel that floats beside the page. */
export function FloatingPanel<Payload = unknown>(
    properties: Omit<DialogProperties<Payload>, "modal">,
): JSX.Element {
    return <Dialog {...properties} modal={false} />;
}

/** Render the button that opens its panel. */
export function FloatingPanelTrigger<Payload = unknown>(
    properties: DialogTriggerProperties<Payload>,
): JSX.Element {
    return <DialogTrigger data-slot="floating-panel-trigger" {...properties} />;
}

/** The properties of a panel's content, the native dialog's attributes included. */
export type FloatingPanelContentProperties = Omit<DialogContentProperties, "showCloseButton"> & {
    /** Where it is in the window, in pixels, which makes it controlled. */
    readonly bounds?: FrameBounds;
    /** Where it is at first, the middle of the window by default. */
    readonly defaultBounds?: FrameBounds;
    /** Handle its bounds changing. */
    readonly onBoundsChange?: (bounds: FrameBounds) => void;
    /** Its least size in pixels, a small palette's by default. */
    readonly minSize?: { readonly width: number; readonly height: number };
};

/** Render a panel over the page, which a person moves by its header and resizes by its sides. */
export function FloatingPanelContent(properties: FloatingPanelContentProperties): JSX.Element {
    // follow its bounds, in the middle of the window until it has some
    const locale = useLocale();
    const [bounds, setBounds] = createControllableSignal<FrameBounds | undefined>({
        isControlled: () => properties.bounds !== undefined,
        value: () => properties.bounds,
        defaultValue: untrack(() => properties.defaultBounds),
        onChange: (next) => (next === undefined ? undefined : properties.onBoundsChange?.(next)),
    });
    const frame = new Frame(
        {
            bounds: () => untrack(bounds) ?? middle(),
            room: () => ({ width: window.innerWidth, height: window.innerHeight }),
            least: () => properties.minSize ?? LEAST,
            onChange: setBounds,
            direction: () => locale.direction,
            canMove: ({ target }) =>
                !(target instanceof Element && target.closest(CONTROLS) !== null),
        },
        useEnvironment().clock,
    );
    const rest = omit(
        properties,
        "bounds",
        "defaultBounds",
        "onBoundsChange",
        "minSize",
        "xstyle",
        "children",
    );
    const place = (): style.Styles => {
        const at = bounds();

        return at === undefined
            ? styles.place(
                  `calc(50% - ${pixels(SIZE.width / 2)})`,
                  `calc(50% - ${pixels(SIZE.height / 2)})`,
                  pixels(SIZE.width),
                  pixels(SIZE.height),
              )
            : styles.place(
                  `max(0px, min(${pixels(at.start)}, 100vw - ${pixels(at.width)}))`,
                  `max(0px, min(${pixels(at.top)}, 100dvh - ${pixels(at.height)}))`,
                  pixels(at.width),
                  pixels(at.height),
              );
    };

    return (
        <FrameContext value={frame}>
            <DialogContent
                data-slot="floating-panel-content"
                closedby="closerequest"
                {...rest}
                showCloseButton={false}
                onPointerDown={(event) => raise(event.currentTarget)}
                onFocusIn={(event) => raise(event.currentTarget)}
                xstyle={[styles.content, place(), properties.xstyle]}
            >
                {properties.children}
                <FrameSides frame={frame} />
            </DialogContent>
        </FrameContext>
    );
}

/** Render what a person moves its panel by, or the arrow keys while it has the focus. */
export function FloatingPanelHeader(
    properties: DialogElementProperties<HTMLDivElement>,
): JSX.Element {
    const frame = useContext(FrameContext);
    if (frame === null) {
        throw new TypeError("a floating panel's header needs its content around it");
    }

    return renderPart(
        "div",
        "floating-panel-header",
        properties,
        [text.sm, styles.header],
        mergeProperties(frame.move().attributes(), { tabindex: 0 }),
    );
}

/** Render the title that names its panel. */
export function FloatingPanelTitle(
    properties: DialogElementProperties<HTMLHeadingElement>,
): JSX.Element {
    return (
        <DialogTitle
            data-slot="floating-panel-title"
            {...properties}
            xstyle={[text.sm, styles.title, properties.xstyle]}
        />
    );
}

/** Render the button that closes its panel. */
export function FloatingPanelClose(properties: DialogButtonProperties): JSX.Element {
    return <DialogClose data-slot="floating-panel-close" {...properties} />;
}

/** Render what a panel shows, which scrolls under its header. */
export function FloatingPanelBody(
    properties: DialogElementProperties<HTMLDivElement>,
): JSX.Element {
    return renderPart("div", "floating-panel-body", properties, [text.sm, styles.body]);
}

/** Read the bounds in the middle of the window. */
function middle(): FrameBounds {
    return {
        start: Math.max((window.innerWidth - SIZE.width) / 2, 0),
        top: Math.max((window.innerHeight - SIZE.height) / 2, 0),
        ...SIZE,
    };
}

/** Put a panel over the page's others. */
function raise(panel: HTMLElement): void {
    // count from the highest of the others
    const others = [
        ...panel.ownerDocument.querySelectorAll<HTMLElement>("[data-slot=floating-panel-content]"),
    ].filter((each) => each !== panel);
    const highest = Math.max(0, ...others.map((each) => Number(each.style.zIndex) || 0));
    if ((Number(panel.style.zIndex) || 0) <= highest) {
        panel.style.zIndex = String(highest + 1);
    }
}

/** Write a length in pixels. */
function pixels(length: number): string {
    return `${String(length)}px`;
}
