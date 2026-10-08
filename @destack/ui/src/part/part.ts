import { type JSX, merge } from "@destack/view";

/** The attributes a part gives the element it renders, which a caller's `render` spreads onto an element of its own, and the reference that takes the element. */
export interface PartAttributes extends Partial<Record<"ref", (element: HTMLElement) => void>> {
    /** The atomic class names of the part's styles. */
    readonly class?: string | undefined;
    /** The inline style: the part's dynamic values, then the caller's. */
    readonly style?: string | undefined;
    /** The element's id, such as the one a label or `aria-controls` names. */
    readonly id?: string | undefined;
    /** The address a link part points at. */
    readonly href?: string | undefined;
    /** The element's ARIA role. */
    readonly role?: JSX.HTMLAttributes<HTMLElement>["role"];
    /** Whether the element ignores the person, on elements that disable. */
    readonly disabled?: boolean | undefined;
    /** The element's place in the tab order. */
    readonly tabindex?: number | undefined;
    /** Handle a click on the element. */
    readonly onClick?: (event: PartEvent<MouseEvent>) => void;
    /** Handle the element taking the focus. */
    readonly onFocus?: (event: PartEvent<FocusEvent>) => void;
    /** Handle a key the element receives. */
    readonly onKeyDown?: (event: PartEvent<KeyboardEvent>) => void;
    /** Handle a pointer moving onto the element. */
    readonly onPointerEnter?: (event: PartEvent<PointerEvent>) => void;
    /** Handle a pointer leaving the element. */
    readonly onPointerLeave?: (event: PartEvent<PointerEvent>) => void;
    /** Handle the element losing the focus. */
    readonly onBlur?: (event: PartEvent<FocusEvent>) => void;
    /** The part's content. */
    readonly children?: JSX.Element;
    /** The part's name and state for styling, such as `data-slot` and `data-state`. */
    readonly [data: `data-${string}`]: string | undefined;
    /** The part's ARIA states and relations, such as `aria-expanded` and `aria-controls`. */
    readonly [aria: `aria-${string}`]: string | undefined;
}

/** An event on the element a part renders, whichever element its caller chose. */
export type PartEvent<Base extends Event> = Base & { readonly currentTarget: HTMLElement };

/** Render another element with a part's attributes, such as a link in place of a button. */
export type Render = (attributes: PartAttributes) => JSX.Element;

/** The attributes besides `aria-*` and `data-*` that a part forwards to an element its caller renders, its content included. */
const FORWARDED: ReadonlySet<string> = new Set([
    "id",
    "href",
    "role",
    "tabindex",
    "onClick",
    "onFocus",
    "onKeyDown",
    "onPointerEnter",
    "onPointerLeave",
    "onBlur",
    "children",
]);

/** Report whether an attribute is one a part forwards to an element its caller renders. */
function isForwarded(key: string | symbol): key is string {
    return (
        typeof key === "string" &&
        (key.startsWith("aria-") || key.startsWith("data-") || FORWARDED.has(key))
    );
}

/** Read the attributes of a part's properties that any element takes, following the properties as they change. */
export function forwarded(source: object): PartAttributes {
    return new Proxy<PartAttributes>(
        {},
        {
            get: (_target, key): unknown =>
                isForwarded(key) ? Reflect.get(source, key) : undefined,
            has: (_target, key) => isForwarded(key) && Reflect.has(source, key),
            ownKeys: () => Reflect.ownKeys(source).filter(isForwarded),
            getOwnPropertyDescriptor: (_target, key) => {
                // describe a forwarded attribute the properties hold as a plain value
                if (!isForwarded(key) || !Reflect.has(source, key)) {
                    return undefined;
                }
                const value: unknown = Reflect.get(source, key);

                return { enumerable: true, configurable: true, value };
            },
        },
    );
}

/** Render a part's own element, or the element its caller renders with the part's attributes and the forwarded ones of its properties. */
export function rendered(
    render: Render | undefined,
    part: PartAttributes,
    properties: object,
    own: () => JSX.Element,
): JSX.Element {
    return render === undefined ? own() : render(merge(forwarded(properties), part));
}
