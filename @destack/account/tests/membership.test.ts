import { RequestId, type Subject } from "@destack/sync";
import { expect, test, refusal } from "@destack/test";
import objectPackage from "@destack/object/package";
import accountPackage from "../src/package.ts";
import { space } from "../src/object/index.ts";
import {
    OWNER_EMAIL,
    STRANGER_EMAIL,
    TestAccountStore,
    type TestPerson,
} from "../src/test/index.ts";

test("let a space's account owner and its viewers read it, its guests find it alone, and a viewer granted later read it", async () => {
    // create the owner's space, and make the member its viewer and the partner its guest
    await using accounts = await TestAccountStore.open();
    const [owner, member, partner, stranger] = [
        await accounts.signIn(OWNER_EMAIL),
        await accounts.signIn("member@example.com"),
        await accounts.signIn("partner@example.com"),
        await accounts.signIn(STRANGER_EMAIL),
    ];
    const { accountId, spaceId } = await accounts.createSpace(owner);
    const grant = (relation: "viewer" | "guest", subject: Subject) =>
        owner.client(objectPackage).relationship.create({
            scope: accountId,
            requestId: RequestId.create(accounts.store.environment),
            object: space.reference(accountId, spaceId),
            relation,
            subject,
        });
    await grant("viewer", member.subject);
    await grant("guest", partner.subject);

    // read and list the space as each person
    const reads = (person: TestPerson) =>
        refusal(person.client(accountPackage).space.get({ accountId, id: spaceId }));
    const finds = async (person: TestPerson) =>
        (await person.client(accountPackage).space.list({ accountId })).items.map(
            (item) => item.id,
        );
    const decided = {
        read: [
            await reads(owner),
            await reads(member),
            await reads(partner),
            await reads(stranger),
        ],
        list: [await finds(partner), await refusal(finds(stranger))],
    };

    // let the stranger read it once the owner makes them a viewer too
    await grant("viewer", stranger.subject);
    const hidden = ["NOT_FOUND", "SCOPE_NOT_FOUND", `scope ${accountId} not found`];
    expect({ ...decided, later: await reads(stranger) }).toEqual({
        read: ["done", "done", ["NOT_FOUND", "NOT_FOUND", `space ${spaceId} not found`], hidden],
        list: [[spaceId], hidden],
        later: "done",
    });
});
