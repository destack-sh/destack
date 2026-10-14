import type { DatabaseConnection } from "@destack/db";

/** A reader of events in commit order, its place kept in a log slot. */
export class EventReader {
    /** The slot's name, unique per reader. */
    readonly name: string;
    /** How long the log keeps events past the reader's place once it advances, in milliseconds. */
    readonly hold: number;

    /** Read in commit order under a slot name, holding unread events for a while. */
    constructor(name: string, hold: number) {
        this.name = name;
        this.hold = hold;
    }

    /** Read the log sequence the next page starts after, the log's start for an unplaced reader. */
    async after(database: DatabaseConnection): Promise<number> {
        return (await database.log.slot(this.name)) ?? 0;
    }

    /** Place the reader at the log's position now, so it reads only what commits from here on. */
    async start(now: number, database: DatabaseConnection): Promise<void> {
        const { sequence } = await database.log.position();
        await database.log.advance(this.name, sequence, now + this.hold);
    }

    /** Place the reader at the log's start unless placed, as its first events append. */
    async place(now: number, database: DatabaseConnection): Promise<void> {
        await database.log.place(this.name, now + this.hold);
    }

    /** Move the reader's place past a sequence, in the transaction that read up to it. */
    async move(from: number, to: number, now: number, database: DatabaseConnection): Promise<void> {
        await database.log.move(this.name, from, to, now + this.hold);
    }

    /** Let the log go of the events after the reader's place. */
    async release(database: DatabaseConnection): Promise<void> {
        await database.log.drop(this.name);
    }

    /** Read one key of the reader's family at its own place, like a partition's offset. */
    at(key: string): EventReader {
        return new EventReader(`${this.name}:${key}`, this.hold);
    }

    /** List the keys of the reader's family placed so far, in order. */
    async keys(database: DatabaseConnection): Promise<string[]> {
        const prefix = `${this.name}:`;
        const slots = await database.log.slots(prefix);

        return slots.map((slot) => slot.slice(prefix.length));
    }
}
