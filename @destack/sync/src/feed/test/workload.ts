import { and, eq, inArray, type DatabaseConnection } from "@destack/db";
import { comment, page, project, tag, task, taskTag } from "../../test/fixture.ts";
import type { Random } from "./random.ts";

/** The rows one seeding insert writes: a few hundred rows of a few columns stay well within every parameter budget. */
const SEED_BATCH = 250;

/** The folders rows live in, most of them followed. */
const FOLDERS = ["inbox", "inbox", "inbox", "inbox", "archive"] as const;

/** Random writes to projects, tasks, comments, tags, task tags and pages: inserts, updates, moves between folders and parents, deletions. */
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

    /** Insert many rows at once: every project, and a number of tasks and comments beyond the ones writes pick. */
    async seed(database: DatabaseConnection, count: number): Promise<void> {
        // insert projects, then tasks and comments in batches well within the parameter budget
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
        // group some writes into one transaction, so that pages carry several changes of one commit
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
        // pick the table, whether the write deletes, and the folder
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

        // write a project
        if (kind === "project") {
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
        // write a task
        else if (kind === "task") {
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
        // write a tag
        else if (kind === "tag") {
            const id = random.pick(this.#tags);
            const values = { id, scope: folder, name: random.pick(["a", "b", "hidden"]) };
            await (isDeleted
                ? database.delete(tag).where(eq(tag.id, id))
                : database
                      .insert(tag)
                      .values(values)
                      .onConflictDoUpdate({ target: tag.id, set: values }));
        }
        // tag a task, or untag it
        else if (kind === "taskTag") {
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
        // write a page within its tree: under a lower-numbered page of its folder mostly, making deep trees, or at the top, deleting only leaves
        else if (kind === "page") {
            const index = random.integer(this.#pages.length);
            const id = this.#pages[index]!;
            const [existing] = await database.select().from(page).where(eq(page.id, id));
            const home = existing?.scope ?? folder;
            const parents = await database
                .select({ id: page.id })
                .from(page)
                .where(and(eq(page.scope, home), inArray(page.id, this.#pages.slice(0, index))));
            const parentId =
                parents.length > 0 && random.chance(0.8) ? random.pick(parents).id : null;
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
        // write a comment
        else {
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
}
