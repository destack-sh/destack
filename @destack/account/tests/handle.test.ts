import { expect, test } from "@destack/test";
import { RequestId } from "@destack/service/request";
import { account } from "../src/object/account.ts";
import { AccountFixture, id } from "./fixture.ts";

/** Suggest a free handle, check typed ones, and claim one with the personal account before any other account. */
test("choose a handle before anything else: suggested from the email, checked as typed, claimed with the personal account", async () => {
    await using fixture = await AccountFixture.open();
    const owner = await fixture.signIn("Owner.Name@example.com");
    const creation = {
        scope: owner.id,
        name: "Owner",
        defaultResidency: "eu" as const,
    };
    const handle = async () => (await owner.client.authentication.current()).profile.handle;

    // start without a handle, and refuse other accounts until the user chooses one
    expect(await handle()).toBeNull();
    await expect(
        owner.client.account.create({
            ...creation,
            requestId: RequestId.create(),
            handle: "team",
            kind: "shared",
        }),
    ).rejects.toMatchObject({
        code: "PRECONDITION_FAILED",
        message: "choose a handle first to create the personal account",
    });

    // suggest the email's local part made valid, and check handles as the user types them
    expect(await owner.client.user.suggestHandle({ id: owner.id })).toEqual({
        handle: "owner-name",
    });
    const check = async (typed: string) =>
        (await owner.client.user.checkHandle({ id: owner.id, handle: typed })).availability;
    expect([await check("owner-name"), await check("Owner!"), await check("-owner")]).toEqual([
        "available",
        "invalid",
        "invalid",
    ]);

    // refuse the personal account before a residency, choose one, then keep it
    const claim = () =>
        owner.client.account.create({
            scope: owner.id,
            name: "Owner",
            requestId: RequestId.create(),
            handle: "owner-name",
            kind: "personal",
        });
    await expect(claim()).rejects.toMatchObject({
        code: "PRECONDITION_FAILED",
        message: "choose a residency with the handle first",
    });
    const choose = async (residency: "eu" | "us") => {
        const current = await owner.client.user.get({ id: owner.id });

        return owner.client.user.update({
            id: owner.id,
            requestId: RequestId.create(),
            revision: current.revision,
            residency,
        });
    };
    await choose("us");
    await expect(choose("eu")).rejects.toMatchObject({
        code: "CONFLICT",
        message: "the residency follows the home space",
    });

    // claim the handle with the personal account at the user's residency
    const personal = await claim();
    expect([personal.defaultResidency, await handle(), await check("owner-name")]).toEqual([
        "us",
        "owner-name",
        "taken",
    ]);

    // number the suggestion past a taken handle, and hide the owner's suggestions from others
    const namesake = await fixture.signIn("owner.name@elsewhere.test");
    expect(await namesake.client.user.suggestHandle({ id: namesake.id })).toEqual({
        handle: "owner-name-2",
    });
    await expect(namesake.client.user.suggestHandle({ id: owner.id })).rejects.toMatchObject({
        code: "NOT_FOUND",
        message: `no user ${owner.id}`,
    });
});

/** Suggest the first free number of a stem past a whole batch of taken ones. */
test("suggest the first free handle past a hundred taken numbers of the stem", async () => {
    await using fixture = await AccountFixture.open();
    const owner = await fixture.signIn("crowded@example.com");

    // take the stem and its numbers 2 to 100 with accounts of the owner
    const now = Date.now();
    await fixture.opened.database.insert(account.table).values(
        Array.from({ length: 100 }, (_, index) => ({
            id: id("account"),
            createdAt: now,
            updatedAt: now,
            scope: owner.id,
            handle: index === 0 ? "crowded" : `crowded-${index + 1}`,
            name: "Crowded",
            defaultResidency: "eu" as const,
        })),
    );

    // suggest the number the second batch of candidates finds free
    expect(await owner.client.user.suggestHandle({ id: owner.id })).toEqual({
        handle: "crowded-101",
    });
});
