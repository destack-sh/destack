import { expect, test } from "@destack/test";
import { ObjectReference } from "./reference.ts";

test("read the parent of a nested row in the row's scope", () => {
    const row = {
        id: "remark-1",
        scope: "space-1",
        parentPackageId: ObjectReference.shape.packageId.parse(
            "package-01996ab0-0000-7000-8000-000000000009",
        ),
        parentType: "article",
        parentId: "article-1",
    };
    expect(ObjectReference.parent(row)).toEqual({
        packageId: "package-01996ab0-0000-7000-8000-000000000009",
        type: "article",
        scope: "space-1",
        id: "article-1",
    });
});
