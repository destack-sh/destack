import { Runner } from "@destack/connection/server";
import { connection } from "@destack/connection";
import { and, Change, eq, isNull, type SQL, type Table } from "@destack/db";
import type { ObjectStore } from "@destack/object/server";
import { present, schema } from "@destack/schema";
import type { Controller } from "@destack/runtime/control";
import { RequestId } from "@destack/runtime/request";
import type { GitHubHosting } from "../github/index.ts";
import { repository } from "../object/index.ts";

/** Identifies each live GitHub repository at GitHub once the forge has its connection's replica. */
export class GitHubController implements Controller {
    /** The controller's name in reports. */
    readonly name = "github";
    /** The repositories, and the replicas of their connections. */
    readonly watches: readonly Table[] = [repository.table, connection.table];
    /** The forge's object store, recording the identifiers as the forge. */
    readonly #objects: Pick<
        ObjectStore,
        "database" | "principal" | "serving" | "mutate" | "clock" | "query"
    >;
    /** The repositories the forge hosts at GitHub. */
    readonly #hosting: GitHubHosting;

    /** Identify the GitHub repositories of a forge through its GitHub App. */
    constructor(
        objects: Pick<
            ObjectStore,
            "database" | "principal" | "serving" | "mutate" | "clock" | "query"
        >,
        hosting: GitHubHosting,
    ) {
        this.#objects = objects;
        this.#hosting = hosting;
    }

    /** List the unidentified repository a change touches, or those of a changed connection. */
    async keys(change: Change): Promise<readonly string[]> {
        // select a repository waiting for its identifier
        if (Change.of(change, repository.table)) {
            const after = Change.after(change);
            const isWaiting =
                after !== null &&
                after.hosting === "github" &&
                after.providerRepositoryId === null &&
                after.deletedAt === null;

            return isWaiting ? [after.id] : [];
        }
        // select the waiting repositories of a changed connection of an account
        else if (Change.of(change, connection.table)) {
            const image = Change.image(change);
            const scope = schema.identifier("account").safeParse(image.scope);

            return scope.success
                ? this.#waiting(
                      and(
                          eq(repository.table.accountId, scope.data),
                          eq(repository.table.connectionId, image.id),
                      ),
                  )
                : [];
        }
        // select nothing for other tables
        else {
            return [];
        }
    }

    /** List every repository waiting for its identifier. */
    async list(): Promise<readonly string[]> {
        return this.#waiting();
    }

    /** Identify a waiting repository once the forge has the replica of its connection. */
    async reconcile(id: string): Promise<undefined> {
        // read the repository while it waits
        const table = repository.table;
        const [row] = await this.#objects.database
            .select()
            .from(table)
            .where(
                and(
                    eq(table.id, schema.identifier("repository").parse(id)),
                    GitHubController.#unidentified(),
                ),
            );
        if (row === undefined) {
            return undefined;
        }

        // wait for the replica of its connection, and identify it as the forge
        const installation = await this.#hosting.installation(
            Runner.of(this.#objects),
            row.accountId,
            present(row.connectionId, "a GitHub repository's connection"),
        );
        if (installation !== undefined) {
            const now = this.#objects.clock.now();
            await this.#objects.mutate(
                this.#objects.serving(repository),
                {
                    id: RequestId.create(now),
                    calls: [repository.call("identify", repository.target(row))],
                },
                { component: this.name },
            );
        }

        return undefined;
    }

    /** List the waiting repositories a condition selects. */
    async #waiting(where?: SQL): Promise<string[]> {
        const rows = await this.#objects.database
            .select({ id: repository.table.id })
            .from(repository.table)
            .where(and(GitHubController.#unidentified(), where));

        return rows.map((row) => row.id);
    }

    /** Match the live GitHub repositories without GitHub's identifier. */
    static #unidentified(): SQL {
        const table = repository.table;

        return and(
            eq(table.hosting, "github"),
            isNull(table.providerRepositoryId),
            isNull(table.deletedAt),
        );
    }
}
