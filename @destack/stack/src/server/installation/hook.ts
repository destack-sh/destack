import { present } from "@destack/schema";
import { SessionRequest, type HookSandbox } from "@destack/object";
import type { AdapterFetch } from "@destack/object/server";
import { Subject } from "@destack/sync";
import type {} from "@destack/package/import-meta";
import { StackError } from "../../error/index.ts";

/** Answer installations' hook sessions through their sandboxes, a POST of one request each. */
export function hookAdapter(sandboxes: readonly HookSandbox[]): AdapterFetch {
    return async (request, admitted) => {
        // find the calling installation's sandbox
        const caller = present(admitted, "the installation the hook adapter admits");
        const sandbox = sandboxes.find((each) => Subject.same(each.subject, caller.subject));
        if (sandbox === undefined) {
            throw StackError.of(
                "HOOKS_MISSING",
                "only installations with hooks in this store can open sessions",
            );
        }

        // answer the session's request
        const asked = SessionRequest.parse(await request.json());

        return Response.json(await sandbox.answer(asked));
    };
}
