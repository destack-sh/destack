import type { Element, ElementContent, Properties, Root } from "hast";
import { find, html, type Schema, svg } from "property-information";
import { type Component, createComponent, dynamic, For, type JSX } from "@destack/view";

/** The properties a component receives for one element of a content tree. */
export interface ContentElementProperties {
    /** The element the component stands in for. */
    readonly node: Element;
    /** The element's children, rendered. */
    readonly children?: JSX.Element;
    /** The element's attributes by their HTML or SVG names, such as `class` and `data-label`. */
    readonly [attribute: string]: unknown;
}

/** A component that renders the elements of one tag in a content tree. */
export type ContentComponent = Component<ContentElementProperties>;

/** The components that stand in for elements by tag name, such as `pre` for a code listing. */
export type ContentComponents = Readonly<Partial<Record<string, ContentComponent>>>;

/** The properties of a content tree's rendering. */
export interface ContentProperties {
    /** The tree, as a hast root such as remark and rehype build from Markdown. */
    readonly tree: Root;
    /** The components that stand in for elements by their tag name, plain elements for the rest. */
    readonly components?: ContentComponents;
}

/** Render a content tree, such as converted Markdown, as elements. */
export function Content(properties: ContentProperties): JSX.Element {
    return (
        <For each={properties.tree.children}>
            {(node) => renderNode(node, properties.components ?? {}, html)}
        </For>
    );
}

/** Render one node of a content tree in its attribute space. */
function renderNode(
    node: Root["children"][number],
    components: ContentComponents,
    space: Schema,
): JSX.Element {
    // render text, leave out comments and doctypes, which draw nothing, and refuse raw HTML
    if (node.type === "text") {
        return node.value;
    } else if (node.type === "comment" || node.type === "doctype") {
        return undefined;
    } else if (node.type !== "element") {
        throw new TypeError("content tree is invalid: it has raw HTML its builder left unparsed");
    }

    // read the element's attributes in its space, and render its children in the space they open
    const elementSpace = node.tagName === "svg" ? svg : space;
    const attributes = attributesOf(node.properties, elementSpace);
    const children = renderChildren(
        node.children,
        components,
        node.tagName === "foreignObject" ? html : elementSpace,
    );
    const component = components[node.tagName];

    // stand a component in for the element
    if (component !== undefined) {
        return createComponent(component, { ...attributes, node, children });
    }
    // else render the element itself
    else {
        const Element = dynamic(() => node.tagName);

        return <Element {...attributes}>{children}</Element>;
    }
}

/** Render the children of an element, none for an empty element such as an image. */
function renderChildren(
    children: readonly ElementContent[],
    components: ContentComponents,
    space: Schema,
): JSX.Element {
    return children.length === 0
        ? undefined
        : children.map((child) => renderNode(child, components, space));
}

/** Turn an element's hast properties into HTML or SVG attributes. */
function attributesOf(properties: Properties, space: Schema): Record<string, string> {
    const attributes: Record<string, string> = {};
    for (const [property, value] of Object.entries(properties)) {
        // leave out absent and false attributes
        if (value === undefined || value === null || value === false) {
            continue;
        }

        // write a list as its separated words, a true flag as present, and the rest as text
        const definition = find(space, property);
        attributes[definition.attribute] = Array.isArray(value)
            ? value.join(definition.commaSeparated ? ", " : " ")
            : value === true
              ? ""
              : String(value);
    }

    return attributes;
}
