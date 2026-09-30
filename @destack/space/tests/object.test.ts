import { expect, test } from "@destack/test";
import { describeObject } from "@destack/object/inspect";
import * as object from "../src/object/index.ts";

test("describe every space object and its methods for the manifest", () => {
    const descriptions = Object.values(object)
        .filter((value) => typeof value === "object" && value !== null && "plural" in value)
        .map((value) => describeObject(value as Parameters<typeof describeObject>[0]));

    expect(descriptions.map((description) => description.name).sort()).toEqual([
        "activity",
        "binding",
        "capture",
        "checkpoint",
        "deployment",
        "installation",
        "installation-revision",
        "instance",
        "network-policy",
        "network-policy-version",
        "package-policy",
        "package-policy-version",
        "relationship",
        "resource",
        "resource-transfer",
        "restoration",
        "role",
        "run",
        "schedule",
        "snapshot",
        "space",
        "transfer",
    ]);
    const installation = descriptions.find((description) => description.name === "installation")!;
    expect(Object.keys(installation.methods)).toEqual([
        "get",
        "list",
        "create",
        "update",
        "delete",
        "plan",
        "submit",
        "approve",
        "open",
    ]);

    // page an installation's revisions through the revision object's own list
    const revision = descriptions.find(
        (description) => description.name === "installation-revision",
    )!;
    expect(revision.methods.list!.route.path).toBe(
        "/spaces/{spaceId}/installation-revisions/query",
    );
});
