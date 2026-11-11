import { expect, test } from "@destack/test";
import { Failure } from "@destack/error";
import type { JsonValue } from "@destack/schema";
import { PageAuthorization } from "./authorization.ts";

/** The page's origin, which serves the request. */
const ROOT = "https://notes--home--ada.destack.space";

/** A refusal the machine answers a call with. */
function refused(reason: string, data?: JsonValue): Failure {
    return Failure.parse({
        code: "FORBIDDEN",
        reason,
        domain: "@destack/object",
        message: reason.toLowerCase(),
        ...(data === undefined ? {} : { data }),
    });
}

test("request the permission a grant lacks or a fresh sign-in from a page's person, and nothing for another refusal", () => {
    // refuse a call for a grant, a step-up, and another reason
    const permission = {
        packageId: "package-01996ab0-0000-7000-8000-000000000001",
        type: "setting",
        name: "write",
    };
    const asks = [
        refused("GRANT_DENIED", { permission, object: {}, grant: "grant-1" }),
        refused("ASSURANCE_UNAUTHENTICATED", { assurance: 2, maxAge: 900_000 }),
        refused("PERMISSION_DENIED"),
    ].map((refusal) => PageAuthorization.url(refusal, ROOT, "/editor?note=1"));

    // request through the origin's request path, returning to the page
    expect(asks).toEqual([
        `${ROOT}/.destack/request?return=%2Feditor%3Fnote%3D1&package=${permission.packageId}&type=setting&name=write`,
        `${ROOT}/.destack/request?return=%2Feditor%3Fnote%3D1&assurance=2&max-age=900000`,
        undefined,
    ]);
});
