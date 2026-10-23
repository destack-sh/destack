import accountPackage, { OAUTH_PATH } from "@destack/account/package";
import { PackageMount } from "@destack/object";
import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { TestUniverse } from "../src/test/index.ts";

test("start a universe over its definition, answering the account package below its mount and nothing elsewhere", async () => {
    // start the universe with a cloud machine serving its residency
    await using started = await TestUniverse.open();
    const { universe, origin } = started;

    // answer the issuer's discovery below the account package's mount, and refuse other paths
    const issuer = `${PackageMount.url(origin, accountPackage.id)}${OAUTH_PATH}`;
    const discovery = await universe.fetch(
        new Request(`${issuer}/.well-known/openid-configuration`),
    );
    const { issuer: discovered } = schema
        .looseObject({ issuer: schema.string() })
        .parse(await discovery.json());
    const outside = await universe.fetch(new Request(`${origin}/auth/sign-in`));
    await started.settle();
    expect([discovery.status, discovered, outside.status]).toEqual([200, issuer, 404]);
});
