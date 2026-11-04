import type { DatabaseConnection, LogClaim, Table } from "@destack/db";

/** A reader of events in commit order, its place stored in a log checkpoint. */
export class EventReader {
    /** The reader's name, unique per reader, which names its checkpoint. */
    readonly name: string;
    /** The events tables it reads, whose changes alone hold its checkpoint back. */
    readonly tables: readonly Table[];
    /** How long the log retains events past the reader's place once it moves, in milliseconds. */
    readonly hold: number;

    /** Read events tables in commit order under a checkpoint, retaining unread ones a while. */
    constructor(name: string, tables: readonly Table[], hold: number) {
        this.name = name;
        this.tables = tables;
        this.hold = hold;
    }

    /** Read the log sequence the next page starts after, the log's start for an unplaced reader. */
    async after(database: DatabaseConnection): Promise<number> {
        return (await database.log.checkpoint(this.name)) ?? 0;
    }

    /** Place the reader at the log's position now, so it reads only what commits from here on. */
    async start(now: number, database: DatabaseConnection): Promise<void> {
        const { sequence } = await database.log.position();
        await database.log.advance(this, sequence, now + this.hold);
    }

    /** Place the reader at the log's start unless placed, as its first events append. */
    async place(now: number, database: DatabaseConnection): Promise<void> {
        await database.log.place(this, now + this.hold);
    }

    /** Read the reader's claimed page, absent while it claims none. */
    async claimed(database: DatabaseConnection): Promise<LogClaim | undefined> {
        return database.log.claimed(this.name);
    }

    /** Claim the reader's page through a sequence unless claimed, absent once it moved on. */
    async claim(
        after: number,
        through: number,
        now: number,
        database: DatabaseConnection,
    ): Promise<LogClaim | undefined> {
        return database.log.claim(this, after, through, now, now + this.hold);
    }

    /** Move the reader's place past a sequence it read, ending its claim. */
    async move(from: number, to: number, now: number, database: DatabaseConnection): Promise<void> {
        await database.log.move(this, from, to, now + this.hold);
    }

    /** Let the log go of the events after the reader's place. */
    async release(database: DatabaseConnection): Promise<void> {
        await database.log.drop(this.name);
    }

    /** Read one key of the reader's family at its place, like a partition's offset. */
    at(key: string): EventReader {
        return new EventReader(`${this.name}:${key}`, this.tables, this.hold);
    }

    /** List the keys of the reader's family placed so far, in order. */
    async keys(database: DatabaseConnection): Promise<string[]> {
        const prefix = `${this.name}:`;
        const checkpoints = await database.log.checkpoints(prefix);

        return checkpoints.map((checkpoint) => checkpoint.slice(prefix.length));
    }
}
