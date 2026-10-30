import { join } from "node:path";
import { BunEnvironment } from "@destack/environment/bun";
import { Dialect } from "../dialect/dialect.ts";
import { TestDatabase } from "./database.ts";
import { ImageIndex, TestTemplate } from "./template.ts";

/** Milliseconds per second. */
const MILLISECONDS = 1000;

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
    const { clock, process } = BunEnvironment.system;
    for (const template of templates.filter((each) => each.dialects?.includes(dialect) ?? true)) {
        // write the image, reporting its host time
        const started = clock.now();
        images[template.name] = {
            [dialect]: await TestDatabase.image(template, dialect, { id, directory }),
        };
        const seconds = ((clock.now() - started) / MILLISECONDS).toFixed(1);
        process.stderr.writeSync(`template ${template.name}: ${seconds} s\n`);
    }

    // leave the index for the run's setup, whole or not at all
    const index = JSON.stringify(ImageIndex.parse(images));
    const { files } = BunEnvironment.system;
    await files.writeTextFile(join(directory, "images.json.partial"), index);
    await files.rename(join(directory, "images.json.partial"), join(directory, "images.json"));
}

/** The module, the run's directory, its id and its database, as the run's setup passes them. */
const [module, directory, id, target] = BunEnvironment.system.process.arguments;

// exit once the run's setup ends, as its end closes this writer's input
void new Response(BunEnvironment.system.process.stdin.readable).arrayBuffer().finally(() => {
    BunEnvironment.system.process.exit(1);
});

// write the templates of the module the run's setup names, then close the database servers
try {
    await write(module, directory, id, target);
} finally {
    await TestDatabase.close();
}

// exit once written, as the read of the input keeps the process alive
BunEnvironment.system.process.exit(0);
