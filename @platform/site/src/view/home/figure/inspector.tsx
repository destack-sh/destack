import { color, font } from "@destack/theme/tokens.stylex";
import { present } from "@destack/schema";
import * as style from "@destack/style";
import { createMemo, createSignal, type JSX } from "@destack/view";

import type { Lighting } from "./ledger";
import type { Stagger } from "./stagger";
import { palette } from "../../palette.stylex";
import { createResizeObserver } from "@destack/view/primitives/resize-observer";

/**
 * Show which service each part of an app figure runs on: point at a part to outline it, name its service, and light that service in the ledger.
 *
 * Once its service is owned, a part shows a solid outline and the service's name.
 * While its service is rented, a part shows a faint dashed outline and says the service is rented.
 */
export function Inspector(properties: {
    isOpenAt: Stagger;
    lighting?: Lighting;
    services: readonly string[];
    children: JSX.Element;
}) {
    // hold the frame and the part under the pointer
    let frame: HTMLDivElement | undefined;
    const [pointed, setPointed] = createSignal<HTMLElement>();

    // point at the part under the pointer and light the service it runs on
    const point = (event: PointerEvent) => {
        // find the nearest named part and the service around it
        const target = event.target instanceof Element ? event.target : undefined;
        const part = target?.closest<HTMLElement>("[data-service]") ?? undefined;
        const service = part?.dataset["service"];

        // show the part and light its service in the ledger
        setPointed(part);
        properties.lighting?.onLight(service === undefined ? undefined : Number(service));
    };

    // stop pointing once the pointer leaves the app
    const leave = () => {
        setPointed(undefined);
        properties.lighting?.onLight(undefined);
    };

    // show the pointed part or else the part of the lit service
    const shown = () => {
        // prefer the part under the pointer over the part of the lit service
        const lit = properties.lighting?.lit;
        const part = pointed();
        if (part !== undefined) {
            return part;
        } else if (lit !== undefined && frame !== undefined) {
            return frame.querySelector<HTMLElement>(`[data-service="${lit}"]`) ?? undefined;
        }

        return undefined;
    };

    // read whether the shown part's service or else the figure's last step is open
    const isOpenFor = (part: HTMLElement) => {
        const service = part.closest<HTMLElement>("[data-service]")?.dataset["service"];

        return properties.isOpenAt(
            service === undefined ? properties.services.length + 1 : Number(service),
        );
    };

    // name the shown part by its component and service once owned and by its markup while rented
    const label = (part: HTMLElement) => {
        // read the nearest named component and the service around it
        const named = part.closest<HTMLElement>("[data-component]")?.dataset["component"];
        const component = present(named, "inspected component");
        const service = part.closest<HTMLElement>("[data-service]")?.dataset["service"];
        const serviceName =
            service === undefined ? undefined : properties.services[Number(service) - 1];

        return isOpenFor(part) ? (serviceName ?? component) : `${serviceName ?? component}, rented`;
    };

    // count the shown part's resizes so the box follows a part that eases to a new size
    const [resizes, setResizes] = createSignal(0);
    createResizeObserver(shown, () => setResizes((count) => count + 1));

    // measure the shown part against the frame with its label
    const box = createMemo(() => {
        // box nothing while no part is shown
        resizes();
        const part = shown();
        if (part === undefined || frame === undefined) {
            return undefined;
        }

        // place the box over the part and hang its label from the nearer side
        const bounds = part.getBoundingClientRect();
        const origin = frame.getBoundingClientRect();

        return {
            left: bounds.left - origin.left,
            top: bounds.top - origin.top,
            width: bounds.width,
            height: bounds.height,
            label: label(part),
            isEnd: bounds.left + bounds.width / 2 > origin.left + origin.width / 2,
            isOpen: isOpenFor(part),
        };
    });

    // keep the last box in place while it fades out
    let last = { left: 0, top: 0, width: 0, height: 0, label: "", isEnd: false, isOpen: false };
    const placed = createMemo(() => {
        last = box() ?? last;

        return last;
    });

    return (
        <div
            ref={frame}
            onPointerMove={point}
            onPointerLeave={leave}
            {...style.attrs(styles.frame)}
        >
            {properties.children}
            <span
                aria-hidden="true"
                style={{
                    left: `${placed().left}px`,
                    top: `${placed().top}px`,
                    width: `${placed().width}px`,
                    height: `${placed().height}px`,
                }}
                {...style.attrs(
                    styles.box,
                    placed().isOpen && styles.boxOpen,
                    box() === undefined && styles.boxGone,
                )}
            >
                <span
                    {...style.attrs(
                        styles.tag,
                        placed().isEnd && styles.tagEnd,
                        placed().isOpen && styles.tagOpen,
                    )}
                >
                    {placed().label}
                </span>
            </span>
        </div>
    );
}

/** The inspector styles. */
const styles = style.create({
    frame: {
        height: "100%",
        minHeight: 0,
        position: "relative",
    },
    box: {
        borderRadius: "4px",
        transitionDuration: "200ms",
        transitionProperty: "opacity, left, top, width, height",
        transitionTimingFunction: "ease",
        outlineColor: color.mutedForeground,
        outlineOffset: "1px",
        outlineStyle: "dashed",
        outlineWidth: "1px",
        pointerEvents: "none",
        position: "absolute",
        zIndex: 3,
    },
    boxGone: {
        opacity: 0,
    },
    boxOpen: {
        backgroundColor: `color-mix(in srgb, ${palette.signal} 8%, transparent)`,
        outlineColor: palette.signal,
        outlineStyle: "solid",
        outlineWidth: "1.5px",
    },
    tag: {
        backgroundColor: color.muted,
        borderRadius: "3px",
        bottom: "calc(100% + 4px)",
        color: color.mutedForeground,
        fontFamily: font.code,
        fontSize: "0.625rem",
        left: "-1px",
        lineHeight: "1rem",
        paddingInline: "0.3125rem",
        position: "absolute",
        whiteSpace: "nowrap",
    },
    tagEnd: {
        left: "auto",
        right: "-1px",
    },
    tagOpen: {
        backgroundColor: palette.signal,
        color: palette.ink,
        fontWeight: 600,
    },
});
