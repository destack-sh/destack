import { spawn } from "node:child_process";
import { dirname, isAbsolute, join, normalize, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import type { Plugin, ViteDevServer } from "vite";
import { normalizePath } from "vite";

const siteDirectory = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repositoryDirectory = resolve(siteDirectory, "../..");

type ContentTask = {
    isPending?: boolean;
    name: string;
    process?: ReturnType<typeof spawn>;
    script: string;
    timeout?: ReturnType<typeof setTimeout>;
    triggers: readonly string[];
    outputs: readonly string[];
};

export function contentPlugin(): Plugin {
    const tasks: ContentTask[] = [
        {
            name: "snippets",
            script: "scripts/generate-snippets.mjs",
            triggers: [
                join(siteDirectory, "src/snippets"),
                join(repositoryDirectory, "language/grammar/destack/queries/highlights.scm"),
            ],
            outputs: [join(siteDirectory, "src/generated/snippets.ts")],
        },
        {
            name: "posts",
            script: "scripts/generate-posts.mjs",
            triggers: [join(siteDirectory, "src/content/blog")],
            outputs: [
                join(siteDirectory, "src/generated/posts.ts"),
                join(siteDirectory, "src/generated/prerender-routes.ts"),
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

function isTriggered(path: string, task: ContentTask) {
    const file = normalize(path);

    return task.triggers.some((trigger) => {
        const normalizedTrigger = normalize(trigger);
        const relation = relative(normalizedTrigger, file);

        return relation === "" || (!relation.startsWith("..") && !isAbsolute(relation));
    });
}

function schedule(task: ContentTask, server: ViteDevServer) {
    clearTimeout(task.timeout);
    task.timeout = setTimeout(() => run(task, server), 150);
}

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

function invalidate(task: ContentTask, server: ViteDevServer) {
    for (const output of task.outputs) {
        const module = server.moduleGraph.getModuleById(normalizePath(output));
        if (module != undefined) {
            server.moduleGraph.invalidateModule(module);
        }
    }

    server.ws.send({ type: "full-reload" });
}
