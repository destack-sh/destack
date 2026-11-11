import { present } from "@destack/schema";
import { SessionRequest } from "@destack/object";
import type { AdapterFetch } from "@destack/object/server";
import { Subject } from "@destack/sync";
import type {} from "@destack/package/import-meta";
import { StackError } from "../../error/index.ts";

/** Answer an installation's hook session through the store's sandbox of it, one request a POST. */
export const hookAdapter: AdapterFetch = async (request, admitted, _context, store) => {
    // find the calling installation's sandbox
    const caller = present(admitted, "the installation the hook adapter admits");
    const sandbox = store.sandboxes.find((each) => Subject.same(each.subject, caller.subject));
    if (sandbox === undefined) {
        throw StackError.of(
            "HOOKS_DENIED",
            "session is forbidden: only installations with hooks in this store can open sessions",
        );
    }

    // answer the session's request
    const asked = SessionRequest.parse(await request.json());

    return Response.json(await sandbox.answer(asked));
};
