import { present } from "@destack/schema";
import { SessionRequest, type HookSandbox } from "@destack/object";
import type { AdapterFetch } from "@destack/object/server";
import { CallError } from "@destack/runtime/error";
import { Subject } from "@destack/sync";

/** Answer installations' hook sessions through their sandboxes, a POST of one request each. */
export function hookAdapter(sandboxes: readonly HookSandbox[]): AdapterFetch {
    return async (request, admitted) => {
        // find the calling installation's sandbox
        const caller = present(admitted, "the installation the hook adapter admits");
        const sandbox = sandboxes.find((each) => Subject.same(each.subject, caller.subject));
        if (sandbox === undefined) {
            throw new CallError("FORBIDDEN", {
                message: "only installations with hooks in this store open sessions",
            });
        }

        // answer the session's request
        const asked = SessionRequest.parse(await request.json());

        return Response.json(await sandbox.answer(asked));
    };
}
