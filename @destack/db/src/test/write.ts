import { join } from "node:path";
import { BunEnvironment } from "@destack/environment/bun";
import { Dialect } from "../dialect/dialect.ts";
import { TestDatabase } from "./database.ts";
import { ImageIndex, TestTemplate } from "./template.ts";

/** Write the images of the templates a module lists into a run's directory, with their index. */
async function write(
    module: string | undefined,
    directory: string | undefined,
    id: string | undefined,
    target: string | undefined,
): Promise<void> {
    // require the template module, the run's directory, its id and its database
    if (module === undefined || directory === undefined || id === undefined) {
        throw new TypeError("write the templates of a module into a run's directory under its id");
    }
    const dialect = Dialect.parse(target);

    // require the module to list templates
    const imported: unknown = await import(module);
    const templates =
        typeof imported === "object" && imported !== null && "templates" in imported
            ? imported.templates
            : undefined;
    if (!Array.isArray(templates) || !templates.every((entry) => entry instanceof TestTemplate)) {
        throw new TypeError(`${module} lists no templates`);
    }

    // drop expired schemas, then write each template's image in the run's database
    if (dialect === "postgresql") {
        await TestDatabase.sweep();
    }
    const images: Record<string, Record<string, unknown>> = {};
    for (const template of templates.filter((each) => each.dialects?.includes(dialect) ?? true)) {
        images[template.name] = {
            [dialect]: await TestDatabase.image(template, dialect, { id, directory }),
        };
    }

    // leave the index for the run's setup
    const index = JSON.stringify(ImageIndex.parse(images));
    await BunEnvironment.system.files.writeTextFile(join(directory, "images.json"), index);
}

/** The module, the run's directory, its id and its database, as the run's setup passes them. */
const [module, directory, id, target] = BunEnvironment.system.process.arguments;

// write the templates of the module the run's setup names, then close the database servers
try {
    await write(module, directory, id, target);
} finally {
    await TestDatabase.close();
}
