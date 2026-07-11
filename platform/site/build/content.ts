import { spawn } from "node:child_process";
import { dirname, isAbsolute, join, normalize, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import type { Plugin, ViteDevServer } from "vite";
import { normalizePath } from "vite";

const siteDirectory = resolve(dirname(fileURLToPath(import.meta.url)), "..");

/// One generated content collection watched during development.
type ContentTask = {
    /// Whether another generation is required after the active process exits.
    isPending?: boolean;

    /// The task name used in diagnostics.
    name: string;

    /// The active generator process.
    process?: ReturnType<typeof spawn>;

    /// The generator path relative to the site directory.
    script: string;

    /// The pending debounce timer.
    timeout?: ReturnType<typeof setTimeout>;

    /// The source paths watched for changes.
    triggers: readonly string[];

    /// The generated modules invalidated after successful generation.
    outputs: readonly string[];
};

/// Regenerate content modules and reload the development server after source changes.
export function contentPlugin(): Plugin {
    const tasks: ContentTask[] = [
        {
            name: "content",
            script: "scripts/generate-content.mjs",
            triggers: [
                join(siteDirectory, "src/content/blog"),
                resolve(siteDirectory, "../../docs"),
            ],
            outputs: [
                join(siteDirectory, "src/generated/documents.ts"),
                join(siteDirectory, "src/generated/posts.ts"),
                join(siteDirectory, "src/generated/prerender-routes.ts"),
                join(siteDirectory, "src/generated/search.ts"),
            ],
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

        return relation === "" || (!relation.startsWith("..") && !isAbsolute(relation));
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

    task.process = spawn("bun", [task.script], {
        cwd: siteDirectory,
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
    for (const output of task.outputs) {
        const module = server.moduleGraph.getModuleById(normalizePath(output));
        if (module != undefined) {
            server.moduleGraph.invalidateModule(module);
        }
    }

    server.ws.send({ type: "full-reload" });
}
