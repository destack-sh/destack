import accountPackage from "../src/package.ts";
import { present } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { expect, test } from "@destack/test";
import { Directory } from "../src/client/index.ts";
import { AccountHandle } from "../src/address/handle.ts";
import { space } from "../src/object/index.ts";
import { TestAccount } from "../src/test/index.ts";

test("suggest the first free number of a person's email stem past the handles accounts take", async () => {
    // take the stem's handle at sign-up and its second number with another account
    await using fixture = await TestAccount.open();
    const owner = await fixture.signIn("crowded@example.com");
    const personal = await owner.client(accountPackage).account.get({
        scope: owner.id,
        id: owner.account,
    });
    await owner.client(accountPackage).account.update({
        scope: owner.id,
        id: owner.account,
        requestId: RequestId.create(fixture.environment),
        revision: personal.revision,
        handle: "crowded",
    });
    await owner.client(accountPackage).account.create({
        scope: owner.id,
        requestId: RequestId.create(fixture.environment),
        handle: "crowded-2",
        name: "Crowded",
        kind: "shared",
        regionId: "eu",
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

test("keep a renamed handle naming its account and spaces until another account takes it", async () => {
    // create a space in an account, and read its address
    await using fixture = await TestAccount.open();
    const owner = await fixture.signIn("ada@example.com");
    const { accountId, handle, spaceId } = await fixture.createSpace(owner);
    const directory = new Directory(fixture.database.snapshot());
    const before = present(await directory.origin(spaceId), "the space's address");
    const rename = async (person: typeof owner, id: typeof accountId, renamed: string) => {
        const { revision } = await person
            .client(accountPackage)
            .account.get({ scope: person.id, id });
        await person.client(accountPackage).account.update({
            scope: person.id,
            id,
            requestId: RequestId.create(fixture.environment),
            revision,
            handle: renamed,
        });
    };

    // rename the account's handle, the old one still naming the account and its space
    await rename(owner, accountId, "lovelace");
    const renamed = {
        old: await directory.holder(handle),
        current: await directory.holder("lovelace"),
        space: (await directory.find(space, `${before.space}--${handle}`))?.state,
        origin: await directory.origin(spaceId),
    };

    // let another account take the old handle, which then names it alone
    const other = await fixture.signIn("babbage@example.com");
    await rename(other, other.account, handle);

    expect({ renamed, taken: await directory.holder(handle) }).toEqual({
        renamed: {
            old: { id: accountId, state: "former" },
            current: { id: accountId, state: "active" },
            space: "former",
            origin: { space: before.space, handle: "lovelace" },
        },
        taken: { id: other.account, state: "active" },
    });
});
