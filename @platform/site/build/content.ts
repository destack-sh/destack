import { collections } from "../content.ts";
import type { DevelopmentServer } from "@destack/web/build";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { watch } from "node:fs";
import { isAbsolute, join, normalize, relative, resolve } from "node:path";

/** The milliseconds to wait after the last source change before regenerating, so one save runs one generation. */
const debounceTime = 150;

/** One generated content collection watched during development. */
type ContentTask = {
    /** The directory in which the generator runs. */
    workingDirectory: string;

    /** Whether another generation is required after the active process exits. */
    isPending: boolean;

    /** Whether content watching has stopped. */
    isClosed: boolean;

    /** The task name used in diagnostics. */
    name: string;

    /** The active generator process. */
    process?: ReturnType<typeof spawn> | undefined;

    /** The generator path relative to the site directory. */
    script: string;

    /** The pending debounce timer. */
    timeout?: ReturnType<typeof setTimeout>;

    /** The source paths watched for changes. */
    triggers: readonly string[];

    /** The generated module directory invalidated after successful generation. */
    outputDirectory: string;
};

/** Regenerate content modules and reload the development server after source changes. */
export function watchContent(
    siteDirectory: string,
    development: DevelopmentServer,
): AsyncDisposable {
    // watch every content source directory
    const task: ContentTask = {
        name: "content",
        outputDirectory: join(siteDirectory, "src/view/content/generated"),
        workingDirectory: siteDirectory,
        isPending: false,
        isClosed: false,
        script: "scripts/generate-content.ts",
        triggers: collections.flatMap((collection) =>
            collection.sources.map((source) => resolve(siteDirectory, "../..", source.directory)),
        ),
    };

    // watch the sources apart from the development server, which replaces its Vite server on restart
    const watchers = task.triggers.map((trigger) =>
        watch(trigger, { recursive: true }, () => schedule(task, development)),
    );

    // stop watching and end any running generator with the server
    return {
        async [Symbol.asyncDispose]() {
            // stop listening and drop queued runs
            task.isClosed = true;
            for (const watcher of watchers) {
                watcher.close();
            }
            clearTimeout(task.timeout);
            task.isPending = false;

            // end the active generator and wait for it to exit
            const process = task.process;
            if (process) {
                const closed = once(process, "close");
                process.kill();
                await closed;
            }
        },
    };
}

/** Debounce one content task after a source change. */
function schedule(task: ContentTask, development: DevelopmentServer) {
    clearTimeout(task.timeout);
    task.timeout = setTimeout(() => run(task, development), debounceTime);
}

/** Start one generator or request another run after its active process exits. */
function run(task: ContentTask, development: DevelopmentServer) {
    // queue another run while one is active
    if (task.process != undefined) {
        task.isPending = true;

        return;
    }

    // run the generator, reporting a failed spawn
    task.process = spawn("bun", ["run", task.script], {
        cwd: task.workingDirectory,
        stdio: "inherit",
    });
    task.process.on("error", (error) => {
        development.vite.config.logger.error(`[site] ${task.name}: ${error.message}`);
    });
    task.process.on("close", (code) => {
        // ignore exits after the server closed
        task.process = undefined;
        if (task.isClosed) {
            return;
        }

        // reload after a successful generation
        if (code === 0) {
            invalidate(task, development);
        }
        // report a failed generation
        else {
            development.vite.config.logger.error(
                `[site] ${task.name} generation failed with code ${code}`,
            );
        }

        // run once more for changes made during this run
        if (task.isPending) {
            task.isPending = false;
            run(task, development);
        }
    });
}

/** Invalidate generated modules and reload the active page. */
function invalidate(task: ContentTask, development: DevelopmentServer) {
    // invalidate generated modules in every environment of the current Vite server
    const outputDirectory = normalize(task.outputDirectory);
    const { vite } = development;
    for (const environment of Object.values(vite.environments)) {
        for (const module of environment.moduleGraph.idToModuleMap.values()) {
            if (module.file != null && isWithin(module.file, outputDirectory)) {
                environment.moduleGraph.invalidateModule(module);
            }
        }
    }

    // reload the active page
    vite.ws.send({ type: "full-reload" });
}

/** Return whether one path belongs to a directory. */
function isWithin(path: string, directory: string) {
    const relation = relative(directory, normalize(path));

    return relation === "" || (!relation.startsWith("..") && !isAbsolute(relation));
}
