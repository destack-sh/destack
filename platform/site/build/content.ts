import { collections } from "../content";
import { spawn } from "node:child_process";
import { isAbsolute, join, normalize, relative, resolve } from "node:path";
import type { Plugin, ViteDevServer } from "vite";

/// One generated content collection watched during development.
type ContentTask = {
    /// The directory in which the generator runs.
    workingDirectory: string;

    /// Whether another generation is required after the active process exits.
    isPending?: boolean;

    /// The task name used in diagnostics.
    name: string;

    /// The active generator process.
    process?: ReturnType<typeof spawn>;

    /// The generator path relative to the site directory.
    script: string;

    /// Additional generator arguments.
    arguments?: readonly string[];

    /// The pending debounce timer.
    timeout?: ReturnType<typeof setTimeout>;

    /// The source paths watched for changes.
    triggers: readonly string[];

    /// The generated module directory invalidated after successful generation.
    outputDirectory: string;
};

/// Regenerate content modules and reload the development server after source changes.
export function contentPlugin(siteDirectory: string): Plugin {
    const tasks: ContentTask[] = [
        {
            name: "package documentation",
            outputDirectory: join(siteDirectory, "src/generated"),
            workingDirectory: siteDirectory,
            script: "scripts/generate-content.ts",
            arguments: ["--reference"],
            triggers: collections.flatMap((collection) =>
                collection.sources.filter((source) => source.readme).map((source) =>
                    resolve(siteDirectory, "../..", source.directory),
                ),
            ),
        },
        {
            name: "content",
            outputDirectory: join(siteDirectory, "src/generated"),
            workingDirectory: siteDirectory,
            script: "scripts/generate-content.ts",
            triggers: collections.flatMap((collection) =>
                collection.sources.map((source) =>
                    resolve(siteDirectory, "../..", source.directory),
                ),
            ),
        },
    ];

    return {
        name: "destack-content",
        apply: "serve",
        configureServer(server) {
            for (const task of tasks) {
                server.watcher.add([...task.triggers]);
            }

            server.watcher.on("all", (_event, path) => {
                const task = tasks.find((task) => isTriggered(path, task));
                if (task != undefined) {
                    schedule(task, server);
                }
            });
        },
    };
}

/// Return whether one changed path belongs to a task source directory.
function isTriggered(path: string, task: ContentTask) {
    const file = normalize(path);

    return task.triggers.some((trigger) => {
        const normalizedTrigger = normalize(trigger);
        const relation = relative(normalizedTrigger, file);

        return (
            relation === "" ||
            (!relation.startsWith("..") && !isAbsolute(relation))
        );
    });
}

/// Debounce one content task after a source change.
function schedule(task: ContentTask, server: ViteDevServer) {
    clearTimeout(task.timeout);
    task.timeout = setTimeout(() => run(task, server), 150);
}

/// Start one generator or request another run after its active process exits.
function run(task: ContentTask, server: ViteDevServer) {
    if (task.process != undefined) {
        task.isPending = true;
        return;
    }

    task.process = spawn("bun", [task.script, ...(task.arguments ?? [])], {
        cwd: task.workingDirectory,
        stdio: "inherit",
    });
    task.process.on("exit", (code) => {
        task.process = undefined;

        if (code === 0) {
            invalidate(task, server);
        } else {
            console.error(`[site] ${task.name} generation failed`);
        }

        if (task.isPending) {
            task.isPending = false;
            run(task, server);
        }
    });
}

/// Invalidate generated modules and reload the active page.
function invalidate(task: ContentTask, server: ViteDevServer) {
    const outputDirectory = normalize(task.outputDirectory);

    // invalidate generated modules in every serving environment before reloading
    for (const environment of Object.values(server.environments)) {
        for (const module of environment.moduleGraph.idToModuleMap.values()) {
            if (module.file != null && isWithin(module.file, outputDirectory)) {
                environment.moduleGraph.invalidateModule(module);
            }
        }
    }

    server.ws.send({ type: "full-reload" });
}

/// Return whether one path belongs to a directory.
function isWithin(path: string, directory: string) {
    const relation = relative(directory, normalize(path));

    return (
        relation === "" || (!relation.startsWith("..") && !isAbsolute(relation))
    );
}
