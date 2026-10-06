import { Subject } from "@destack/sync";
import { eq, type Select } from "@destack/db";
import type { Call, CallOf, PreparedCallOf } from "@destack/object";
import { ServiceError } from "@destack/service/error";
import {
    ORIGIN_FIELDS,
    type OriginColumns,
    reference,
    repository,
    RepositoryOrigin,
} from "../object/index.ts";
import type { GitListing } from "../storage/index.ts";

/** The origin fields a call or a repository names, before they are read as an origin. */
export type OriginFields = { readonly [Name in (typeof ORIGIN_FIELDS)[number]]?: unknown };

/** A repository's origin, its references and its leases on the server. */
export const Repository = { authorizeWrite, update, record, readColumns, createdId, readOrigin };

/** A repository's row. */
type Row = Select<typeof repository.table>;

/** One call of a repository method. */
type RepositoryCall = Call<typeof repository.table>;

/** A call recording a listing of a repository's references. */
type RecordCall =
    | PreparedCallOf<typeof repository, "refresh">
    | CallOf<typeof repository, "report">;

/** Require the push permission for push access. */
async function authorizeWrite(call: CallOf<typeof repository, "open">): Promise<void> {
    if (call.input.mode === "write") {
        await call.requireAuthorization().require(repository.permission("push"), call.reference());
    }
}

/** Update the repository with its prepared origin, at the prepared revision. */
async function update(
    call: PreparedCallOf<typeof repository, "update">,
    next: (call?: RepositoryCall) => Promise<Row>,
): Promise<Row> {
    // change the repository alone when its origin stays
    const move = call.prepared;
    if (move === null) {
        return next();
    }

    // require the revision the origin was prepared at
    if (call.target.revision !== move.revision) {
        throw new ServiceError("CONFLICT", { message: "repository revision has changed" });
    }

    return next(call.with({ input: { ...call.input, ...move.columns } }));
}

/** Record a listing's references and default branch. */
async function record(call: RecordCall, listing: GitListing): Promise<Row> {
    // read the recorded references by name
    const { database, target } = call;
    const rows = await database
        .select()
        .from(reference.table)
        .where(eq(reference.table.parentId, target.id));
    const recorded = new Map(rows.map((row) => [row.name, row]));
    const listed = new Map(listing.references.map((entry) => [entry.name, entry]));

    // create new references, and update the references that changed
    for (const entry of listing.references) {
        if (!recorded.has(entry.name)) {
            await createReference(call, entry);
        }
    }
    for (const row of rows) {
        await updateReference(call, row, listed.get(row.name));
    }

    // move the default branch when it changed
    return target.defaultReference === listing.defaultReference
        ? target
        : call.update({ defaultReference: listing.defaultReference });
}

/** Read a repository's origin columns. */
function readColumns(target: Row): OriginColumns {
    const { hosting, remote, authentication, connectedAccountId } = target;
    const { secret, provider, providerRepositoryId } = target;

    return {
        hosting,
        machine: target.machine,
        remote,
        authentication,
        connectedAccountId,
        secret,
        provider,
        providerRepositoryId,
    };
}

/** Read the identifier of the repository a creation makes. */
function createdId(call: CallOf<typeof repository, "create">): string {
    if (call.id === undefined) {
        throw new TypeError("a repository creation has no identifier");
    }

    return call.id;
}

/** Read the origin from a call's fields and refuse invalid combinations. */
function readOrigin(fields: OriginFields): RepositoryOrigin {
    // parse the call's origin fields and read a machine's subject from its key
    const named = Object.fromEntries(
        ORIGIN_FIELDS.filter((name) => fields[name] !== undefined && fields[name] !== null).map(
            (name) => [
                name,
                name === "machine" ? Subject.read(String(fields[name])) : fields[name],
            ],
        ),
    );
    const parsed = RepositoryOrigin.safeParse(named);
    if (!parsed.success) {
        throw new ServiceError("BAD_REQUEST", {
            message: "repository origin names no platform, github, git or machine origin",
            data: { issues: parsed.error.issues },
        });
    }

    return parsed.data;
}

/** Create a reference a listing names for the first time. */
async function createReference(
    call: RecordCall,
    entry: GitListing["references"][number],
): Promise<void> {
    // NOTE #Performance: one invoked call per reference: a first refresh of 10k tags runs 10k calls
    await call.invoke(reference).create({
        parentId: call.target.id,
        name: entry.name,
        object: entry.object,
        commit: entry.commit,
        observedAt: call.now,
    });
}

/** Mark a recorded reference deleted when the listing lacks it, or move or return it when the listing changed it. */
async function updateReference(
    call: RecordCall,
    row: Select<typeof reference.table>,
    entry: GitListing["references"][number] | undefined,
): Promise<void> {
    // compare the listing with the recorded reference
    const isGone = entry === undefined && row.deletedAt === null;
    const isChanged =
        entry !== undefined &&
        (row.object !== entry.object || row.commit !== entry.commit || row.deletedAt !== null);

    // mark a vanished reference deleted, at its read revision
    if (isGone) {
        await call.invoke(reference).update({
            id: row.id,
            revision: row.revision,
            deletedAt: call.now,
        });
    }
    // move or return a changed reference, at its read revision
    else if (isChanged) {
        await call.invoke(reference).update({
            id: row.id,
            revision: row.revision,
            object: entry.object,
            commit: entry.commit,
            observedAt: call.now,
            deletedAt: null,
        });
    }
}
