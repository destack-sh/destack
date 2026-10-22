import { MemoryBucket } from "@destack/bucket/test";
import type { DatabaseConnection } from "@destack/db";
import {
    DatabasePersonalKeyring,
    type EventKind,
    EventStore,
    type SettingRetention,
} from "@destack/event";
import { LocalKeyring } from "@destack/identity";
import { call } from "../audit/call.ts";
import { Clock } from "@destack/environment";

/** Open a test store's event log over its database, recording the calls and some kinds. */
export async function testEvents(
    database: DatabaseConnection,
    kinds: readonly EventKind[] = [],
    clock: Clock = Clock.system,
    settings?: SettingRetention,
): Promise<EventStore> {
    // store the segments in memory and seal personal values under a new keyring
    const bucket = new MemoryBucket(clock);
    const keyring = await LocalKeyring.read(LocalKeyring.generate());

    return new EventStore({
        database,
        kinds: [call.eventKind(), ...kinds],
        files: () => bucket,
        personal: new DatabasePersonalKeyring(database, keyring),
        ...(settings === undefined ? {} : { settings }),
        clock,
    });
}
