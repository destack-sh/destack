import { mkdir, readdir, readFile, rm, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { formatSource } from "@destack/check";
import { icons } from "@phosphor-icons/core";

/** The Phosphor weights, each a directory of SVG files whose names carry the weight as a suffix. */
const WEIGHTS = ["thin", "light", "regular", "bold", "fill", "duotone"] as const;

/** The opening and closing tags around each Phosphor icon's SVG body. */
const SVG_ELEMENT = /^<svg [^>]*>(?<body>.*)<\/svg>\s*$/su;

/** The directory holding the Phosphor modules. */
const OUTPUT = dirname(fileURLToPath(import.meta.url));

/** The directory holding one generated module per icon. */
const ICONS = join(OUTPUT, "icon");

/** The module with every icon, its categories and its search words. */
const ICONS_LIST = join(OUTPUT, "icons.ts");

/** The directory holding Phosphor's SVG sources. */
const ASSETS = join(
    dirname(fileURLToPath(import.meta.resolve("@phosphor-icons/core/package.json"))),
    "assets",
);

/** A Phosphor icon as its metadata describes it. */
interface PhosphorIcon {
    /** The icon's name. */
    readonly name: string;
    /** The categories it belongs to. */
    readonly categories: readonly string[];
    /** The words that find it. */
    readonly tags: readonly string[];
}

/** Write one module per icon with its SVG body in each weight, and the list of every icon. */
async function generate(): Promise<void> {
    // read every weight's icon bodies
    const bodies = await Promise.all(WEIGHTS.map((weight) => readWeight(weight)));
    const names = [...(bodies[0]?.keys() ?? [])].toSorted();

    // write every icon with its categories and search words, without Phosphor's release markers, rejecting an icon without metadata
    const metadata: readonly PhosphorIcon[] = icons;
    const described = new Map(metadata.map((icon) => [icon.name, icon]));
    const listed = names.map((name) => {
        // read the icon's metadata
        const icon = described.get(name);
        if (icon === undefined) {
            throw new Error(`the ${name} icon has no metadata`);
        }
        const tags = icon.tags.filter((tag) => !tag.startsWith("*"));

        return `    { name: ${JSON.stringify(name)}, categories: ${JSON.stringify(icon.categories)}, tags: ${JSON.stringify(tags)} },\n`;
    });
    await writeModule(
        ICONS_LIST,
        `/** Every Phosphor icon in name order: its name, the categories it belongs to and the words that find it. */\nexport const ICONS = [\n${listed.join("")}] as const;\n\n/** The name of a Phosphor icon. */\nexport type IconName = (typeof ICONS)[number]["name"];\n`,
    );

    // replace the icon modules, dropping icons Phosphor removed
    await rm(ICONS, { recursive: true, force: true });
    await mkdir(ICONS);

    // write each icon's bodies, rejecting an icon a weight misses
    for (const name of names) {
        const entries = WEIGHTS.map((weight, index) => {
            const body = bodies[index]?.get(name);
            if (body === undefined) {
                throw new Error(`the ${weight} weight has no ${name} icon`);
            }

            return `    ${weight}: ${JSON.stringify(body)},\n`;
        });
        const source = `import type { IconBodies } from "../../icon/icon.tsx";\n\n/** The SVG body of the ${name} icon in each weight. */\nconst icon: IconBodies = {\n${entries.join("")}};\n\nexport default icon;\n`;
        await writeModule(join(ICONS, `${name}.ts`), source);
    }
}

/** Write one generated module in the package's format. */
async function writeModule(path: string, source: string): Promise<void> {
    await writeFile(path, await formatSource(path, source));
}

/** Read the SVG body of each icon of one weight, keyed by the icon name. */
async function readWeight(weight: (typeof WEIGHTS)[number]): Promise<Map<string, string>> {
    // list the weight's files in name order
    const directory = join(ASSETS, weight);
    const files = (await readdir(directory)).filter((file) => file.endsWith(".svg")).toSorted();
    const suffix = weight === "regular" ? ".svg" : `-${weight}.svg`;

    // strip the weight suffix and the SVG element from each file
    const bodies = new Map<string, string>();
    for (const file of files) {
        const match = SVG_ELEMENT.exec(await readFile(join(directory, file), "utf8"));
        if (match?.groups?.["body"] === undefined) {
            throw new Error(`${file} has no SVG element`);
        }
        bodies.set(file.slice(0, -suffix.length), match.groups["body"]);
    }

    return bodies;
}

await generate();
