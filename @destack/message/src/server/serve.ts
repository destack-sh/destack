import type { ImplementationInput } from "@destack/object/server";
import { Task } from "@destack/workflow";
import { task } from "../object/task.ts";
import type messagePackage from "../package.ts";
import { implementEndpoint } from "./endpoint.ts";
import { implementMessage, type MessageOptions } from "./message.ts";

/** Serve messages through the given providers, with webhook endpoints and the package's tasks. */
export function serveMessages(options: MessageOptions): ImplementationInput<typeof messagePackage> {
    // forget browsers their push services forgot through the package's tasks
    const tasks = task.implement((step, input) => Task.run(step, input));

    return {
        objects: [implementMessage(options), implementEndpoint(options.keys), ...tasks.objects],
        controllers: tasks.controllers,
    };
}
