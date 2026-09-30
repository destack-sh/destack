import { expect, test } from "@destack/test";
import { RequestId } from "@destack/service/request";
import { ServiceError } from "@destack/service/error";
import { ids, openSpace, serveSpace } from "./fixture/space.ts";

test("install an application into a space as its owner, and refuse a stranger whether its alias is taken or free", async () => {
    const client = await serveSpace(await openSpace());
    const install = (user: string, alias = "notes") =>
        client(user).installation.create({
            spaceId: ids.space,
            requestId: RequestId.create(),
            packageId: ids.package,
            selection: { kind: "release", version: "2026.9.0" },
            alias,
        });

    // install the package as an application under its alias
    const installed = await install("owner");
    expect([installed.role, installed.alias, installed.packageId, installed.status]).toEqual([
        "application",
        "notes",
        ids.package,
        "enabled",
    ]);

    // refuse a caller with no role in the space alike for a free and a taken alias
    for (const alias of ["tasks", "notes"]) {
        await expect(install("stranger", alias)).rejects.toEqual(
            new ServiceError("NOT_FOUND", { defined: true, message: `no scope ${ids.space}` }),
        );
    }
});
