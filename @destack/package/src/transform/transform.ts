import MagicString from "magic-string";
import { parseAst } from "rolldown/parseAst";
import { type ESTree, Visitor } from "rolldown/utils";
import { ModuleMetadata } from "../definition/metadata.ts";
import type { ModulePackage, PackageLocator } from "./locator.ts";

/** The module-local binding holding injected metadata. */
const BINDING = "__destackModule";

/** The source of each static import or re-export. */
const IMPORT_SOURCE = /\bfrom\s*["']([^"']+)["']/gu;

/** A transformed module and the edits that map it to its authored text. */
export interface ModuleTransform {
    /** The transformed source text. */
    readonly code: string;
    /** The edits generating its source map. */
    readonly source: MagicString;
}

/** Inject module metadata into import.meta.destack and declaration constructor calls. */
export function transformModule(
    code: string,
    path: string,
    owner: ModulePackage,
    packages: PackageLocator,
): ModuleTransform | undefined {
    // skip modules without metadata reads or imported constructors
    if (!isStampable(code, path, owner.metadata, packages)) {
        return undefined;
    }

    // collect local bindings of constructors and namespaces imported from their defining package
    const program = parseAst(code, { lang: /\.[cm]?tsx$/u.test(path) ? "tsx" : "ts" }, path);
    const constructors = new Map<string, number>();
    const namespaces = new Map<string, ReadonlyMap<string, number>>();
    for (const statement of program.body) {
        if (statement.type !== "ImportDeclaration" || statement.importKind === "type") {
            continue;
        }
        const exported = packages.imported(statement.source.value, path, owner.metadata);
        for (const specifier of statement.specifiers) {
            // read the module parameter of a named value import
            const position =
                specifier.type === "ImportSpecifier" &&
                specifier.importKind !== "type" &&
                specifier.imported.type === "Identifier"
                    ? exported.get(specifier.imported.name)
                    : undefined;

            // bind named constructor imports
            if (position !== undefined) {
                constructors.set(specifier.local.name, position);
            }
            // bind namespaces of packages with constructors
            else if (specifier.type === "ImportNamespaceSpecifier" && exported.size > 0) {
                namespaces.set(specifier.local.name, exported);
            }
        }
    }

    // replace metadata reads and append metadata to constructor calls that omit it
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
        // append the module to constructor calls that omit it
        CallExpression(node) {
            if (stampCall(node, constructors, namespaces, source)) {
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

/** Report whether a module reads its metadata or imports a constructor, without parsing it. */
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

    // name a constructor one of its import sources provides
    for (const [, specifier] of code.matchAll(IMPORT_SOURCE)) {
        if (specifier === undefined) {
            throw new TypeError("the import source pattern captures no specifier");
        }
        const constructors = packages.imported(specifier, path, metadata);
        if ([...constructors.keys()].some((name) => code.includes(name))) {
            return true;
        }
    }

    return false;
}

/** Append the module binding to a constructor call, padding omitted arguments, reporting whether it did. */
function stampCall(
    node: ESTree.CallExpression,
    constructors: ReadonlyMap<string, number>,
    namespaces: ReadonlyMap<string, ReadonlyMap<string, number>>,
    source: MagicString,
): boolean {
    // skip other calls, calls passing their module explicitly, and spread arguments
    const position = findConstructor(node.callee, constructors, namespaces);
    const values = node.arguments;
    if (
        position === undefined ||
        values.length > position ||
        values.some((value) => value.type === "SpreadElement")
    ) {
        return false;
    }

    // place the binding at its parameter position
    const padding = Array.from({ length: position - values.length }, () => "undefined");
    const appended = [...padding, BINDING].join(", ");
    const last = values.at(-1);

    // append after the last argument
    if (last) {
        source.appendLeft(last.end, `, ${appended}`);
    }
    // insert into empty argument lists before the closing parenthesis
    else {
        source.appendLeft(node.end - 1, appended);
    }

    return true;
}

/** Find the module parameter position of the imported constructor a callee names. */
function findConstructor(
    callee: ESTree.Expression,
    constructors: ReadonlyMap<string, number>,
    namespaces: ReadonlyMap<string, ReadonlyMap<string, number>>,
): number | undefined {
    // read a named import
    if (callee.type === "Identifier") {
        return constructors.get(callee.name);
    }
    // read a namespace member
    else if (
        callee.type === "MemberExpression" &&
        !callee.computed &&
        callee.object.type === "Identifier" &&
        callee.property.type === "Identifier"
    ) {
        return namespaces.get(callee.object.name)?.get(callee.property.name);
    }

    return undefined;
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
