import { present } from "@destack/schema";
import { SessionRequest } from "@destack/object";
import type { MountFetch } from "@destack/object/server";
import { Subject } from "@destack/sync";
import type {} from "@destack/package/import-meta";
import { StackError } from "@destack/stack/error";

/** Answer an installation's hook session through the store's isolate of it, one request a POST. */
export const hookMount: MountFetch = async (request, admitted, _context, store) => {
    // find the calling installation's isolate
    const caller = present(admitted, "the installation the hook mount admits");
    const isolate = store.isolates.find((each) => Subject.same(each.subject, caller.subject));
    if (isolate === undefined) {
        throw StackError.of(
            "HOOKS_DENIED",
            "session is forbidden: only installations with hooks in this store can open sessions",
        );
    }

    // answer the session's request
    const asked = SessionRequest.parse(await request.json());

    return Response.json(await isolate.answer(asked));
};
