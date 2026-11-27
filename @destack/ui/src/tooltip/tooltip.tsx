import type { Clock } from "@destack/environment";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, radius, space, stroke, width } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    type Accessor,
    createContext,
    createEffect,
    createSignal,
    type JSX,
    omit,
    onCleanup,
    Show,
    untrack,
    useContext,
    useEnvironment,
} from "@destack/view";
import { Hover } from "../interaction/index.ts";
import {
    type ElementPartProperties,
    mergeProperties,
    type PartAttributes,
    type Render,
    renderPart,
} from "../part/index.ts";
import { HoverPopover } from "../popover/index.ts";
import { type Anchor, Position, PositionAnchor, ANCHOR_PROPERTIES } from "../position/index.ts";
import {
    controlOf,
    OpenChange,
    type OpenChildren,
    type OpenProperties,
    type OpenReason,
    Overlay,
    TopLayer,
    type TriggerProperties,
} from "../layer/index.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The wait before a resting pointer shows a tooltip, in milliseconds. */
const DELAY = 700;

/** The wait before a tooltip the pointer left hides, in milliseconds. */
const CLOSE_DELAY = 100;

/** The time after a tooltip hides in which the next one shows at once, in milliseconds. */
const TIMEOUT = 300;

/** The widest a tooltip grows before its text wraps, 320px. */
const WIDTH = width.tooltip;

/** The properties people change in a tooltip's controls and inspector. */
const TOOLTIP_CONTENT_PROPERTIES = {
    side: ANCHOR_PROPERTIES.side.default("top"),
    align: ANCHOR_PROPERTIES.align.default("center"),
};

/** A tooltip: a hover popover that shows at once and without motion while its group is warm. */
export class TooltipControl<Payload = unknown> extends HoverPopover<Payload> {
    /** Whether the tooltip shows without motion: warm, or opened by the keyboard. */
    readonly isInstant: Accessor<boolean>;
    /** The group the tooltip shows and hides in. */
    readonly #group: TooltipGroup;
    /** The clock the group's warmth is read on. */
    readonly #clock: Clock;
    /** Set whether the tooltip shows without motion. */
    readonly #setInstant: (isInstant: boolean) => void;
    /** Whether the tooltip's content is on the page: while open, and while it leaves. */
    readonly isPresent: Accessor<boolean>;
    /** Set whether the tooltip's content left the page, as its exit motion ended. */
    readonly #setGone: (isGone: boolean) => void;

    /** Create a tooltip of the nearest group, waiting the group's delay on the page's clock. */
    constructor(options: TooltipOptions = {}) {
        // wait the group's delay, none while it is warm
        const group = useContext(TooltipGroupContext);
        const { clock } = useEnvironment();
        super(
            options,
            () => group.delayOf(options.delay, clock.now()),
            () => options.closeDelay ?? group.closeDelay,
        );
        this.#group = group;
        this.#clock = clock;
        const [isInstant, setInstant] = createSignal(false);
        this.isInstant = isInstant;
        this.#setInstant = setInstant;

        // keep the content on the page from opening until its exit motion ends, never in between
        const [isGone, setGone] = createSignal(true, { ownedWrite: true });
        this.isPresent = () => this.isOpen() || !isGone();
        this.#setGone = setGone;

        // count the tooltip in its group as it shows and hides, its content on the page once shown
        createEffect(this.isOpen, (isShown, wasShown) => {
            group.follow(isShown, wasShown, clock.now());
            if (isShown) {
                setGone(false);
            }
        });
    }

    /** Take the content off the page once its exit motion ends, unless it opened again. */
    unmount(content: HTMLElement): void {
        void Promise.allSettled(content.getAnimations().map((motion) => motion.finished)).then(
            () => {
                if (!untrack(this.isOpen)) {
                    this.#setGone(true);
                }
            },
        );
    }

    /** Leave the content to the group, which says whether a tooltip moves. */
    override mark(): void {}

    /** Open after the delay, at once and without motion while the group is warm. */
    override openLater(change?: OpenChange<Payload>): void {
        this.#setInstant(this.#group.isWarm(this.#clock.now()));
        super.openLater(change);
    }

    /** Open now, without motion where asked, as when the keyboard reaches the trigger. */
    override open(change?: OpenChange<Payload>, isImmediate = false): void {
        if (isImmediate) {
            this.#setInstant(true);
        }
        super.open(change);
    }
}

/** The tooltip of the nearest tooltip root, null outside one. */
export const TooltipContext = createContext<TooltipControl | null>(null);

/** The tooltips that show one after another without waiting, after the last one hid. */
export class TooltipGroup {
    /** The wait before a resting pointer shows a tooltip of the group, in milliseconds. */
    readonly delay: number;
    /** The wait before a tooltip of the group the pointer left hides, in milliseconds. */
    readonly closeDelay: number;
    /** The time after a tooltip hides in which the next one shows at once, in milliseconds. */
    readonly timeout: number;
    /** The number of the group's tooltips on screen. */
    #shown: number;
    /** When the last of the group's tooltips hid. */
    #hiddenAt: number;

    /** Create a group that waits a delay, skipped for a while after a tooltip hides. */
    constructor(delay: number, closeDelay: number, timeout: number) {
        // start cold with no tooltip shown
        this.delay = delay;
        this.closeDelay = closeDelay;
        this.timeout = timeout;
        this.#shown = 0;
        this.#hiddenAt = Number.NEGATIVE_INFINITY;
    }

    /** Report whether a tooltip of the group showed a moment ago, so the next shows at once. */
    isWarm(now: number): boolean {
        return this.#shown > 0 || now - this.#hiddenAt < this.timeout;
    }

    /** Return the wait before a tooltip shows at a time. */
    delayOf(own: number | undefined, now: number): number {
        return this.isWarm(now) ? 0 : (own ?? this.delay);
    }

    /** Count a tooltip showing or hiding at a time. */
    follow(isShown: boolean, wasShown: boolean | undefined, now: number): void {
        if (isShown) {
            this.#shown += 1;
        } else if (wasShown === true) {
            this.#shown -= 1;
            this.#hiddenAt = now;
        }
    }
}

/** The group of the nearest tooltip provider, one group for the page outside one. */
const TooltipGroupContext = createContext<TooltipGroup>(
    new TooltipGroup(DELAY, CLOSE_DELAY, TIMEOUT),
);

/** The properties of a tooltip provider. */
export interface TooltipProviderProperties {
    /** The wait before a resting pointer shows a tooltip in milliseconds, 700 by default. */
    readonly delay?: number;
    /** The wait before a tooltip the pointer left hides in milliseconds, 100 by default. */
    readonly closeDelay?: number;
    /** The time after a tooltip hides in which the next one shows at once. */
    readonly timeout?: number;
    /** The tooltips the group includes. */
    readonly children?: JSX.Element;
}

/** Group the tooltips inside, so moving from one trigger to the next shows its tooltip at once. */
export function TooltipProvider(properties: TooltipProviderProperties): JSX.Element {
    const group = new TooltipGroup(
        properties.delay ?? DELAY,
        properties.closeDelay ?? CLOSE_DELAY,
        properties.timeout ?? TIMEOUT,
    );

    return <TooltipGroupContext value={group}>{properties.children}</TooltipGroupContext>;
}

/** The styles of a tooltip. */
const styles = style.create({
    content: {
        outlineStyle: { default: "none", [media.forcedColors]: "solid" },
        outlineWidth: stroke.border,
        width: "fit-content",
        maxWidth: `min(${WIDTH}, 100vw - 2 * ${space[1]})`,
        inset: "auto",
        paddingBlock: space[1],
        paddingInline: space[3],
        borderRadius: radius[3],
        backgroundColor: color.foreground,
        color: color.background,
        textWrap: "balance",
        overflowWrap: "anywhere",
    },
    instant: {
        transitionDuration: "0s",
    },
});

/** How a tooltip opens. */
export interface TooltipOptions extends OpenProperties {
    /** The wait before a resting pointer shows the tooltip, in milliseconds. */
    readonly delay?: number;
    /** The wait before a tooltip the pointer left hides, in milliseconds. */
    readonly closeDelay?: number;
}

/** The properties of a tooltip root, its open state included. */
export interface TooltipProperties<Payload = unknown> extends TooltipOptions {
    /** The tooltip that triggers elsewhere open, which holds the open state in the root's place. */
    readonly control?: TooltipControl<Payload>;
    /** The trigger and content, or what it shows for the trigger it opened from. */
    readonly children?: OpenChildren<Payload>;
}

/** The properties of a tooltip's trigger, a native button's attributes included. */
export type TooltipTriggerProperties<Payload = unknown> = ElementPartProperties &
    Omit<JSX.ButtonHTMLAttributes<HTMLButtonElement>, "class" | "style"> &
    TriggerProperties<Payload, TooltipControl<Payload>>;

/** The properties of a tooltip's content besides its declared ones: its anchor and gap. */
export interface TooltipContentAttributes extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class"
> {
    /** What it is placed beside in place of its trigger: an element, or a box in the viewport. */
    readonly anchor?: Anchor | undefined;
    /** The gap from the trigger in pixels, 4 by default. */
    readonly sideOffset?: number;
    /** The StyleX styles applied after the tooltip's styles. */
    readonly xstyle?: style.Styles;
    /** Render another element with its attributes, its own by default. */
    readonly render?: Render;
}

/** Read the tooltip of the nearest tooltip root, refusing elements outside one. */
export function useTooltip(): TooltipControl {
    const tooltip = useContext(TooltipContext);
    if (tooltip === null) {
        throw new TypeError("tooltip elements need a tooltip root around them");
    }

    return tooltip;
}

/** Connect a trigger to the tooltip that describes it. */
export function Tooltip<Payload = unknown>(properties: TooltipProperties<Payload>): JSX.Element {
    // take the caller's tooltip, else one of its own
    const tooltip = controlOf(properties, () => new TooltipControl<Payload>(properties));

    return (
        <TooltipContext value={tooltip}>{tooltip.state.render(properties.children)}</TooltipContext>
    );
}

/** Render the element that shows its tooltip on hover after the delay and on focus at once. */
export function TooltipTrigger<Payload = unknown>(
    properties: TooltipTriggerProperties<Payload>,
): JSX.Element {
    // show its root's tooltip, or the one it is given from elsewhere
    const given = untrack(() => properties.control);
    const tooltip = given ?? useTooltip();
    const rest = omit(properties, "control", "payload");
    let element: HTMLElement | undefined;
    const change = (reason: OpenReason, target: HTMLElement, event?: Event) =>
        new OpenChange(reason, event, { element: target, payload: properties.payload });
    onCleanup(() => given?.drop(element));

    // open the tooltip on hover after its delay, and on focus at once
    let isPressed = false;
    const hover = new Hover({
        onHoverStart: ({ target }) => tooltip.openLater(change("trigger-hover", target)),
        onHoverEnd: (event) => {
            isPressed = false;
            tooltip.leave(event);
        },
    });

    return renderPart(
        "button",
        "tooltip-trigger",
        rest,
        [],
        mergeProperties(hover.attributes(), {
            type: "button" as const,
            get "data-popup-open"() {
                return tooltip.state.isFrom(element) ? "" : undefined;
            },
            get "aria-describedby"() {
                // point at the tooltip only while it is on the page
                return tooltip.isPresent() && (given === undefined || tooltip.state.isFrom(element))
                    ? tooltip.id
                    : undefined;
            },
            ref: (target: HTMLElement) => {
                element = target;
                if (given === undefined) {
                    tooltip.setTrigger(target);
                }
            },
            onFocus: (event: FocusEvent & { readonly currentTarget: HTMLElement }) => {
                // show a tooltip the keyboard reaches at once and without motion, none for a pointer
                const target = event.currentTarget;
                if (!isPressed && target.matches(":focus-visible")) {
                    tooltip.open(change("trigger-focus", target, event), true);
                }
            },
            onPointerDown: (event: PointerEvent) => {
                isPressed = true;
                tooltip.close(new OpenChange("trigger-press", event));
            },
            onPointerUp: () => (isPressed = false),
            onPointerCancel: () => (isPressed = false),
            onBlur: (event: FocusEvent) => tooltip.close(new OpenChange("focus-out", event)),
            onKeyDown: (event: KeyboardEvent) => {
                // dismiss the tooltip on Escape
                if (event.key === "Escape") {
                    tooltip.close(new OpenChange("escape-key", event));
                }
            },
        }),
    );
}

/** Render the text that describes its trigger, in the top layer beside it. */
export const TooltipContent = defineComponent({
    name: "tooltipContent",
    properties: TOOLTIP_CONTENT_PROPERTIES,
    render(
        properties: RenderProperties<TooltipContentAttributes, typeof TOOLTIP_CONTENT_PROPERTIES>,
    ): JSX.Element {
        // render the content only while the tooltip is on the page, as most never open
        const tooltip = useTooltip();

        return (
            <Show when={tooltip.isPresent()}>
                <TooltipBody {...properties} />
            </Show>
        );
    },
});

/** Render a tooltip's content while it is on the page: shown, then hidden as it leaves. */
function TooltipBody(
    properties: RenderProperties<TooltipContentAttributes, typeof TOOLTIP_CONTENT_PROPERTIES>,
): JSX.Element {
    // read the tooltip and its placement
    const tooltip = useTooltip();
    const isInstant = tooltip.isInstant;
    const { scheduler } = useEnvironment();
    const rest = omit(properties, "anchor", "side", "align", "sideOffset");

    // show and hide the tooltip as it opens and closes
    createEffect(tooltip.isOpen, (isOpen) => untrack(() => tooltip.sync(isOpen)));

    // leave the tooltip open while a pointer rests on it
    const hover = new Hover({
        onHoverStart: () => tooltip.open(),
        onHoverEnd: () => tooltip.closeLater(new OpenChange("trigger-hover")),
    });

    return (
        <Overlay>
            <PositionAnchor
                anchor={properties.anchor}
                onAnchor={(element) => tooltip.setAnchor(element)}
            />
            {renderPart(
                "div",
                "tooltip-content",
                rest,
                () => [
                    text.xs,
                    styles.content,
                    Position.beside(properties.side, properties.align),
                    Position.offset(properties.side, properties.sideOffset),
                    isInstant() && styles.instant,
                ],
                mergeProperties(hover.attributes(), {
                    id: tooltip.id,
                    popover: "hint" as const,
                    role: "tooltip" as const,
                    get "data-open"() {
                        return tooltip.isOpen() ? "" : undefined;
                    },
                    get "data-closed"() {
                        return tooltip.isOpen() ? undefined : "";
                    },
                    get "data-instant"() {
                        return isInstant() ? "" : undefined;
                    },
                    get "data-side"() {
                        return properties.side;
                    },
                    get "data-align"() {
                        return properties.align;
                    },
                    ref: (element: HTMLElement) => {
                        TopLayer.of(element).watch(element, scheduler);
                        tooltip.setContent(element);
                    },
                    onToggle: (event: ToggleEvent & { readonly currentTarget: HTMLElement }) => {
                        // follow the platform, then leave the page once hidden
                        tooltip.follow(event);
                        if (event.newState === "closed") {
                            tooltip.unmount(event.currentTarget);
                        }
                    },
                }),
            )}
        </Overlay>
    );
}

/** The properties of a tooltip's content, the native element's attributes included. */
export type TooltipContentProperties = ComponentProperties<typeof TooltipContent>;

/** Render a control with a hint: a tooltip saying its text, the control its trigger. */
export function Hint(properties: {
    /** What the hint says, none for a control without one. */
    readonly text: JSX.Element;
    /** Render the control with the trigger's attributes, none where it has no hint. */
    readonly children: (attributes: PartAttributes) => JSX.Element;
}): JSX.Element {
    // leave a control without text, or one a tooltip around already triggers, as it is
    const isTriggering = useContext(TooltipContext) !== null;
    if (isTriggering || untrack(() => properties.text) === undefined) {
        return properties.children({});
    }

    return (
        <Tooltip>
            <TooltipTrigger render={(attributes) => properties.children(attributes)} />
            <TooltipContent>{properties.text}</TooltipContent>
        </Tooltip>
    );
}
