import MagicString from "magic-string";
import { parseAst } from "rolldown/parseAst";
import { ModuleMetadata } from "../definition/metadata.ts";
import type { ModulePackage, PackageLocator } from "./locator.ts";

/** The module-local binding holding injected metadata. */
const BINDING = "__destackModule";

/** The source of each static import or re-export. */
const IMPORT_SOURCE = /\bfrom\s*["']([^"']+)["']/g;

/** A transformed module and the edits that map it to its authored text. */
export interface ModuleTransform {
    /** The transformed source text. */
    readonly code: string;
    /** The edits generating its source map. */
    readonly source: MagicString;
}

/** A parsed AST node with source offsets. */
interface Node {
    /** The node type name. */
    readonly type: string;
    /** The offset of the first character. */
    readonly start: number;
    /** The offset after the last character. */
    readonly end: number;
    readonly [key: string]: unknown;
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
    const program = parseAst(code, { lang: /\.[cm]?tsx$/.test(path) ? "tsx" : "ts" }, path);
    const constructors = new Map<string, number>();
    const namespaces = new Map<string, Readonly<Record<string, number>>>();
    for (const statement of program.body) {
        if (statement.type !== "ImportDeclaration" || statement.importKind === "type") {
            continue;
        }
        const exported = packages.imported(statement.source.value, path, owner.metadata);
        for (const specifier of statement.specifiers) {
            // bind named constructor imports
            if (
                specifier.type === "ImportSpecifier" &&
                specifier.importKind !== "type" &&
                specifier.imported.type === "Identifier" &&
                Object.hasOwn(exported, specifier.imported.name)
            ) {
                constructors.set(specifier.local.name, exported[specifier.imported.name]!);
            }
            // bind namespaces of packages with constructors
            else if (
                specifier.type === "ImportNamespaceSpecifier" &&
                Object.keys(exported).length > 0
            ) {
                namespaces.set(specifier.local.name, exported);
            }
        }
    }

    // replace metadata reads and append metadata to constructor calls that omit it
    const source = new MagicString(code);
    let isChanged = false;
    visit(program, (node) => {
        // replace metadata reads with the module binding
        if (isMetadataRead(node)) {
            source.overwrite(node.start, node.end, BINDING);
            isChanged = true;
        }
        // append the module to constructor calls that omit it
        else if (stampCall(node, constructors, namespaces, source)) {
            isChanged = true;
        }
    });
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
        const constructors = packages.imported(specifier!, path, metadata);
        if (Object.keys(constructors).some((name) => code.includes(name))) {
            return true;
        }
    }

    return false;
}

/** Append the module binding to a constructor call, padding omitted arguments, reporting whether it did. */
function stampCall(
    node: Node & Record<string, any>,
    constructors: ReadonlyMap<string, number>,
    namespaces: ReadonlyMap<string, Readonly<Record<string, number>>>,
    source: MagicString,
): boolean {
    // skip other calls, calls passing their module explicitly, and spread arguments
    const position =
        node.type === "CallExpression"
            ? findConstructor(node.callee, constructors, namespaces)
            : undefined;
    const values = (node.arguments ?? []) as Node[];
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
    callee: Node & Record<string, any>,
    constructors: ReadonlyMap<string, number>,
    namespaces: ReadonlyMap<string, Readonly<Record<string, number>>>,
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
        const exported = namespaces.get(callee.object.name);

        return exported && Object.hasOwn(exported, callee.property.name)
            ? exported[callee.property.name]
            : undefined;
    }

    return undefined;
}

/** Report whether a node reads import.meta.destack. */
function isMetadataRead(node: Node): boolean {
    const object = node.object as Node | undefined;
    const property = node.property as (Node & { name?: string }) | undefined;

    return (
        node.type === "MemberExpression" &&
        object?.type === "MetaProperty" &&
        property?.type === "Identifier" &&
        property.name === "destack"
    );
}

/** Visit every AST node in source order, skipping children of rewritten metadata reads. */
function visit(node: unknown, callback: (node: Node & Record<string, any>) => void): void {
    // descend through arrays of nodes
    if (Array.isArray(node)) {
        for (const child of node) {
            visit(child, callback);
        }

        return;
    }

    // ignore scalar values
    if (!node || typeof node !== "object" || typeof (node as Node).type !== "string") {
        return;
    }

    // visit the node before its children
    callback(node as Node & Record<string, any>);
    if (isMetadataRead(node as Node)) {
        return;
    }

    // descend into child nodes
    for (const key in node) {
        const value = (node as Record<string, unknown>)[key];
        if (value && typeof value === "object") {
            visit(value, callback);
        }
    }
}
