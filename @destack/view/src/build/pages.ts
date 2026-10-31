import { RolldownMagicString } from "rolldown";
import { type ESTree, parseSync, Visitor } from "rolldown/utils";
import type { Plugin } from "@destack/package/build";
import { ViewError } from "../error/error.ts";
import { VIEW_PACKAGE } from "./plugin.ts";

/** The properties of a static view's render that only its server reads. */
const PAGES_PROPERTIES = new Set(["origin", "pages"]);

/** Remove static views' origins and pages modules from browser modules, as Next.js does. */
export function pagesPlugin(): Plugin {
    return {
        name: `${VIEW_PACKAGE}/pages`,
        transform: {
            filter: { id: /\.[cm]?[jt]sx?(?:\?|$)/u, code: /defineView/u },
            handler(code, id, options) {
                // leave server modules, which render the pages
                const [path = id] = id.split("?");

                return options?.ssr === true ? undefined : removePages(code, path);
            },
        },
    };
}

/** Remove the origins and pages modules a module's static views declare, if any. */
export function removePages(
    code: string,
    path: string,
): { readonly code: string; readonly map: string } | undefined {
    // parse the module
    const parsed = parseSync(path, code);
    if (parsed.errors.length) {
        throw ViewError.of("SOURCE_INVALID", `invalid view source: ${path}`);
    }

    // find the server properties of each static view the module declares
    const removed = pagesProperties(parsed.program);
    if (removed.length === 0) {
        return undefined;
    }

    // remove each property with the comma after it
    const source = new RolldownMagicString(code);
    for (const property of removed) {
        const end = code[property.end] === "," ? property.end + 1 : property.end;
        source.remove(property.start, end);
    }

    return {
        code: source.toString(),
        map: source.generateMap({ source: path, hires: true }).toString(),
    };
}

/** List the server properties of each static view a module declares inline. */
function pagesProperties(program: ESTree.Program): ESTree.Node[] {
    const removed: ESTree.Node[] = [];
    new Visitor({
        CallExpression(node) {
            // select the render of each view declared with an inline definition
            const [definition] = node.arguments;
            const isDeclared =
                node.callee.type === "Identifier" &&
                node.callee.name === "defineView" &&
                definition?.type === "ObjectExpression";
            const render = isDeclared ? propertyOf(definition, "render") : undefined;

            // select the server properties of a static render
            if (render?.type === "ObjectExpression" && isStatic(render)) {
                const site = render.properties.filter((property) => {
                    const key = property.type === "Property" ? keyOf(property) : undefined;

                    return key !== undefined && PAGES_PROPERTIES.has(key);
                });
                removed.push(...site);
            }
        },
    }).visit(program);

    return removed;
}

/** Read the value of an object literal's property, absent when it has none. */
function propertyOf(object: ESTree.ObjectExpression, name: string): ESTree.Node | undefined {
    const found = object.properties.find(
        (property) => property.type === "Property" && keyOf(property) === name,
    );

    return found?.type === "Property" ? found.value : undefined;
}

/** Report whether a render object literal states the static mode. */
function isStatic(render: ESTree.ObjectExpression): boolean {
    const mode = propertyOf(render, "mode");

    return mode?.type === "Literal" && mode.value === "static";
}

/** Read a property's name, absent for a computed one. */
function keyOf(property: ESTree.ObjectProperty): string | undefined {
    const { key } = property;
    // name a plain key by itself
    if (!property.computed && key.type === "Identifier") {
        return key.name;
    }
    // name a quoted key by its text
    else if (!property.computed && key.type === "Literal") {
        return String(key.value);
    }
    // name no computed key
    else {
        return undefined;
    }
}
