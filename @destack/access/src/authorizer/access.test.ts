import { expect, onTestFinished, test } from "@destack/test";
import { eq, Snapshot } from "@destack/db";
import { schema } from "@destack/schema";
import { Subject } from "@destack/sync";
import {
    accessRelationship,
    accessRole,
    accessRolePermission,
    type AccessContext,
    Relationship,
} from "../index.ts";
import { node } from "../test/fixture.ts";
import { openFixture, userSubject } from "../test/database.ts";

/** The role bob holds on a note. */
const ROLE = schema.identifier("role").parse("role-019f5530-8000-7000-8000-000000000001");

/** The permission the role grants once regranted. */
const ROLE_PERMISSION = schema
    .identifier("role-permission")
    .parse("role-permission-019f5530-8000-7000-8000-000000000002");

/** Bob's binding of the role. */
const RELATIONSHIP = schema
    .identifier("relationship")
    .parse("relationship-019f5530-8000-7000-8000-000000000003");

test("list what a caller's access changed between two resolutions: its subjects and the bindings of regranted roles", async () => {
    // bind bob to a role on one note that grants nothing yet
    const fixture = await openFixture("sqlite");
    onTestFinished(() => fixture.close());
    const { database, authorizer, bob } = fixture;
    const at = { scope: "personal", createdAt: 1, updatedAt: 1 };
    await database
        .insert(accessRole)
        .values({ ...at, id: ROLE, name: "reader", description: "", isUniversal: false });
    await database.insert(accessRelationship).values(
        Relationship.encode(
            {
                id: RELATIONSHIP,
                object: node.reference("personal", "a"),
                role: ROLE,
                subject: userSubject("bob"),
                createdAt: 1,
                expiresAt: null,
            },
            "personal",
        ),
    );
    const resolve = (context: AccessContext) =>
        authorizer.resolve(Snapshot.live(database), "personal", context);
    const before = await resolve(bob);

    // let the role grant viewing, and add carol to bob's subjects
    await database.insert(accessRolePermission).values({
        id: ROLE_PERMISSION,
        roleId: ROLE,
        scope: "personal",
        packageId: node.definition.packageId,
        type: node.name,
        name: "view",
    });
    const after = await resolve({ ...bob, subjects: [...bob.subjects, userSubject("carol")] });

    // regrant the bound role, keep the chain, and add carol alone
    expect({
        regranted: after.regranted(before).map((binding) => Relationship.objectOf(binding)),
        unchanged: after.regranted(after),
        chain: after.chainKey === before.chainKey,
        subjects: after.changedSubjects(before).map((subject) => Subject.key(subject)),
    }).toEqual({
        regranted: [node.reference("personal", "a")],
        unchanged: [],
        chain: true,
        subjects: [Subject.key(userSubject("carol"))],
    });

    // keep the role's grant as it was once its permission goes again
    await database.delete(accessRolePermission).where(eq(accessRolePermission.roleId, ROLE));
    expect((await resolve(bob)).regranted(before)).toEqual([]);
});
