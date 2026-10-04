import { RolldownMagicString } from "rolldown";
import { type ESTree, parseSync, Visitor } from "rolldown/utils";
import type { BuildExtension, Plugin } from "@destack/package/build";

/** The package exporting the icon component and one module per icon under `phosphor/<name>`. */
const ICON_PACKAGE = "@destack/icon";

/** Pass each icon drawn by a literal name its bodies in builds of packages that depend on `@destack/icon`. */
export const iconExtension: BuildExtension = {
    transform: () => [iconPlugin()],
};

/** Pass each icon named by a literal its bodies from a static import of the icon's module. */
export function iconPlugin(): Plugin {
    return {
        name: ICON_PACKAGE,
        enforce: "pre",
        transform: {
            filter: { id: /\.[jt]sx(?:\?|$)/u, code: new RegExp(`["']${ICON_PACKAGE}["']`, "u") },
            handler(code, id) {
                // parse the module
                const [path = id] = id.split("?");
                const parsed = parseSync(path, code);
                if (parsed.errors.length) {
                    throw new TypeError(`invalid icon source: ${id}`);
                }

                // pass each literal-named icon its module's bodies, importing each module once
                const imported = findIconImport(parsed.program);
                const source = new RolldownMagicString(code);
                const modules = new Map<string, string>();
                new Visitor({
                    JSXOpeningElement(element) {
                        // name the import of the element's icon module
                        const name = readIconName(element, imported, id);
                        if (name === undefined) {
                            return;
                        }
                        const local = modules.get(name.value) ?? `__icon_${modules.size}`;
                        modules.set(name.value, local);

                        // pass the bodies after the name
                        source.appendLeft(name.attribute.end, ` icon={${local}}`);
                    },
                }).visit(parsed.program);

                // keep a module without literal-named icons as written
                const named = imported.named;
                if (modules.size === 0 || named === undefined) {
                    return undefined;
                }

                // import the icons' modules after the component's import
                for (const [name, local] of modules) {
                    const specifier = JSON.stringify(`${ICON_PACKAGE}/phosphor/${name}`);
                    source.appendLeft(
                        named.declaration.end,
                        `\nimport ${local} from ${specifier};`,
                    );
                }

                return {
                    code: source.toString(),
                    map: source.generateMap({ source: path, hires: true }).toString(),
                };
            },
        },
    };
}

/** The local names under which a module imports the icon component. */
interface IconImport {
    /** The import of the component by name with its local name, absent without one. */
    readonly named:
        | { readonly declaration: ESTree.ImportDeclaration; readonly component: string }
        | undefined;
    /** The local names of namespace imports of the icon package. */
    readonly namespaces: ReadonlySet<string>;
}

/** Find a module's imports of the icon component: by name, and through namespaces of the package. */
function findIconImport(program: ESTree.Program): IconImport {
    // collect the component's import and the package's namespaces
    let named: IconImport["named"];
    const namespaces = new Set<string>();
    for (const statement of program.body) {
        // select value imports of the icon package
        if (
            statement.type !== "ImportDeclaration" ||
            statement.source.value !== ICON_PACKAGE ||
            statement.importKind === "type"
        ) {
            continue;
        }

        // keep the component's local name and each namespace's
        for (const specifier of statement.specifiers) {
            if (specifier.type === "ImportNamespaceSpecifier") {
                namespaces.add(specifier.local.name);
            } else if (
                specifier.type === "ImportSpecifier" &&
                specifier.importKind !== "type" &&
                (specifier.imported.type === "Literal"
                    ? specifier.imported.value
                    : specifier.imported.name) === "Icon"
            ) {
                named = { declaration: statement, component: specifier.local.name };
            }
        }
    }

    return { named, namespaces };
}

/** Report whether an element draws the icon component, by its local name or through a namespace. */
function drawsIcon(element: ESTree.JSXOpeningElement, imported: IconImport): boolean {
    const name = element.name;
    if (name.type === "JSXIdentifier") {
        return name.name === imported.named?.component;
    } else if (name.type === "JSXMemberExpression") {
        return (
            name.object.type === "JSXIdentifier" &&
            imported.namespaces.has(name.object.name) &&
            name.property.name === "Icon"
        );
    } else {
        return false;
    }
}

/** Read the literal name of an icon element without bodies, refusing one whose bodies the build cannot pass. */
function readIconName(
    element: ESTree.JSXOpeningElement,
    imported: IconImport,
    id: string,
): { readonly value: string; readonly attribute: ESTree.JSXAttribute } | undefined {
    // leave other elements and icons with passed bodies as written
    const hasBodies = element.attributes.some(
        (attribute) => attribute.type === "JSXAttribute" && attribute.name.name === "icon",
    );
    if (!drawsIcon(element, imported) || hasBodies) {
        return undefined;
    }

    // refuse spread properties and namespace members, whose names the build cannot read
    const at = `${id}:${element.start}`;
    const alternatives = `pass icon imported from ${ICON_PACKAGE}/phosphor/<name>, or draw LazyIcon from ${ICON_PACKAGE}/lazy`;
    if (element.attributes.some((attribute) => attribute.type === "JSXSpreadAttribute")) {
        throw new TypeError(`icon with spread properties and no bodies: ${at}: ${alternatives}`);
    } else if (element.name.type === "JSXMemberExpression") {
        throw new TypeError(
            `icon drawn through a namespace import without bodies: ${at}: import Icon by name, ${alternatives}`,
        );
    }

    // read a name written as a string literal, bare or in braces
    const attribute = element.attributes.find(
        (entry): entry is ESTree.JSXAttribute =>
            entry.type === "JSXAttribute" && entry.name.name === "name",
    );
    const value =
        attribute?.value?.type === "JSXExpressionContainer"
            ? attribute.value.expression
            : attribute?.value;
    if (attribute === undefined || value?.type !== "Literal" || typeof value.value !== "string") {
        throw new TypeError(`icon without a literal name or bodies: ${at}: ${alternatives}`);
    }

    return { value: value.value, attribute };
}
