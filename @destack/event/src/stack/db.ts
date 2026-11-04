import type { Table } from "@destack/db";
import { eventSegment, segmentDeletion } from "../archive/catalog.ts";
import type { EventKind } from "../kind/kind.ts";
import { personalKey } from "../store/personal-keyring.ts";

/** The tables of a host's events of some kinds. */
export function eventTables(kinds: readonly EventKind[]): readonly Table[] {
    return [
        ...kinds.map((kind) => kind.table),
        eventSegment,
        segmentDeletion,
        ...(kinds.some((kind) => kind.subject !== undefined) ? [personalKey] : []),
    ];
}
