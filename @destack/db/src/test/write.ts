import { writeFile } from "node:fs/promises";
import { join } from "node:path";
import { TEST_DIALECTS, TestDatabase } from "./database.ts";
import { ImageIndex, TestTemplate } from "./template.ts";

/** Write the images of the templates a module lists into a run's directory, with their index. */
async function write(
    module: string | undefined,
    directory: string | undefined,
    id: string | undefined,
): Promise<void> {
    // require the module listing the templates, the run's directory and the run's id
    if (module === undefined || directory === undefined || id === undefined) {
        throw new TypeError("write the templates of a module into a run's directory under its id");
    }

    // require the module to list templates
    const imported: unknown = await import(module);
    const templates =
        typeof imported === "object" && imported !== null && "templates" in imported
            ? imported.templates
            : undefined;
    if (!Array.isArray(templates) || !templates.every((entry) => entry instanceof TestTemplate)) {
        throw new TypeError(`${module} lists no templates`);
    }

    // drop expired schemas, then write each template's image in each test dialect
    if (TEST_DIALECTS.includes("postgresql")) {
        await TestDatabase.sweep();
    }
    const images: Record<string, Record<string, unknown>> = {};
    for (const template of templates) {
        for (const dialect of TEST_DIALECTS.filter(
            (each) => template.dialects?.includes(each) ?? true,
        )) {
            const image = await TestDatabase.image(template, dialect, { id, directory });
            images[template.name] = { ...images[template.name], [dialect]: image };
        }
    }

    // leave the index for the run's setup
    await writeFile(join(directory, "images.json"), JSON.stringify(ImageIndex.parse(images)));
}

// write the templates of the module the run's setup names
await write(process.argv[2], process.argv[3], process.argv[4]);
