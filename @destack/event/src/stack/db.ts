import type { Table } from "@destack/db";
import { eventSegment } from "../archive/catalog.ts";
import type { EventKind } from "../kind/kind.ts";
import { personalKey } from "../personal/personal.ts";

/** The tables a host keeps events of some kinds in. */
export function eventTables(kinds: readonly EventKind[]): readonly Table[] {
    return [
        ...kinds.map((kind) => kind.table),
        eventSegment,
        ...(kinds.some((kind) => kind.subject !== undefined) ? [personalKey] : []),
    ];
}
