import MagicString from "magic-string";
import { parseAst } from "rolldown/parseAst";
import { type ESTree, Visitor } from "rolldown/utils";
import { ModuleMetadata } from "../definition/metadata.ts";
import type { ModulePackage, PackageLocator } from "./locator.ts";

/** The module-local binding holding injected metadata. */
const BINDING = "__destackModule";

/** The source of each static import or re-export. */
const IMPORT_SOURCE = /\bfrom\s*["']([^"']+)["']/gu;

/** A local import binding of a package with stamped functions. */
interface Binding {
    /** The module parameter positions of the package's stamped functions, by export path. */
    readonly stamps: ReadonlyMap<string, number>;
    /** The export path the binding names, empty for a namespace. */
    readonly path: readonly string[];
}

/** A transformed module and the edits that map it to its authored text. */
export interface ModuleTransform {
    /** The transformed source text. */
    readonly code: string;
    /** The edits generating its source map. */
    readonly source: MagicString;
}

/** Inject module metadata into import.meta.destack and the calls and tags of stamped functions. */
export function transformModule(
    code: string,
    path: string,
    owner: ModulePackage,
    packages: PackageLocator,
): ModuleTransform | undefined {
    // skip modules without metadata reads or imported stamped functions
    if (!isStampable(code, path, owner.metadata, packages)) {
        return undefined;
    }

    // collect the local bindings of imports from packages with stamped functions
    const program = parseAst(code, { lang: /\.[cm]?tsx$/u.test(path) ? "tsx" : "ts" }, path);
    const bindings = new Map<string, Binding>();
    for (const statement of program.body) {
        if (statement.type !== "ImportDeclaration" || statement.importKind === "type") {
            continue;
        }
        const stamps = packages.imported(statement.source.value, path, owner.metadata);
        if (stamps.size === 0) {
            continue;
        }
        for (const specifier of statement.specifiers) {
            // bind named value imports by their exported name
            if (
                specifier.type === "ImportSpecifier" &&
                specifier.importKind !== "type" &&
                specifier.imported.type === "Identifier"
            ) {
                bindings.set(specifier.local.name, { stamps, path: [specifier.imported.name] });
            }
            // bind namespaces by their members
            else if (specifier.type === "ImportNamespaceSpecifier") {
                bindings.set(specifier.local.name, { stamps, path: [] });
            }
        }
    }

    // replace metadata reads and pass the module to stamped calls and tags that omit it
    const source = new MagicString(code);
    let isChanged = false;
    new Visitor({
        // replace metadata reads with the module binding
        MemberExpression(node) {
            if (isMetadataRead(node)) {
                source.overwrite(node.start, node.end, BINDING);
                isChanged = true;
            }
        },
        // append the module to stamped calls that omit it
        CallExpression(node) {
            if (stampCall(node, bindings, source)) {
                isChanged = true;
            }
        },
        // call a stamped tag with the module, its result tagging the template
        TaggedTemplateExpression(node) {
            const position = findStamp(node.tag, bindings);
            if (position !== undefined) {
                source.appendLeft(node.tag.end, `(${moduleArguments(position, 0)})`);
                isChanged = true;
            }
        },
    }).visit(program);
    if (!isChanged) {
        return undefined;
    }

    // declare the frozen binding before any module code runs
    source.prepend(`const ${BINDING} = Object.freeze(${JSON.stringify(owner.metadata)});\n`);

    return { code: source.toString(), source };
}

/** Report whether a module reads its metadata or names a stamped function, without parsing it. */
function isStampable(
    code: string,
    path: string,
    metadata: ModuleMetadata,
    packages: PackageLocator,
): boolean {
    // read the metadata
    if (code.includes("import.meta.destack")) {
        return true;
    }

    // name the root of a stamped function one of its import sources provides
    for (const [, specifier] of code.matchAll(IMPORT_SOURCE)) {
        if (specifier === undefined) {
            throw new TypeError("the import source pattern captures no specifier");
        }
        const stamps = packages.imported(specifier, path, metadata);
        if ([...stamps.keys()].some((name) => code.includes(name.split(".", 1)[0] ?? name))) {
            return true;
        }
    }

    return false;
}

/** Append the module binding to a stamped call, padding omitted arguments, reporting whether it did. */
function stampCall(
    node: ESTree.CallExpression,
    bindings: ReadonlyMap<string, Binding>,
    source: MagicString,
): boolean {
    // skip other calls, calls passing their module explicitly, and spread arguments
    const position = findStamp(node.callee, bindings);
    const values = node.arguments;
    if (
        position === undefined ||
        values.length > position ||
        values.some((value) => value.type === "SpreadElement")
    ) {
        return false;
    }

    // append after the last argument
    const appended = moduleArguments(position, values.length);
    const last = values.at(-1);
    if (last) {
        source.appendLeft(last.end, `, ${appended}`);
    }
    // insert into empty argument lists before the closing parenthesis
    else {
        source.appendLeft(node.end - 1, appended);
    }

    return true;
}

/** Find the module parameter position of the stamped function an imported name or its member chain names. */
function findStamp(
    expression: ESTree.Expression,
    bindings: ReadonlyMap<string, Binding>,
): number | undefined {
    // collect the member names down to the root identifier
    const members: string[] = [];
    let current = expression;
    while (
        current.type === "MemberExpression" &&
        !current.computed &&
        current.property.type === "Identifier"
    ) {
        members.unshift(current.property.name);
        current = current.object;
    }
    if (current.type !== "Identifier") {
        return undefined;
    }

    // resolve the root's import to the export path
    const binding = bindings.get(current.name);

    return binding?.stamps.get([...binding.path, ...members].join("."));
}

/** Write the arguments placing the module binding at its parameter position after the given ones. */
function moduleArguments(position: number, given: number): string {
    const padding = Array.from({ length: position - given }, () => "undefined");

    return [...padding, BINDING].join(", ");
}

/** Report whether a member expression reads import.meta.destack. */
function isMetadataRead(node: ESTree.MemberExpression): boolean {
    return (
        !node.computed &&
        node.object.type === "MetaProperty" &&
        node.property.type === "Identifier" &&
        node.property.name === "destack"
    );
}
