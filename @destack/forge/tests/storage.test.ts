import { expect, test } from "@destack/test";
import { readdir, readFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { ServiceError } from "@destack/service/error";
import { ArtifactsStorage } from "../src/artifacts/index.ts";
import { GitLease, LocalGitStorage } from "../src/local/index.ts";
import { GitAdvertisement } from "../src/storage/index.ts";
import { advertise, git, History, temporary, Worktree } from "./fixture/git.ts";

test("create a local repository, push a branch and a tag into it, list them and delete it, each step again as well", async () => {
    const storage = new LocalGitStorage(await temporary("destack-storage-"));
    const id = "repository-site";

    // create an empty repository with HEAD at a branch that does not exist yet
    await storage.create(id);
    expect([storage.remote(id), await storage.references(id)]).toEqual([
        `file://${storage.directory}/repository-site.git`,
        { defaultReference: null, references: [] },
    ]);

    // push a commit and an annotated tag with plain git, list them, and keep them on a second create
    const access = await storage.open(id, "write");
    const worktree = await Worktree.open();
    const commit = await worktree.commit("README.md", "# Site\n");
    const tag = await worktree.tag("v1");
    await worktree.push(access.url, "main", "v1");
    await storage.create(id);
    expect([access, await storage.references(id)]).toEqual([
        { url: storage.remote(id), mode: "write", headers: {} },
        {
            defaultReference: "refs/heads/main",
            references: [
                { name: "refs/heads/main", object: commit, commit },
                { name: "refs/tags/v1", object: tag, commit },
            ],
        },
    ]);

    // refuse names leaving the directory, and delete the repository twice
    await expect(storage.create("../escape")).rejects.toThrow(
        "invalid local repository name: ../escape",
    );
    await storage.delete(id);
    await storage.delete(id);
    expect(await readdir(storage.directory)).toEqual([]);
});

test("parse the reference advertisement git upload-pack serves, peeling annotated tags and tags of tags", async () => {
    // write a branch, an annotated tag and a tag of that tag into a local repository
    const storage = new LocalGitStorage(await temporary("destack-storage-"));
    await storage.create("repository-advertised");
    const repository = fileURLToPath(storage.remote("repository-advertised"));
    const history = new History();
    const commit = history.commit("main", "README.md", "# Site\n");
    const tag = history.tag("v1", commit);
    history.tag("signed", tag);
    const [main, v1, signed] = await history.write(repository);

    // read every reference with the commit it resolves to, as the local storage lists them too
    const listing = GitAdvertisement.parse(await advertise(repository));
    expect(listing).toEqual({
        defaultReference: "refs/heads/main",
        references: [
            { name: "refs/heads/main", object: main, commit: main },
            { name: "refs/tags/signed", object: signed, commit: main },
            { name: "refs/tags/v1", object: v1, commit: main },
        ],
    });
    expect(await storage.references("repository-advertised")).toEqual(listing);
});

test("create, list and delete a Cloudflare Artifacts repository through its REST API and Git's advertisement, accepting each step again", async () => {
    // keep one repository's history for the stand-in to advertise
    const history = new History();
    history.commit("main", "README.md", "# Site\n");
    const repository = await temporary("destack-artifacts-");
    await git(repository, "init", "--quiet", "--bare", "--initial-branch=main");
    const [commit] = await history.write(repository);

    // stand in for exactly the endpoints the storage calls, recording each with its credential
    const requests: unknown[] = [];
    let state: "absent" | "created" | "deleted" = "absent";
    const api = "https://api.cloudflare.com/client/v4/accounts/acme/artifacts/namespaces/sites";
    const fetch = async (input: RequestInfo | URL, options?: RequestInit) => {
        const request = new Request(input, options);
        const body: unknown = request.method === "POST" ? await request.json() : undefined;
        requests.push([request.method, request.url, request.headers.get("authorization"), body]);

        // create, read, delete and issue tokens as Artifacts documents them
        if (request.url === `${api}/repos` && state === "created") {
            return envelope(409, null);
        } else if (request.url === `${api}/repos`) {
            state = "created";

            return envelope(200, { id: "repo_1", name: "repository-site" });
        } else if (request.method === "GET" && request.url === `${api}/repos/repository-site`) {
            return envelope(state === "created" ? 200 : 404, { name: "repository-site" });
        } else if (request.method === "DELETE" && state !== "created") {
            return envelope(404, null);
        } else if (request.method === "DELETE") {
            state = "deleted";

            return envelope(202, { id: "repo_1" });
        } else if (request.url === `${api}/tokens`) {
            return envelope(200, {
                id: "token_1",
                plaintext: "art_v1_secret?expires=1",
                scope: "read",
                expires_at: "2026-10-03T06:00:00Z",
            });
        }

        return new Response(await advertise(repository), {
            headers: { "content-type": "application/x-git-upload-pack-advertisement" },
        });
    };
    const storage = new ArtifactsStorage({
        account: "acme",
        namespace: "sites",
        token: "api",
        fetch,
    });

    // create the repository twice, list its branch, grant push access and delete it twice
    const id = "repository-site";
    await storage.create(id);
    await storage.create(id);
    const listing = await storage.references(id);
    const access = await storage.open(id, "write");
    await storage.delete(id);
    await storage.delete(id);
    const remote = "https://acme.artifacts.cloudflare.net/git/sites/repository-site.git";
    const basic = `Basic ${new TextEncoder().encode("x:art_v1_secret?expires=1").toBase64()}`;
    expect([storage.remote(id), listing, access]).toEqual([
        remote,
        {
            defaultReference: "refs/heads/main",
            references: [{ name: "refs/heads/main", object: commit, commit }],
        },
        {
            url: remote,
            mode: "write",
            headers: { authorization: basic },
            expiresAt: Date.parse("2026-10-03T06:00:00Z"),
        },
    ]);

    // manage under the account's token and read Git under the repository's token
    const token = (scope: string) => [
        "POST",
        `${api}/tokens`,
        "Bearer api",
        { repo: id, scope, ttl: 3600 },
    ];
    expect(requests).toEqual([
        ["POST", `${api}/repos`, "Bearer api", { name: id }],
        ["POST", `${api}/repos`, "Bearer api", { name: id }],
        ["GET", `${api}/repos/${id}`, "Bearer api", undefined],
        token("read"),
        ["GET", `${remote}/info/refs?service=git-upload-pack`, basic, undefined],
        token("write"),
        ["DELETE", `${api}/repos/${id}`, "Bearer api", undefined],
        ["DELETE", `${api}/repos/${id}`, "Bearer api", undefined],
    ]);
});

test("refuse a Cloudflare Artifacts token whose expiry is no time", async () => {
    // issue a token with an unreadable expiry
    const storage = new ArtifactsStorage({
        account: "acme",
        namespace: "sites",
        token: "api",
        fetch: async () => envelope(200, { plaintext: "art_v1_secret", expires_at: "soon" }),
    });

    // refuse leasing it
    await expect(storage.open("repository-site", "read")).rejects.toEqual(
        new ServiceError("BAD_GATEWAY", {
            message: "cloudflare artifacts issued a token with an invalid expiry: soon",
        }),
    );
});

test("check one commit of a leased repository out into a new working tree without its history", async () => {
    // push two commits on main into a local repository
    const storage = new LocalGitStorage(await temporary("destack-storage-"));
    await storage.create("repository-leased");
    const worktree = await Worktree.open();
    const first = await worktree.commit("README.md", "# Site\n");
    await worktree.commit("README.md", "# Site, again\n");
    await worktree.push(storage.remote("repository-leased"), "main");

    // check the first commit out through a read lease, alone
    const lease = await storage.open("repository-leased", "read");
    const directory = await temporary("destack-leased-");
    await GitLease.checkout(lease, first, directory, new AbortController().signal);
    expect([
        await readFile(join(directory, "README.md"), "utf8"),
        await git(directory, "rev-parse", "HEAD"),
        await git(directory, "rev-list", "--count", "HEAD"),
    ]).toEqual(["# Site\n", first, "1"]);
});

/** Answer a Cloudflare API request with its envelope. */
function envelope(status: number, result: unknown): Response {
    const isSuccess = status < 300;

    return Response.json(
        {
            success: isSuccess,
            errors: isSuccess ? [] : [{ code: status, message: "refused" }],
            result,
        },
        { status },
    );
}
