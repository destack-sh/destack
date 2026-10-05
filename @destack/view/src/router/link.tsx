import { useNavigate } from "@solidjs/router";
import type { JSX } from "@solidjs/web";
import type { ObjectReference } from "@destack/sync";
import { urlOf } from "../page/view.ts";
import { omit } from "../solid/reactive.ts";

/** Where a link goes: a path or URL, or an object opened in the view presenting it. */
export type LinkTarget = string | ObjectReference;

/** The properties of a link, an anchor's properties included. */
export interface LinkProperties extends Omit<JSX.AnchorHTMLAttributes<HTMLAnchorElement>, "href"> {
    /** Where the link goes: a path or URL, or an object opened in the strongest view presenting it. */
    readonly href: LinkTarget;
    /** The view opening an object, the strongest presenting its type by default. */
    readonly view?: string;
    /** Replace the current history entry instead of adding one. */
    readonly replace?: boolean;
    /** Preload the route's code and data on hover and focus, true by default. */
    readonly prefetch?: boolean;
    /** Animate a navigation within the page with a view transition, true by default. */
    readonly viewTransition?: boolean;
}

/** Link to a path, a URL or an object, navigating within the page through the router. */
export function Link(properties: LinkProperties): JSX.Element {
    // resolve the target: a path or URL as given, an object through the open path
    const navigate = useNavigate();
    const isObject = () => typeof properties.href !== "string";
    const href = () =>
        typeof properties.href === "string"
            ? properties.href
            : urlOf(
                  properties.href,
                  properties.view === undefined ? {} : { view: properties.view },
              );
    const rest = omit(
        properties,
        "href",
        "view",
        "replace",
        "prefetch",
        "viewTransition",
        "onClick",
    );

    // animate a plain navigation within the page after the caller's handler
    const click = (event: MouseEvent & { currentTarget: HTMLAnchorElement; target: Element }) => {
        // run the caller's handler first
        runHandler(properties.onClick, event);

        // leave modified clicks, objects and other origins to the router and the browser
        const url = new URL(href(), location.href);
        if (
            event.defaultPrevented ||
            isObject() ||
            properties.viewTransition === false ||
            !isPlain(event) ||
            url.origin !== location.origin ||
            !("startViewTransition" in document)
        ) {
            return;
        }
        event.preventDefault();
        document.startViewTransition(() =>
            navigate(`${url.pathname}${url.search}${url.hash}`, {
                replace: properties.replace === true,
            }),
        );
    };

    return (
        <a
            data-slot="link"
            {...rest}
            href={href()}
            rel={isObject() ? "external" : properties.rel}
            replace={properties.replace === true ? "" : undefined}
            preload={properties.prefetch === false ? "false" : undefined}
            onClick={click}
        />
    );
}

/** Report whether a click is a plain primary click the page navigates for. */
function isPlain(event: MouseEvent): boolean {
    return (
        event.button === 0 && !event.metaKey && !event.ctrlKey && !event.shiftKey && !event.altKey
    );
}

/** Run a caller's click handler, a function or a handler bound to its data. */
function runHandler(
    handler: JSX.EventHandlerUnion<HTMLAnchorElement, MouseEvent> | undefined,
    event: MouseEvent & { currentTarget: HTMLAnchorElement; target: Element },
): void {
    if (typeof handler === "function") {
        handler(event);
    } else if (Array.isArray(handler)) {
        handler[0](handler[1], event);
    }
}
