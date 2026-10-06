import { and, eq, inArray, type DatabaseConnection } from "@destack/db";
import { aligned } from "@destack/schema";
import { comment, page, project, tag, task, taskTag } from "../../test/fixture.ts";
import type { Random } from "./random.ts";

/** The rows of one seeding insert, well within every parameter budget. */
const SEED_BATCH = 250;

/** The folders rows live in, most of them followed. */
const FOLDERS = ["inbox", "inbox", "inbox", "inbox", "archive"] as const;

/** Random writes to every test table. */
export class Workload {
    /** The random sequence the writes follow. */
    readonly #random: Random;
    /** The project identities written. */
    readonly #projects: readonly string[];
    /** The task identities written. */
    readonly #tasks: readonly string[];
    /** The comment identities written. */
    readonly #comments: readonly string[];
    /** The tag identities written. */
    readonly #tags: readonly string[];
    /** The task tag identities written. */
    readonly #taskTags: readonly string[];
    /** The page identities written. */
    readonly #pages: readonly string[];

    /** Write rows of a few projects, many tasks and more comments. */
    constructor(random: Random, sizes = { projects: 4, tasks: 24, comments: 40 }) {
        // name the rows the writes pick from
        this.#random = random;
        this.#projects = Array.from({ length: sizes.projects }, (_, index) => `p${index}`);
        this.#tasks = Array.from({ length: sizes.tasks }, (_, index) => `t${index}`);
        this.#comments = Array.from({ length: sizes.comments }, (_, index) => `c${index}`);
        this.#tags = Array.from({ length: 4 }, (_, index) => `g${index}`);
        this.#taskTags = Array.from({ length: 30 }, (_, index) => `j${index}`);
        this.#pages = Array.from({ length: 16 }, (_, index) => `q${index}`);
    }

    /** Insert every project and a number of extra tasks and comments. */
    async seed(database: DatabaseConnection, count: number): Promise<void> {
        // insert projects followed by tasks and comments in batches
        const random = this.#random;
        await database.insert(project).values(
            this.#projects.map((id) => ({
                id,
                scope: "inbox",
                name: random.pick(["a", "b"]),
            })),
        );
        for (let start = 0; start < count; start += SEED_BATCH) {
            const ids = Array.from(
                { length: Math.min(SEED_BATCH, count - start) },
                (_, index) => start + index,
            );
            await database.insert(task).values(
                ids.map((index) => ({
                    id: `s${index}`,
                    scope: random.pick(FOLDERS),
                    projectId: random.pick(this.#projects),
                    state: random.pick(["open", "doing", "done"]),
                    rank: random.integer(10),
                    points: random.chance(0.2) ? null : random.integer(5),
                    title: null,
                    isSecret: random.chance(0.15),
                })),
            );
            await database.insert(comment).values(
                ids.map((index) => ({
                    id: `r${index}`,
                    scope: random.pick(FOLDERS),
                    taskId: `s${random.integer(count)}`,
                    position: random.integer(6),
                })),
            );
        }
    }

    /** Make one write, or a few in one transaction. */
    async write(database: DatabaseConnection): Promise<void> {
        // group some writes into one transaction
        if (this.#random.chance(0.15)) {
            await database.transaction(async (transaction) => {
                for (let index = 0; index < 3; index += 1) {
                    await this.#write(transaction);
                }
            });
        } else {
            await this.#write(database);
        }
    }

    /** Make one write to a random row. */
    async #write(database: DatabaseConnection): Promise<void> {
        // pick the table, the deletion and the folder
        const random = this.#random;
        const kind = random.pick([
            "project",
            "task",
            "task",
            "task",
            "comment",
            "comment",
            "tag",
            "taskTag",
            "taskTag",
            "page",
            "page",
            "page",
        ] as const);
        const isDeleted = random.chance(0.2);
        const folder = random.pick(FOLDERS);

        // write a row of the picked table
        if (kind === "project") {
            await this.#writeProject(database, isDeleted, folder);
        } else if (kind === "task") {
            await this.#writeTask(database, isDeleted, folder);
        } else if (kind === "tag") {
            await this.#writeTag(database, isDeleted, folder);
        } else if (kind === "taskTag") {
            await this.#writeTaskTag(database, isDeleted, folder);
        } else if (kind === "page") {
            await this.#writePage(database, isDeleted, folder);
        } else {
            await this.#writeComment(database, isDeleted, folder);
        }
    }

    /** Write a project. */
    async #writeProject(
        database: DatabaseConnection,
        isDeleted: boolean,
        folder: string,
    ): Promise<void> {
        // pick the row and its values, then delete or upsert it
        const random = this.#random;
        const id = random.pick(this.#projects);
        const values = { id, scope: folder, name: random.pick(["a", "b", "x"]) };
        if (isDeleted) {
            await database.delete(project).where(eq(project.id, id));
        } else {
            await database
                .insert(project)
                .values(values)
                .onConflictDoUpdate({ target: project.id, set: values });
        }
    }

    /** Write a task. */
    async #writeTask(
        database: DatabaseConnection,
        isDeleted: boolean,
        folder: string,
    ): Promise<void> {
        // pick the row and its values, then delete or upsert it
        const random = this.#random;
        const id = random.pick(this.#tasks);
        const values = {
            id,
            scope: folder,
            projectId: random.chance(0.9) ? random.pick(this.#projects) : null,
            state: random.pick(["open", "doing", "done"]),
            rank: random.integer(10),
            points: random.chance(0.2) ? null : random.integer(5),
            title: random.chance(0.3) ? null : random.pick(["write", "ship", "fix"]),
            isSecret: random.chance(0.15),
        };
        if (isDeleted) {
            await database.delete(task).where(eq(task.id, id));
        } else {
            await database
                .insert(task)
                .values(values)
                .onConflictDoUpdate({ target: task.id, set: values });
        }
    }

    /** Write a tag in a folder or in a project's own scope. */
    async #writeTag(
        database: DatabaseConnection,
        isDeleted: boolean,
        folder: string,
    ): Promise<void> {
        // pick the row and its values, then delete or upsert it
        const random = this.#random;
        const id = random.pick(this.#tags);
        const scope = random.chance(0.4) ? random.pick(this.#projects) : folder;
        const values = { id, scope, name: random.pick(["a", "b", "hidden"]) };
        await (isDeleted
            ? database.delete(tag).where(eq(tag.id, id))
            : database
                  .insert(tag)
                  .values(values)
                  .onConflictDoUpdate({ target: tag.id, set: values }));
    }

    /** Tag or untag a task. */
    async #writeTaskTag(
        database: DatabaseConnection,
        isDeleted: boolean,
        folder: string,
    ): Promise<void> {
        // pick the row and its values, then delete or upsert it
        const random = this.#random;
        const id = random.pick(this.#taskTags);
        const values = {
            id,
            scope: folder,
            taskId: random.pick(this.#tasks),
            tagId: random.pick(this.#tags),
        };
        await (isDeleted
            ? database.delete(taskTag).where(eq(taskTag.id, id))
            : database
                  .insert(taskTag)
                  .values(values)
                  .onConflictDoUpdate({ target: taskTag.id, set: values }));
    }

    /** Write a page under a lower page of its folder or at the top, deleting only leaves. */
    async #writePage(
        database: DatabaseConnection,
        isDeleted: boolean,
        folder: string,
    ): Promise<void> {
        // pick the row and its values, then delete or upsert it
        const random = this.#random;
        const index = random.integer(this.#pages.length);
        const id = aligned(this.#pages, index);
        const [existing] = await database.select().from(page).where(eq(page.id, id));
        const home = existing?.scope ?? folder;
        const parents = await database
            .select({ id: page.id })
            .from(page)
            .where(and(eq(page.scope, home), inArray(page.id, this.#pages.slice(0, index))));
        const parentId = parents.length > 0 && random.chance(0.8) ? random.pick(parents).id : null;
        if (isDeleted) {
            const [child] = await database
                .select({ id: page.id })
                .from(page)
                .where(eq(page.parentId, id))
                .limit(1);
            if (child === undefined) {
                await database.delete(page).where(eq(page.id, id));
            }
        } else if (existing !== undefined) {
            await database
                .update(page)
                .set({ parentId, rank: random.integer(10) })
                .where(eq(page.id, id));
        } else {
            await database
                .insert(page)
                .values({ id, scope: home, parentId, rank: random.integer(10) });
        }
    }

    /** Write a comment. */
    async #writeComment(
        database: DatabaseConnection,
        isDeleted: boolean,
        folder: string,
    ): Promise<void> {
        // pick the row and its values, then delete or upsert it
        const random = this.#random;
        const id = random.pick(this.#comments);
        const values = {
            id,
            scope: folder,
            taskId: random.chance(0.95) ? random.pick(this.#tasks) : null,
            position: random.integer(6),
        };
        if (isDeleted) {
            await database.delete(comment).where(eq(comment.id, id));
        } else {
            await database
                .insert(comment)
                .values(values)
                .onConflictDoUpdate({ target: comment.id, set: values });
        }
    }
}
