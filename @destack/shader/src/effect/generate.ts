import { writeFile } from "node:fs/promises";
import { shaderRegistry } from "shaders/registry";

/** The first line of the generated module. */
const HEADER = "// generate with `bun run generate` from the shaders effect registry";

/** Name an effect's module namespace in camel case, such as linearGradient for LinearGradient. */
function namespaceOf(name: string): string {
    return `${name.charAt(0).toLowerCase()}${name.slice(1)}`;
}

/** Write one documentation sentence from an effect's description, ending in a period. */
function sentenceOf(description: string | undefined, name: string): string {
    const [first = `Draw the ${name} effect`] = (description ?? "").trim().split(/(?<=\.)\s+/u);
    const text = (first === "" ? `Draw the ${name} effect` : first).replaceAll("*/", "* /");

    return text.endsWith(".") ? text : `${text}.`;
}

/** The effects in name order. */
const effects = shaderRegistry.toSorted((left, right) => left.name.localeCompare(right.name));
/** The type import of each effect's module, its properties among them. */
const imports = effects.map(
    (effect) => `import type * as ${namespaceOf(effect.name)} from "shaders/core/${effect.name}";`,
);
/** The component of each effect, typed by its module's properties. */
const components = effects.map(
    (effect) =>
        `/** ${sentenceOf(effect.description, effect.name)} */\nexport const ${effect.name} = defineEffect<${namespaceOf(effect.name)}.ComponentProps>("${effect.name}");`,
);

// write the module
await writeFile(
    new URL("effects.ts", import.meta.url),
    `${HEADER}\n\n${imports.join("\n")}\nimport { defineEffect } from "./effect.tsx";\n\n${components.join("\n\n")}\n`,
);
