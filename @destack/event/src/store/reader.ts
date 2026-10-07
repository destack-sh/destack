import type { DatabaseConnection } from "@destack/db";

/** A reader of events in commit order, its place kept in a log slot. */
export class EventReader {
    /** The slot's name, unique per reader. */
    readonly name: string;
    /** How long the log keeps the events after the reader's place once it advances, in milliseconds. */
    readonly hold: number;

    /** Read in commit order under a slot name, holding unread events for a while. */
    constructor(name: string, hold: number) {
        this.name = name;
        this.hold = hold;
    }

    /** Read the log sequence the reader's next page starts after, the log's start for a reader never placed. */
    async after(database: DatabaseConnection): Promise<number> {
        return (await database.log.slot(this.name)) ?? 0;
    }

    /** Place the reader at the log's position now, so it reads only what commits from here on. */
    async start(database: DatabaseConnection, now: number): Promise<void> {
        await this.advance(database, (await database.log.position()).sequence, now);
    }

    /** Keep the reader's place after a sequence, in the transaction that read up to it. */
    async advance(database: DatabaseConnection, sequence: number, now: number): Promise<void> {
        await database.log.advance(this.name, sequence, now + this.hold);
    }

    /** Let the log go of the events after the reader's place. */
    async release(database: DatabaseConnection): Promise<void> {
        await database.log.drop(this.name);
    }
}
