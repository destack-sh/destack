import { type FSWatcher, watch } from "node:fs";
import { sep } from "node:path";
import { eq, isNull, type Table, type Change } from "@destack/db";
import type { Package } from "@destack/package";
import { ServerCall } from "@destack/object";
import { type ObjectServer } from "@destack/object/server";
import { schema, type Identifier } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { RequestId } from "@destack/service/request";
import type { CheckoutSource } from "@destack/space/server";
import type { Controller, Reconciliation } from "@destack/service/control";
import type { BuildView } from "@destack/space/object";
import { Checkout } from "../checkout/index.ts";
import { preview, type Preview } from "../object/index.ts";
import type { BuildServiceOptions } from "./server.ts";
import { SpaceSession } from "./session.ts";

/** How long a preview waits for its machine to serve spaces before trying again: five seconds. */
const SPACES_WAIT_MILLISECONDS = 5000;

/** How long a package's files stay quiet before its preview builds again: an editor writes a save's events within tens of milliseconds. */
const SETTLE_MILLISECONDS = 200;

/** The previews whose sources a machine watches at once: a developer runs a handful, each watching a few directories. */
const WATCHED_PREVIEWS = 32;

/** The directories whose changes no build reads: Git's own files and installed dependencies. */
const IGNORED_DIRECTORIES = new Set([".git", "node_modules"]);

/** Run each preview: build its checkout's package, and submit the build to its installation. */
export class PreviewController implements Controller {
    /** The controller's name in reports. */
    readonly name = "preview";
    /** The tables the controller watches. */
    readonly watches: readonly Table[] = [preview.table];
    /** The object server observing previews as the machine. */
    readonly #server: Pick<ObjectServer, "database" | "execute" | "principal">;
    /** What the previews build with. */
    readonly #options: BuildServiceOptions;

    /** Run previews with an object server and what they build with. */
    constructor(
        server: Pick<ObjectServer, "database" | "execute" | "principal">,
        options: BuildServiceOptions,
    ) {
        this.#server = server;
        this.#options = options;
    }

    /** List the preview a change touches. */
    keys(change: Change<typeof preview.table>): readonly string[] {
        return [change.key.id];
    }

    /** List every preview. */
    async list(): Promise<readonly string[]> {
        const rows = await this.#server.database
            .select({ id: preview.table.id })
            .from(preview.table);

        return rows.map((row) => row.id);
    }

    /** Build and submit a preview's current generation, or finalize its deletion. */
    async reconcile(id: string): Promise<number | undefined> {
        // skip a gone preview and finalize a deleted one
        const [row] = await this.#server.database
            .select()
            .from(preview.table)
            .where(eq(preview.table.id, schema.identifier("preview").parse(id)));
        const now = Date.now();
        if (row === undefined) {
            return undefined;
        } else if (row.deletionRequestedAt !== null) {
            await this.#server.execute(
                this.#server.principal,
                preview,
                "finalize",
                [ServerCall.of(row)],
                now,
            );

            return undefined;
        } else if (row.observedGeneration === row.generation) {
            return undefined;
        }
        // wait while the machine serves no spaces
        else if (this.#options.spaces() === undefined) {
            return SPACES_WAIT_MILLISECONDS;
        }

        // run the generation
        try {
            const views = await this.#run(row);
            await this.#observe(row, "running", { views }, "Running", "");
        } catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            await this.#observe(row, "failed", {}, "Failed", message);
        }

        return undefined;
    }

    /** Build a preview's package in its space and submit the build, returning the views it mounts. */
    async #run(row: Preview): Promise<Record<string, BuildView>> {
        // build every output the package implies in the space as the preview's person
        const { machine, builders } = this.#options;
        const located = await Checkout.locate(this.#server.database, machine, {
            checkout: row.checkoutId,
            directory: row.package,
        });
        const selection = {
            kind: "checkout" as const,
            machine,
            checkout: row.checkoutId,
            directory: located.directory,
        };
        const session = await SpaceSession.open(this.#options, row.login);
        const built = await session.build(
            row.space,
            selection,
            await builders.outputs(located.path),
        );
        const reader = await this.#options.read(built.space, built.manifest);
        await this.#submit(row, session, selection, reader.manifest.package, built.id);

        // collect the views each output mounts
        const views: Record<string, BuildView> = {};
        for (const [output, compiled] of Object.entries(reader.manifest.outputs)) {
            for (const [name, view] of Object.entries(compiled.views)) {
                views[name] = { ...view, output };
            }
        }

        return views;
    }

    /** Submit a preview's build to its installation as the preview's person, installing it once. */
    async #submit(
        row: Preview,
        session: SpaceSession,
        selection: CheckoutSource,
        built: Package,
        build: Identifier<"build">,
    ): Promise<void> {
        // install the package following the checkout once
        const spaces = session.client;
        const spaceId = schema.identifier("space").parse(row.space);
        const { items } = await spaces.installation.list({
            spaceId,
            where: { packageId: built.id },
        });
        const installed =
            items.find((item) => PreviewController.#isFollowing(item.selection, selection)) ??
            (await spaces.installation.create({
                spaceId,
                requestId: RequestId.create(),
                packageId: built.id,
                selection,
                alias: built.name.slice(built.name.lastIndexOf("/") + 1),
            }));

        // submit the build for the installation to follow
        await spaces.installation.submit({
            spaceId,
            id: installed.id,
            requestId: RequestId.create(),
            selection,
            build: { kind: "build", build },
        });
    }

    /** Report whether an installation follows the same checkout's package directory. */
    static #isFollowing(
        followed: {
            readonly kind: string;
            readonly checkout?: string;
            readonly directory?: string;
        },
        selection: { readonly checkout: string; readonly directory: string },
    ): boolean {
        return (
            followed.kind === "checkout" &&
            followed.checkout === selection.checkout &&
            followed.directory === selection.directory
        );
    }

    /** Record a preview's status and observed fields at its current generation. */
    async #observe(
        row: Preview,
        status: Preview["status"],
        fields: Readonly<Record<string, unknown>>,
        reason: string,
        message: string,
    ): Promise<void> {
        const ready = status === "running" ? "true" : "false";
        await this.#server.execute(
            this.#server.principal,
            preview,
            "observe",
            [
                ServerCall.of(row, {
                    observedGeneration: row.generation,
                    conditions: { ready: { status: ready, reason, message } },
                    fields: { status, ...fields },
                }),
            ],
            Date.now(),
        );
    }
}

/** Start a new generation of each preview once the files of its package, or of the workspace packages it depends on, change. */
export class SourceController implements Controller {
    /** The controller's name in reports. */
    readonly name = "preview-source";
    /** Watch each preview's sources until the preview is deleted. */
    readonly mode = "follow";
    /** The previews, whose deletion stops their watch. */
    readonly watches: readonly Table[] = [preview.table];
    /** The previews watched at once. */
    readonly concurrency = WATCHED_PREVIEWS;
    /** The object server starting previews' generations as the machine. */
    readonly #server: Pick<ObjectServer, "database" | "execute" | "principal">;
    /** What the previews build with. */
    readonly #options: BuildServiceOptions;

    /** Watch previews with an object server and the machine's checkouts and builders. */
    constructor(
        server: Pick<ObjectServer, "database" | "execute" | "principal">,
        options: BuildServiceOptions,
    ) {
        this.#server = server;
        this.#options = options;
    }

    /** List the previews not being deleted. */
    async list(): Promise<readonly string[]> {
        const rows = await this.#server.database
            .select({ id: preview.table.id })
            .from(preview.table)
            .where(isNull(preview.table.deletionRequestedAt));

        return rows.map((row) => row.id);
    }

    /** Watch a preview's sources until stopped, starting a generation once each burst of changes settles. */
    async reconcile(key: string, { signal }: Reconciliation): Promise<undefined> {
        // read the preview's package directory and the directories its build reads
        const id = schema.identifier("preview").parse(key);
        const [row] = await this.#server.database
            .select()
            .from(preview.table)
            .where(eq(preview.table.id, id));
        if (row === undefined) {
            throw new ServiceError("NOT_FOUND", { message: `preview ${id} not found` });
        }
        const located = await Checkout.locate(this.#server.database, this.#options.machine, {
            checkout: row.checkoutId,
            directory: row.package,
        });
        const directories = await this.#options.builders.sources(located.path);

        // start a generation once a burst of changes settles
        const failed = new AbortController();
        const ended = AbortSignal.any([signal, failed.signal]);
        let settling: ReturnType<typeof setTimeout> | undefined;
        const changed = (filename: string | null) => {
            // leave changes to Git's files and installed dependencies
            const segments = filename?.split(sep) ?? [];
            if (segments.some((segment) => IGNORED_DIRECTORIES.has(segment))) {
                return;
            }

            // restart the settling period
            clearTimeout(settling);
            settling = setTimeout(() => {
                this.#refresh(id).catch((error: unknown) => failed.abort(error));
            }, SETTLE_MILLISECONDS);
        };

        // watch each directory until the preview leaves the list or the refresh fails
        const watchers: FSWatcher[] = [];
        try {
            for (const directory of directories) {
                watchers.push(
                    watch(directory, { recursive: true }, (_event, filename) => changed(filename)),
                );
            }
            await new Promise<void>((resolve) => {
                if (ended.aborted) {
                    resolve();
                }
                ended.addEventListener("abort", () => resolve(), { once: true });
            });
        } finally {
            clearTimeout(settling);
            for (const watcher of watchers) {
                watcher.close();
            }
        }

        // end with why the watch stopped
        throw ended.reason;
    }

    /** Start a new generation of a preview still listed, which its controller builds. */
    async #refresh(id: Identifier<"preview">): Promise<void> {
        const [row] = await this.#server.database
            .select()
            .from(preview.table)
            .where(eq(preview.table.id, id));
        if (row !== undefined && row.deletionRequestedAt === null) {
            await this.#server.execute(
                this.#server.principal,
                preview,
                "refresh",
                [ServerCall.of(row, {})],
                Date.now(),
            );
        }
    }
}
