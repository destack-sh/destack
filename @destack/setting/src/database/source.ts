import type { RecordProvenance } from "@destack/model/source";

/** Index the declaring account, space or installation independently of its revision. */
export function sourceKey(source: RecordProvenance): string {
    if (source.kind === "account") {
        return JSON.stringify([source.kind, source.accountId]);
    } else if (source.kind === "stack") {
        return JSON.stringify([source.kind, source.spaceId]);
    } else {
        return JSON.stringify([source.kind, source.installationId]);
    }
}

/** Compare a source declaration independently of its applied revision. */
export function sameSource(left: RecordProvenance | null, right?: RecordProvenance): boolean {
    if (!left || !right || left.kind !== right.kind || left.name !== right.name) {
        return false;
    }

    if (left.kind === "account" && right.kind === "account") {
        return left.accountId === right.accountId;
    } else if (left.kind === "stack" && right.kind === "stack") {
        return left.spaceId === right.spaceId;
    } else if (left.kind === "package" && right.kind === "package") {
        return left.installationId === right.installationId;
    }

    return false;
}
