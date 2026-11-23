import * as style from "@destack/style";
import { dynamic, type JSX, merge, omit, untrack } from "@destack/view";
import { access, type MaybeAccessor } from "@destack/view/primitives/utils";

/** The key a part's styles stand under for a kit part its caller renders, which composes them. */
export const PART_STYLES = Symbol("part styles");

/** The attributes a part gives the element it renders, and the reference that takes the element. */
export interface PartAttributes
    extends Partial<Record<"ref", (element: HTMLElement) => void>>, JSX.AriaAttributes {
    /** The atomic class names of the part's styles. */
    readonly class?: string | undefined;
    /** The inline style: the part's dynamic values, then the caller's. */
    readonly style?: string | undefined;
    /** The part's StyleX styles, which a kit part composes with the kit's in place of the class. */
    readonly [PART_STYLES]?: style.Styles;
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
    /** The part's name and state for styling, such as `data-slot` and `data-open`. */
    readonly [data: `data-${string}`]: string | undefined;
}

/** An event on the element a part renders, whichever element its caller chose. */
export type PartEvent<Base extends Event> = Base & { readonly currentTarget: HTMLElement };

/** Render another element with a part's attributes, such as a link in place of a button. */
export type Render = (attributes: PartAttributes) => JSX.Element;

/** The attributes besides `aria-*` and `data-*` that a part forwards to its caller's element. */
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

/** Read the attributes of a part's properties that any element takes. */
function forwarded(source: object): PartAttributes {
    return new Proxy<PartAttributes>(
        {},
        {
            get: (_target, key): unknown =>
                isForwarded(key) ? Reflect.get(source, key) : undefined,
            has: (_target, key) => isForwarded(key) && Reflect.has(source, key),
            ownKeys: () => Reflect.ownKeys(source).filter(isForwarded),
            getOwnPropertyDescriptor: (_target, key) =>
                isForwarded(key) && Reflect.has(source, key)
                    ? {
                          enumerable: true,
                          configurable: true,
                          get: (): unknown => Reflect.get(source, key),
                      }
                    : undefined,
        },
    );
}

/** Merge a part's attributes under its caller's, the caller's winning. */
export function mergeProperties<Part extends object, Caller extends object>(
    part: Part,
    caller: Caller,
) {
    // chain the handlers and references both sides set
    const chained = {};
    const keys = untrack(() =>
        Object.keys(part).filter((name) => isChained(name) && name in caller),
    );
    for (const key of keys) {
        Object.defineProperty(chained, key, {
            enumerable: true,
            get: () => chain(Reflect.get(caller, key), Reflect.get(part, key)),
        });
    }

    return merge(part, caller, chained);
}

/** The attributes an element part gives its own element besides its slot and styles. */
export type ElementAttributes<Tag extends keyof JSX.IntrinsicElements> = Omit<
    JSX.IntrinsicElements[Tag],
    "class" | "style"
> & { readonly [data: `data-${string}`]: string | number | undefined };

/** What `useRender` renders: the caller's element, or the part's own element of a tag. */
export interface UseRenderOptions {
    /** Render another element with the part's attributes, the part's own element by default. */
    readonly render: Render | undefined;
    /** The tag of the part's own element. */
    readonly defaultTagName: keyof JSX.IntrinsicElements;
    /** The part's attributes, such as its slot, state and styles. */
    readonly part: object;
    /** The caller's other attributes, all of which the part's own element takes. */
    readonly properties: object;
}

/** The properties every element part takes besides its element's attributes. */
export interface ElementPartProperties {
    /** The StyleX styles applied after the part's styles. */
    readonly xstyle?: style.Styles;
    /** The inline style applied after the part's dynamic values. */
    readonly style?: JSX.HTMLAttributes<HTMLElement>["style"];
    /** Render another element with the part's attributes, the part's own element by default. */
    readonly render?: Render;
    /** An outer part's class, which its styles under `PART_STYLES` stand in for. */
    readonly class?: string | undefined;
    /** An outer part's styles, composed between the part's styles and the caller's. */
    readonly [PART_STYLES]?: style.Styles;
}

/** Render a part's own element of a tag, or the element its caller renders, with its attributes. */
export function useRender(options: UseRenderOptions): JSX.Element {
    // render the caller's element with the attributes any element takes
    if (options.render !== undefined) {
        return options.render(mergeProperties(options.part, forwarded(options.properties)));
    }

    // render the part on its default tag
    const Element = dynamic(() => options.defaultTagName);

    return <Element {...mergeProperties(options.part, options.properties)} />;
}

/** Render an element part on its own tag or the element its caller renders. */
export function renderPart<Tag extends keyof JSX.IntrinsicElements>(
    tag: Tag,
    slot: string,
    properties: ElementPartProperties & Omit<JSX.IntrinsicElements[Tag], "class">,
    styles: MaybeAccessor<style.Styles>,
    attributes?: ElementAttributes<Tag>,
): JSX.Element {
    // refuse a class that comes without an outer part's styles, which it stands for
    if (untrack(() => properties.class !== undefined && properties[PART_STYLES] === undefined)) {
        throw new TypeError(
            `part '${slot}' is invalid: a class comes only with an outer part's styles`,
        );
    }

    // mark the part and give it its styles, then an outer part's it renders for, then the caller's
    const rest = omit(properties, "xstyle", "style", "render", "class", PART_STYLES);
    const composed = (): style.Styles => [
        access(styles),
        properties[PART_STYLES],
        properties.xstyle,
    ];
    const part = merge({ "data-slot": slot }, attributes ?? {}, () => ({
        ...style.attributes(composed(), properties.style),
        [PART_STYLES]: composed(),
    }));

    // read the caller's element once
    const render = untrack(() => properties.render);

    return useRender({ render, defaultTagName: tag, part, properties: rest });
}

/** Run two handlers in sequence, or take the one that is a function. */
function chain(first: unknown, second: unknown): unknown {
    // take a lone handler
    if (!isHandler(first) || !isHandler(second)) {
        return isHandler(first) ? first : second;
    }

    return (...values: unknown[]) => {
        first(...values);
        second(...values);
    };
}

/** Report whether a value is a handler or a reference callback. */
function isHandler(value: unknown): value is (...values: unknown[]) => void {
    return typeof value === "function";
}

/** Report whether a key names an event handler or a reference, which merged attributes chain. */
function isChained(key: string): boolean {
    return key === "ref" || /^on[A-Z]/u.test(key);
}
