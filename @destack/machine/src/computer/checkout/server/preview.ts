import type { FileSystem } from "@destack/environment";
import { Workspace } from "@destack/check/workspace";
import { join, sep } from "node:path";
import { type Controller, eq, type Following, type Select } from "@destack/db";
import { type ObjectStore } from "@destack/object/server";
import type { Identifier } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { CheckoutServer } from "./checkout.ts";
import { preview } from "../object/index.ts";
import type {} from "@destack/package/import-meta";
import { MachineError } from "../../../error/index.ts";

/** How long a package's files stay quiet before its preview builds again. */
const SETTLE_MILLISECONDS = 200;

/** The previews whose sources a machine watches at once. */
const WATCHED_PREVIEWS = 32;

/** The directories whose changes no build reads: Git's own files and installed dependencies. */
const IGNORED_DIRECTORIES = new Set([".git", "node_modules"]);

/** Start a new generation of each preview once its package's files change. */
export class PreviewSource {
    /** The object store starting previews' generations as the machine. */
    readonly #store: Pick<ObjectStore, "database" | "mutate" | "as" | "clock" | "environment">;
    /** The development workflow the previews build with. */
    readonly #server: CheckoutServer;

    /** Watch previews with an object store and the machine's checkouts and compilers. */
    constructor(
        store: Pick<ObjectStore, "database" | "mutate" | "as" | "clock" | "environment">,
        server: CheckoutServer,
    ) {
        this.#store = store;
        this.#server = server;
    }

    /** Build the controller watching each preview's sources until the preview is deleted. */
    controller(): Controller<typeof preview.table> {
        return {
            name: "preview-source",
            table: preview.table,
            pending: { deletedAt: { isNull: true } },
            concurrency: WATCHED_PREVIEWS,
            follow: (rows, following) => this.follow(rows, following),
        };
    }

    /** Watch a preview's sources until stopped. */
    async follow(
        rows: readonly Select<typeof preview.table>[],
        { signal, synced }: Following,
    ): Promise<void> {
        // read the preview's package directory and the directories its build reads
        const [row] = rows;
        if (row === undefined) {
            throw MachineError.of("PREVIEW_NOT_FOUND", "followed preview row not found");
        }
        const { id } = row;
        const located = await CheckoutServer.locate(this.#server.machine, {
            checkout: row.checkoutId,
            directory: row.package,
        });
        const directories = await PreviewSource.#sources(located.path, this.#server.host.files);

        // start a generation once a burst of changes settles
        const failed = new AbortController();
        const ended = AbortSignal.any([signal, failed.signal]);
        let settling = new AbortController();
        const changed = (filename: string) => {
            // leave changes to Git's files and installed dependencies
            const segments = filename.split(sep);
            if (segments.some((segment) => IGNORED_DIRECTORIES.has(segment))) {
                return;
            }

            // restart the settling period on the store's clock
            settling.abort();
            const period = new AbortController();
            settling = period;
            this.#store.clock
                .sleep(SETTLE_MILLISECONDS, period.signal)
                .then(
                    () => this.#refresh(id),
                    () => undefined,
                )
                .catch((error: unknown) => failed.abort(error));
        };

        // watch each directory until the preview leaves the list or a watch or the refresh fails
        const { files } = this.#server.host;
        const watch = async (directory: string) => {
            const changes = await files.watch(directory, { recursive: true, signal: ended });
            for await (const change of changes) {
                changed(change.path);
            }
        };
        for (const directory of directories) {
            void watch(directory).catch((error: unknown) => failed.abort(error));
        }
        try {
            synced();
            await new Promise<void>((resolve) => {
                if (ended.aborted) {
                    resolve();
                }
                ended.addEventListener("abort", () => resolve(), { once: true });
            });
        } finally {
            settling.abort();
        }

        // end with why the watch stopped
        throw ended.reason;
    }

    /** Start a new generation of a preview still listed, which its controller builds. */
    async #refresh(id: Identifier<"preview">): Promise<void> {
        const [row] = await this.#store.database
            .select()
            .from(preview.table)
            .where(eq(preview.table.id, id));
        if (row !== undefined && row.deletedAt === null) {
            await this.#store.mutate(this.#store.as(preview), {
                id: RequestId.create(this.#store.environment),
                calls: [preview.call("refresh", preview.target(row))],
            });
        }
    }

    /** List the directories whose files a package directory's build reads. */
    static async #sources(directory: string, files: FileSystem): Promise<string[]> {
        // read the package's workspace, reading the directory alone outside one
        const workspace = await Workspace.read(directory, files);
        const member = workspace?.members.find(
            (candidate) => join(workspace.root, candidate.directory) === directory,
        );
        if (workspace === undefined || member === undefined) {
            return [directory];
        }

        return workspace
            .dependencies(member.directory)
            .map((dependency) => join(workspace.root, dependency))
            .toSorted();
    }
}
