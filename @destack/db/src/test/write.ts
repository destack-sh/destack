import { join } from "node:path";
import { BunEnvironment } from "@destack/environment/bun";
import { Dialect } from "../dialect/dialect.ts";
import { TestDatabase } from "./database.ts";
import { ImageIndex, TemplateImage, TestTemplate } from "./template.ts";

/** Milliseconds per second. */
const MILLISECONDS = 1000;

/** The host the writer runs on, as the run's child. */
const HOST = BunEnvironment.system;

/** The run's template module, directory, id and database, and the one template a child writes. */
interface WriteArguments {
    /** The module listing the templates. */
    readonly module: string;
    /** The run's directory. */
    readonly directory: string;
    /** The run's id. */
    readonly id: string;
    /** The database the images are written in. */
    readonly dialect: Dialect;
    /** The index of the one template a child writes, absent for the writer of every template. */
    readonly index: number | undefined;
}

/** Write the images of the templates a module lists into a run's directory, each in a process. */
async function write(given: WriteArguments): Promise<void> {
    // require the module to list templates, and take those of the run's database
    const imported: unknown = await import(given.module);
    const listed =
        typeof imported === "object" && imported !== null && "templates" in imported
            ? imported.templates
            : undefined;
    if (!Array.isArray(listed) || !listed.every((entry) => entry instanceof TestTemplate)) {
        throw new TypeError(`'${given.module}' lists no templates`);
    }
    const templates = listed.filter((each) => each.dialects?.includes(given.dialect) ?? true);

    // write the child's one template
    if (given.index !== undefined) {
        await writeImage(given, templateAt(templates, given.index), given.index);
    }
    // write every template, the first here and each other in a child of its own
    else {
        if (given.dialect === "postgresql") {
            await TestDatabase.sweep();
        }
        const children = templates.slice(1).map((_, offset) => spawnChild(given, offset + 1));
        await Promise.all([
            templates.length > 0 ? writeImage(given, templateAt(templates, 0), 0) : undefined,
            ...children.map(async (child) => await awaitChild(child, children)),
        ]);
        await writeIndex(given, templates);
    }
}

/** Write one template's image into the run's database and its file, reporting its host time. */
async function writeImage(
    given: WriteArguments,
    template: TestTemplate,
    index: number,
): Promise<void> {
    // write the image
    const started = HOST.clock.now();
    const run = { id: given.id, directory: given.directory };
    const image = await TestDatabase.image(template, given.dialect, run);
    await HOST.files.writeTextFile(imagePath(given.directory, index), JSON.stringify(image));

    // report its time
    const seconds = ((HOST.clock.now() - started) / MILLISECONDS).toFixed(1);
    HOST.process.stderr.writeSync(`template ${template.name}: ${seconds} s\n`);
}

/** Start a child writing one template, sharing the writer's report. */
function spawnChild(given: WriteArguments, index: number) {
    const self = new URL(import.meta.url).pathname;
    const arguments_ = [given.module, given.directory, given.id, given.dialect, String(index)];

    return HOST.process.spawn([HOST.process.executable, self, ...arguments_], {
        variables: HOST.process.variables,
        stdin: "pipe",
        stdout: "inherit",
        stderr: "inherit",
    });
}

/** Wait for a child, stopping every other child once one fails. */
async function awaitChild(
    child: ReturnType<typeof spawnChild>,
    children: readonly ReturnType<typeof spawnChild>[],
): Promise<void> {
    const code = await child.exited;
    if (code !== 0) {
        for (const other of children) {
            other.kill("SIGKILL");
        }
        throw new Error(`writing a template failed with ${code}`);
    }
}

/** Leave the index of the written images for the run's setup, whole or not at all. */
async function writeIndex(
    given: WriteArguments,
    templates: readonly TestTemplate[],
): Promise<void> {
    // read each image by its template's name
    const { directory } = given;
    const images: Record<string, Record<string, unknown>> = {};
    for (const [index, template] of templates.entries()) {
        const text = await HOST.files.readTextFile(imagePath(directory, index));
        images[template.name] = { [given.dialect]: TemplateImage.parse(JSON.parse(text)) };
    }

    // write the index at once
    const index = JSON.stringify(ImageIndex.parse(images));
    await HOST.files.writeTextFile(join(directory, "images.json.partial"), index);
    await HOST.files.rename(join(directory, "images.json.partial"), join(directory, "images.json"));
}

/** Take the template at an index the writer listed. */
function templateAt(templates: readonly TestTemplate[], index: number): TestTemplate {
    const template = templates[index];
    if (template === undefined) {
        throw new TypeError(`the module lists no template ${index}`);
    }

    return template;
}

/** The file one template's image is written to. */
function imagePath(directory: string, index: number): string {
    return join(directory, `image-${index}.json`);
}

/** Read the arguments the run's setup or the writer passes. */
function readArguments(values: readonly string[]): WriteArguments {
    const [module, directory, id, target, index] = values;
    if (module === undefined || directory === undefined || id === undefined) {
        throw new TypeError("write the templates of a module into a run's directory under its id");
    }

    return {
        module,
        directory,
        id,
        dialect: Dialect.parse(target),
        index: index === undefined ? undefined : Number(index),
    };
}

// exit once the parent ends, as its end closes this writer's input
void new Response(HOST.process.stdin.readable).arrayBuffer().finally(() => {
    HOST.process.exit(1);
});

// write the templates, reporting a suppressed failure with both errors, then close the database servers
try {
    await write(readArguments(HOST.process.arguments));
} catch (error) {
    throw error instanceof SuppressedError
        ? new AggregateError(
              [error.error, error.suppressed],
              "writing a template failed while disposing",
          )
        : error;
} finally {
    await TestDatabase.close();
}

// exit once written, as the read of the input keeps the process alive
HOST.process.exit(0);
