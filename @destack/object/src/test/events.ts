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
import type { Environment } from "@destack/environment";

/** Open a test store's event log over its database, recording the calls and some kinds. */
export async function testEvents(
    database: DatabaseConnection,
    environment: Pick<Environment, "clock" | "crypto" | "random">,
    kinds: readonly EventKind[] = [],
    settings?: SettingRetention,
): Promise<EventStore> {
    // store the segments in memory and seal personal values under a new keyring
    const { clock } = environment;
    const bucket = new MemoryBucket(clock);
    const keyring = await LocalKeyring.read(LocalKeyring.generate(environment), environment);

    return new EventStore({
        database,
        kinds: [call.eventKind(), ...kinds],
        files: () => bucket,
        personal: new DatabasePersonalKeyring(database, keyring, environment),
        ...(settings === undefined ? {} : { settings }),
        clock,
    });
}
