import { expect, test } from "@destack/test";
import { readdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { CodeStorage } from "../src/codestorage/index.ts";
import { LocalGitStorage } from "../src/local/index.ts";
import { GitAdvertisement } from "../src/storage/index.ts";
import { advertise, git, History, temporary, Worktree } from "./fixture/git.ts";
import { verifyToken } from "./fixture/token.ts";

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
    const access = await storage.access(id, "push");
    const worktree = await Worktree.open();
    const commit = await worktree.commit("README.md", "# Site\n");
    const tag = await worktree.tag("v1");
    await worktree.push(access.remote, "main", "v1");
    await storage.create(id);
    expect([access.credential, await storage.references(id)]).toEqual([
        null,
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

test("create, list and delete a code.storage repository through its HTTP API and Git's advertisement, accepting each step again", async () => {
    // keep one repository's history for the stand-in to advertise
    const history = new History();
    history.commit("main", "README.md", "# Site\n");
    const repository = await temporary("destack-code-storage-");
    await git(repository, "init", "--quiet", "--bare", "--initial-branch=main");
    const [commit] = await history.write(repository);

    // stand in for exactly the endpoints the storage calls, recording each with its token's claims
    const keys = await crypto.subtle.generateKey({ name: "ECDSA", namedCurve: "P-256" }, false, [
        "sign",
        "verify",
    ]);
    const requests: unknown[] = [];
    let state: "absent" | "created" | "deleted" = "absent";
    const claimsOf = async (token: string) => {
        const { alg, claims } = await verifyToken(token, keys.publicKey);

        return [
            alg,
            claims.iss,
            claims.sub,
            claims.repo,
            claims.scopes,
            (claims.exp as number) - (claims.iat as number),
        ];
    };
    const fetch = async (input: RequestInfo | URL, options?: RequestInit) => {
        const request = new Request(input, options);
        const authorization = request.headers.get("authorization")!;
        const [scheme, credential] = authorization.split(" ") as [string, string];
        const token =
            scheme === "Basic"
                ? new TextDecoder().decode(Uint8Array.fromBase64(credential)).slice("t:".length)
                : credential;
        const body = request.method === "POST" ? await request.json() : undefined;
        requests.push([request.method, request.url, scheme, await claimsOf(token), body]);

        // create, delete and advertise as code.storage documents, answering repeated steps with their problems
        const problem = (status: number, code: string, detail: string) =>
            Response.json({ code, detail }, { status });
        if (request.method === "POST" && state === "created") {
            return problem(409, "conflict", "repository already exists");
        } else if (request.method === "POST") {
            state = "created";

            return Response.json({
                repo_name: "repository-site",
                repo_id: "repo_7f2b3d9",
                http_url: "repository-site",
                message: "repository created",
            });
        } else if (request.method === "DELETE" && state === "deleted") {
            return problem(409, "repository_deleted", "The repository is already deleted");
        } else if (request.method === "DELETE") {
            state = "deleted";

            return Response.json({
                repo_name: "repository-site",
                repo_id: "repo_7f2b3d9",
                message:
                    "Repository repository-site deletion initiated. Physical storage cleanup will complete asynchronously.",
            });
        }

        return new Response(await advertise(repository), {
            headers: { "content-type": "application/x-git-upload-pack-advertisement" },
        });
    };
    const storage = new CodeStorage({
        organization: "acme",
        key: keys.privateKey,
        api: new URL("https://api.acme.code.storage/api"),
        git: new URL("https://acme.code.storage"),
        fetch,
    });

    // create the repository twice, list its branch, grant push access and delete it twice
    const id = "repository-site";
    await storage.create(id);
    await storage.create(id);
    const listing = await storage.references(id);
    const access = await storage.access(id, "push");
    await storage.delete(id);
    await storage.delete(id);
    expect([storage.remote(id), listing, access.remote, access.credential!.username]).toEqual([
        "https://acme.code.storage/repository-site.git",
        {
            defaultReference: "refs/heads/main",
            references: [{ name: "refs/heads/main", object: commit, commit }],
        },
        "https://acme.code.storage/repository-site.git",
        "t",
    ]);
    expect(await claimsOf(access.credential!.password)).toEqual([
        "ES256",
        "acme",
        "destack-repository",
        "repository-site",
        ["git:read", "git:write"],
        3600,
    ]);

    // send each request under a token scoped to the repository and the operation
    const management = [
        "ES256",
        "acme",
        "destack-repository",
        "repository-site",
        ["repo:write"],
        60,
    ];
    const create = [
        "POST",
        "https://api.acme.code.storage/api/repos",
        "Bearer",
        management,
        { repo_name: "repository-site" },
    ];
    const remove = [
        "DELETE",
        "https://api.acme.code.storage/api/repos/repository-site",
        "Bearer",
        management,
        undefined,
    ];
    expect(requests).toEqual([
        create,
        create,
        [
            "GET",
            "https://acme.code.storage/repository-site.git/info/refs?service=git-upload-pack",
            "Basic",
            ["ES256", "acme", "destack-repository", "repository-site", ["git:read"], 3600],
            undefined,
        ],
        remove,
        remove,
    ]);
});
