import { color } from "@destack/theme/tokens.stylex";
import { present } from "@destack/schema";
import * as stylex from "@destack/style";
import { createMemo, createSignal, type JSX } from "@destack/view";

import { tokens } from "../style/tokens.stylex";
import type { Lighting } from "./ledger";

/**
 * Inspect an app figure the way browser tools do: point at any part marked `data-component` to see what it is.
 *
 * Once owned, a part shows its outline, its component and the service it runs on, and lights that service in the ledger.
 * While rented, a part shows only a faint dashed outline and the minified markup of a closed build.
 */
export function Inspector(properties: {
    isOpen: boolean;
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
        const part = target?.closest<HTMLElement>("[data-component]") ?? undefined;
        const service = target?.closest<HTMLElement>("[data-service]")?.dataset["service"];

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

    // name the shown part by its component and service once owned and by its markup while rented
    const label = (part: HTMLElement) => {
        // read the nearest named component and the service around it
        const named = part.closest<HTMLElement>("[data-component]")?.dataset["component"];
        const component = present(named, "inspected component");
        const service = part.closest<HTMLElement>("[data-service]")?.dataset["service"];
        const serviceName =
            service === undefined ? undefined : properties.services[Number(service) - 1];

        return properties.isOpen
            ? [component, serviceName].filter((name) => name !== undefined).join(" · ")
            : `${part.tagName.toLowerCase()}.${minified(component)}`;
    };

    // measure the shown part against the frame with its label
    const box = createMemo(() => {
        // box nothing while no part is shown
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
        };
    });

    // keep the last box in place while it fades out
    let last = { left: 0, top: 0, width: 0, height: 0, label: "", isEnd: false };
    const placed = createMemo(() => {
        last = box() ?? last;

        return last;
    });

    return (
        <div
            ref={frame}
            onPointerMove={point}
            onPointerLeave={leave}
            {...stylex.attrs(styles.frame)}
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
                {...stylex.attrs(
                    styles.box,
                    properties.isOpen && styles.boxOpen,
                    box() === undefined && styles.boxGone,
                )}
            >
                <span
                    {...stylex.attrs(
                        styles.tag,
                        placed().isEnd && styles.tagEnd,
                        properties.isOpen && styles.tagOpen,
                    )}
                >
                    {placed().label}
                </span>
            </span>
        </div>
    );
}

/** Return a short, stable class name for a component, as a minifier would print it. */
function minified(component: string) {
    let hash = 7;
    for (const character of component) {
        hash = (hash * 31 + character.charCodeAt(0)) >>> 0;
    }

    return hash.toString(36).slice(-6);
}

/** The inspector styles. */
const styles = stylex.create({
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
        backgroundColor: "rgb(255 121 46 / 8%)",
        outlineColor: tokens.signal,
        outlineStyle: "solid",
        outlineWidth: "1.5px",
    },
    tag: {
        backgroundColor: color.muted,
        borderRadius: "3px",
        bottom: "calc(100% + 4px)",
        color: color.mutedForeground,
        fontFamily: tokens.monoFont,
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
        backgroundColor: tokens.signal,
        color: tokens.signalInk,
        fontWeight: 600,
    },
});
