import type { Direction } from "@destack/locale";
import type { Axis } from "../position/index.ts";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { type JSX, merge, omit, onCleanup, useLocale } from "@destack/view";
import { useOverflowEntry } from "../overflow/index.ts";
import { Button, type ButtonProperties } from "../button/index.ts";
import { type ElementPartProperties, mergeProperties, renderPart } from "../part/index.ts";
import { Separator, type SeparatorProperties } from "../separator/index.ts";

/** What a toolbar moves the focus between, its items that Tab passes over included. */
const ITEMS =
    "button:not(:disabled), a[href], input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]";

/** What takes the arrow keys itself, which Tab alone leaves. */
const OWN_ARROWS =
    "textarea, [contenteditable], [role=slider], [role=spinbutton], input:not([type=checkbox], [type=radio], [type=button])";

/** The text the arrow keys leave at its ends. */
const TEXT = "input:is(:not([type]), [type=text], [type=search], [type=url], [type=tel])";

/** The keys a list of options leaves alone. */
const ACROSS: ReadonlySet<string> = new Set(["ArrowLeft", "ArrowRight"]);

/** The styles of a toolbar. */
const styles = style.create({
    toolbar: {
        display: "flex",
        alignItems: "center",
        gap: space[1],
        minWidth: 0,
    },
    vertical: {
        flexDirection: "column",
    },
    controls: {
        display: "contents",
        flexDirection: "inherit",
    },
    group: {
        display: "flex",
        flexDirection: "inherit",
        alignItems: "center",
        gap: space[1],
    },
    separator: {
        alignSelf: "stretch",
        marginBlock: space[1],
    },
    across: {
        marginBlock: 0,
        marginInline: space[1],
    },
});

/** The properties of an element of a toolbar, with its own attributes. */
export type ToolbarElementProperties = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> &
    ElementPartProperties;

/** The properties of a toolbar, with its element's attributes. */
export type ToolbarProperties = ToolbarElementProperties & {
    /** The direction it runs along, horizontal by default. */
    readonly orientation?: Axis;
    /** Whether the arrow keys go from its last item to its first, as they do by default. */
    readonly loop?: boolean;
    /** Whether its controls take no input. */
    readonly disabled?: boolean | undefined;
};

/** Render controls as one Tab stop, which the arrow keys move between. */
export function Toolbar(properties: ToolbarProperties): JSX.Element {
    // remember the control the focus left from, and come back to it
    const locale = useLocale();
    const toolbar = merge({ orientation: "horizontal" as const, loop: true }, properties);
    const rest = omit(properties, "orientation", "loop", "disabled", "children");
    let last: HTMLElement | undefined;
    const removers: (() => void)[] = [];
    onCleanup(() => removers.forEach((remove) => remove()));
    const move = (event: KeyboardEvent, element: HTMLElement): void => {
        // leave the arrows to a control that takes them, and others along the other axis
        const { target } = event;
        const isAcross = toolbar.orientation === "horizontal";
        const isTaken = event.altKey || event.ctrlKey || event.metaKey || event.defaultPrevented;
        if (!(target instanceof HTMLElement) || isTaken) {
            return;
        }
        const forward = isAcross
            ? locale.direction === "rtl"
                ? "ArrowLeft"
                : "ArrowRight"
            : "ArrowDown";
        const backward = isAcross
            ? locale.direction === "rtl"
                ? "ArrowRight"
                : "ArrowLeft"
            : "ArrowUp";

        // move to the item beside it, or an end
        const items = itemsOf(element);
        const place = items.indexOf(target);
        const next =
            event.key === forward
                ? place + 1
                : event.key === backward
                  ? place - 1
                  : event.key === "Home"
                    ? 0
                    : event.key === "End"
                      ? items.length - 1
                      : undefined;
        const isOwn =
            target.matches(OWN_ARROWS) || target.matches("select")
                ? !leaves(target, event, locale.direction)
                : false;
        if (next === undefined || place === -1 || isOwn) {
            return;
        }
        event.preventDefault();
        event.stopPropagation();
        const count = items.length;
        const looped = toolbar.loop
            ? (next + count) % count
            : Math.min(Math.max(next, 0), count - 1);
        items[looped]?.focus();
    };
    let isTabbing = false;
    let isTabbed = false;
    const follow = (event: KeyboardEvent | PointerEvent): void => {
        isTabbed = event instanceof KeyboardEvent && event.key === "Tab";
    };
    const keys = (event: KeyboardEvent, element: HTMLElement): void => {
        // leave by Tab as from one control: the browser goes on from its first or last
        const isKept =
            event.target instanceof HTMLElement &&
            event.target.matches(OWN_ARROWS) &&
            !event.target.matches(TEXT);
        if (event.key === "Tab" && !event.defaultPrevented && !isKept) {
            const items = itemsOf(element).filter((item) => item.tabIndex >= 0);
            last = document.activeElement instanceof HTMLElement ? document.activeElement : last;
            isTabbing = true;
            (event.shiftKey ? items[0] : items.at(-1))?.focus({ preventScroll: true });
        } else {
            move(event, element);
        }
    };
    const enter = (event: FocusEvent & { readonly currentTarget: HTMLElement }): void => {
        // come back by Tab to the control the focus left from
        const { currentTarget, relatedTarget, target } = event;
        const isInside = relatedTarget instanceof Node && currentTarget.contains(relatedTarget);
        const isKept = last !== undefined && itemsOf(currentTarget).includes(last);
        if (isTabbed && !isInside && isKept && last !== target) {
            last?.focus();
        }
    };
    const leave = (event: FocusEvent & { readonly currentTarget: HTMLElement }): void => {
        // remember the control, unless Tab moved to an end on its way out
        const { currentTarget, relatedTarget, target } = event;
        const isInside = relatedTarget instanceof Node && currentTarget.contains(relatedTarget);
        if (!isInside && target instanceof HTMLElement && !isTabbing) {
            last = target;
        }
        isTabbing = isInside && isTabbing;
    };

    return renderPart(
        "div",
        "toolbar",
        rest,
        () => [styles.toolbar, toolbar.orientation === "vertical" && styles.vertical],
        mergeProperties(
            {
                role: "toolbar" as const,
                get "aria-orientation"() {
                    return toolbar.orientation;
                },
                get "aria-disabled"() {
                    return toolbar.disabled === true ? ("true" as const) : undefined;
                },
                get children() {
                    return (
                        <fieldset
                            role="none"
                            disabled={toolbar.disabled === true}
                            {...style.attrs(styles.controls)}
                        >
                            {toolbar.children}
                        </fieldset>
                    );
                },
                onFocusIn: enter,
                onFocusOut: leave,
            },
            {
                ref: (element: HTMLElement) => {
                    // tell the focus by Tab from a press and a call
                    const { ownerDocument } = element;
                    ownerDocument.addEventListener("keydown", follow, { capture: true });
                    ownerDocument.addEventListener("pointerdown", follow, { capture: true });
                    removers.push(() => {
                        ownerDocument.removeEventListener("keydown", follow, { capture: true });
                        ownerDocument.removeEventListener("pointerdown", follow, { capture: true });
                    });

                    // take the arrows before a group inside moves by them alone
                    element.addEventListener("keydown", (event) => keys(event, element), {
                        capture: true,
                    });
                },
            },
        ),
    );
}

/** Render controls of a toolbar that belong together. */
export function ToolbarGroup(properties: ToolbarElementProperties): JSX.Element {
    return renderPart("div", "toolbar-group", properties, styles.group, { role: "group" as const });
}

/** Render a line between controls of a toolbar, across the way it runs. */
export function ToolbarSeparator(properties: SeparatorProperties): JSX.Element {
    // stand across a horizontal toolbar
    const separator = merge({ orientation: "vertical" as const }, properties);

    // show only between two that show
    let element: HTMLElement | undefined;
    useOverflowEntry({ kind: "separator", element: () => element });

    return (
        <Separator
            {...separator}
            ref={[separator.ref, (created: HTMLElement) => (element = created)]}
            data-slot="toolbar-separator"
            xstyle={[
                styles.separator,
                separator.orientation === "horizontal" && styles.across,
                properties.xstyle,
            ]}
        />
    );
}

/** Render a square button of a toolbar showing its icon, focusable by the keys while disabled. */
export function ToolbarButton(properties: ButtonProperties): JSX.Element {
    // stay in reach of the keys while disabled
    const rest = omit(properties, "disabled", "onClick");

    // hide in an overflow where it does not fit
    let element: HTMLElement | undefined;
    useOverflowEntry({ kind: "item", element: () => element });

    return (
        <Button
            isIconOnly
            data-slot="toolbar-button"
            variant="ghost"
            {...rest}
            ref={[rest.ref, (created: HTMLElement) => (element = created)]}
            aria-disabled={properties.disabled === true ? "true" : undefined}
            onClick={(event) => {
                if (properties.disabled !== true && typeof properties.onClick === "function") {
                    properties.onClick(event);
                }
            }}
        />
    );
}

/** Report whether a key leaves a control that takes the arrows: a list, or a text at its end. */
function leaves(target: HTMLElement, event: KeyboardEvent, direction: Direction): boolean {
    // leave a list of options across it
    if (target.matches("select")) {
        return ACROSS.has(event.key);
    }

    // leave a text where the caret goes no further, its direction's way
    if (!(target instanceof HTMLInputElement) || !target.matches(TEXT)) {
        return false;
    }
    const { selectionStart, selectionEnd, value } = target;
    const isCaret = selectionStart === selectionEnd && !event.shiftKey;
    const isForward = event.key === (direction === "rtl" ? "ArrowLeft" : "ArrowRight");
    const isBackward = event.key === (direction === "rtl" ? "ArrowRight" : "ArrowLeft");

    return (
        isCaret &&
        ((isForward && selectionEnd === value.length) || (isBackward && selectionStart === 0))
    );
}

/** List what a toolbar moves the focus between, without what its controls open. */
function itemsOf(toolbar: HTMLElement): HTMLElement[] {
    return [...toolbar.querySelectorAll<HTMLElement>(ITEMS)].filter(
        (item) => item.closest("[inert], [hidden], [popover], select *") === null,
    );
}
