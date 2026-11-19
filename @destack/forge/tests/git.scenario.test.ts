import { GitAdvertisement } from "@destack/git";
import { Worktree } from "@destack/git/test";
import { RequestId } from "@destack/sync";
import { expect, test } from "@destack/test";
import forgePackage from "../src/package.ts";
import { ACCOUNT_ID, TestForge } from "../src/test/index.ts";

test("serve a platform repository's advertisement to git below its account's handle, asking anonymous callers to authenticate and concealing it from strangers", async () => {
    // create a platform repository of the publishing account, and push a commit into it
    const forge = await TestForge.open();
    const owner = forge.client(forge.owner);
    const created = await owner.repository.create({
        spaceId: forge.space,
        accountId: ACCOUNT_ID,
        requestId: RequestId.create(forge.environment),
        name: "site",
        hosting: "platform",
    });
    const lease = await owner.repository.open({
        spaceId: forge.space,
        id: created.id,
        mode: "write",
    });
    const worktree = await Worktree.open();
    const commit = await worktree.commit("README.md", "# Site\n");
    await worktree.push(lease.url, "main");

    // ask its advertisement as git does: as the owner, anonymously and as a stranger
    const path = `${forgePackage.mounts.git.path}/@example/site.git/info/refs?service=git-upload-pack`;
    const [owned, anonymous, stranger] = await Promise.all([
        forge.fetch(path, {}, forge.owner),
        forge.fetch(path),
        forge.fetch(path, {}, forge.stranger()),
    ]);

    // answer the owner the pushed branch, ask the anonymous caller for a credential, and conceal it from the stranger
    expect([
        owned.status,
        GitAdvertisement.parse(new Uint8Array(await owned.arrayBuffer())),
        anonymous.status,
        stranger.status,
    ]).toEqual([
        200,
        {
            defaultReference: "refs/heads/main",
            references: [{ name: "refs/heads/main", object: commit, commit }],
        },
        401,
        404,
    ]);
});
