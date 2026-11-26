import type { DeclarationDescription } from "@destack/package";
import type { BuildExtension, Compilation, Plugin } from "@destack/package/build";
import { OBJECT_KIND } from "../object/description.ts";

/** The entrypoint exporting a package's object types, which its device worker imports. */
export const OBJECTS_ENTRYPOINT = "./objects";

/** The generated module behind the objects entrypoint. */
const OBJECTS_MODULE = "virtual:@destack/object/objects";

/** Export each package's object types from one generated entrypoint of its browser output. */
export const objectExtension: BuildExtension = {
    compile(compilation) {
        // export nothing outside a browser output, or for a package declaring no object types
        const declared = compilation.declarations.filter(
            (declaration) =>
                declaration.symbol.package.id === compilation.package.id &&
                OBJECT_KIND.is(declaration.kind, declaration.package.id),
        );
        if (compilation.runtime !== "browser" || declared.length === 0) {
            return [];
        }

        // emit the entrypoint exporting each type under its declaration name
        compilation.entry(OBJECTS_ENTRYPOINT, OBJECTS_MODULE);

        return [objectsPlugin(declared, compilation)];
    },
};

/** Resolve and write the generated module exporting each object type under its declaration name. */
function objectsPlugin(
    declared: readonly DeclarationDescription[],
    compilation: Pick<Compilation, "locate">,
): Plugin {
    // name each declaring export once
    const values = new Map<string, string>();
    const imports: string[] = [];
    const types = declared.map((declaration) => {
        // locate the declaring export, importing it the first time
        const located = compilation.locate(declaration);
        const key = `${located.file} ${located.export}`;
        const value = values.get(key) ?? `value${String(values.size)}`;
        if (!values.has(key)) {
            values.set(key, value);
            imports.push(
                `import { ${located.export} as ${value} } from ${JSON.stringify(located.file)};`,
            );
        }

        // read the type as the export, or as the member of it holding the type
        const member = declaration.member === undefined ? "" : `.${declaration.member}`;

        return `    ${JSON.stringify(declaration.name)}: ${value}${member},`;
    });

    // write the module once
    const id = `\0${OBJECTS_MODULE}`;
    const source = [...imports, ``, `export const objects = {`, ...types, `};`, ``].join("\n");

    return {
        name: "@destack/object/objects",
        resolveId: (requested) => (requested === OBJECTS_MODULE ? id : null),
        load: (loaded) => (loaded === id ? source : null),
    };
}
