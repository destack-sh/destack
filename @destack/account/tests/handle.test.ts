import accountPackage from "../src/package.ts";
import { RequestId } from "@destack/sync";
import { expect, test } from "@destack/test";
import { AccountHandle } from "../src/address/handle.ts";
import { TestAccountStore } from "../src/test/index.ts";

test("suggest the first free number of a person's email stem past the handles accounts take", async () => {
    // take the stem's handle at sign-up and its second number with another account
    await using fixture = await TestAccountStore.open();
    const owner = await fixture.signIn("crowded@example.com");
    const personal = await owner.client(accountPackage).account.get({
        scope: owner.id,
        id: owner.account,
    });
    await owner.client(accountPackage).account.update({
        scope: owner.id,
        id: owner.account,
        requestId: RequestId.create(fixture.clock.now()),
        revision: personal.revision,
        handle: "crowded",
    });
    await owner.client(accountPackage).account.create({
        scope: owner.id,
        requestId: RequestId.create(fixture.clock.now()),
        handle: "crowded-2",
        name: "Crowded",
        kind: "shared",
        residencyId: "eu",
    });

    // suggest the third number
    expect(await owner.client(accountPackage).user.suggestHandle({ id: owner.id })).toEqual({
        handle: "crowded-3",
    });
});

test("refuse handles with doubled hyphens, which punycode labels such as xn-- start with", () => {
    const handles = ["xn--abc", "a--b", "a-b", "a"];
    expect(handles.map((handle) => AccountHandle.safeParse(handle).success)).toEqual([
        false,
        false,
        true,
        true,
    ]);
});
