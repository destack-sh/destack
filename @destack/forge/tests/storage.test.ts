import { expect, refusal, test } from "@destack/test";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { ResourceId } from "@destack/resource";
import { Simulation } from "@destack/environment/simulation";
import { testSimulation } from "@destack/object/test";
import { nth, present } from "@destack/schema";
import {
    ARTIFACTS_PROVIDER,
    type ArtifactsNamespace,
    type ArtifactsRepositoryHandle,
    ArtifactsStorage,
    artifactsProvider,
} from "../src/cloudflare/index.ts";
import { repositoryStorage } from "../src/stack/storage.ts";
import { GitLease, LocalGitStorage } from "../src/local/index.ts";
import { GitAdvertisement } from "../src/storage/index.ts";
import { advertise, git, History, temporary, Worktree } from "./fixture/git.ts";

/** The host the local repositories live on, a world's node over the host's files and git. */
const host = testSimulation().environment("storage");

test("create a local repository, push a branch and a tag into it, list them and delete it, each step again as well", async () => {
    const storage = new LocalGitStorage(await temporary("destack-storage-"), host);
    const id = "repository-site";

    // create an empty repository with HEAD at a branch that does not exist yet
    const remote = await storage.create(id);
    expect([remote, await storage.references(id)]).toEqual([
        `file://${storage.directory}/repository-site.git`,
        { defaultReference: null, references: [] },
    ]);

    // push a commit and an annotated tag with plain git, list them, and leave them on a second create
    const access = await storage.open(id, "write");
    const worktree = await Worktree.open();
    const commit = await worktree.commit("README.md", "# Site\n");
    const tag = await worktree.tag("v1");
    await worktree.push(access.url, "main", "v1");
    expect([await storage.create(id), access, await storage.references(id)]).toEqual([
        remote,
        { url: remote, mode: "write", headers: {} },
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
    expect(await host.files.readDirectory(storage.directory)).toEqual([]);
});

test("parse the reference advertisement git upload-pack serves, peeling annotated tags and tags of tags", async () => {
    // write a branch, an annotated tag and a tag of that tag into a local repository
    const storage = new LocalGitStorage(await temporary("destack-storage-"), host);
    const repository = fileURLToPath(await storage.create("repository-advertised"));
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

test("create, list, lease and delete an Artifacts repository through its namespace binding, refusing a second create", async () => {
    // store one repository's history for the remote to advertise
    const history = new History();
    history.commit("main", "README.md", "# Site\n");
    const advertised = await temporary("destack-artifacts-");
    await git(advertised, "init", "--quiet", "--bare", "--initial-branch=main");
    const [commit] = await history.write(advertised);

    // answer Git's advertisement at the remote, recording each request's credential
    const fetched: unknown[] = [];
    const fetch = async (request: Request) => {
        fetched.push([request.url, request.headers.get("authorization")]);

        return new Response(await advertise(advertised), {
            headers: { "content-type": "application/x-git-upload-pack-advertisement" },
        });
    };

    // create the repository, refuse it again, list its branch, lease push access and delete it twice
    const namespace = new TestArtifactsNamespace();
    const storage = new ArtifactsStorage(namespace, "git-site", fetch);
    const remote = await storage.create("repository-site");
    const duplicate = await storage.create("repository-site").catch((error: unknown) => error);
    const listing = await storage.references("repository-site");
    const lease = await storage.open("repository-site", "write");
    await storage.delete("repository-site");
    await storage.delete("repository-site");

    // answer the remote Artifacts gave, and lease it with Bearer tokens Artifacts issued
    const created = "https://acme.artifacts.cloudflare.net/git/sites/git-site.repository-site.git";
    expect([remote, duplicate, listing, lease]).toEqual([
        created,
        artifactsError("ALREADY_EXISTS", "git-site.repository-site"),
        {
            defaultReference: "refs/heads/main",
            references: [{ name: "refs/heads/main", object: commit, commit }],
        },
        {
            url: created,
            mode: "write",
            headers: { authorization: "Bearer art_v1_2?expires=1791000000" },
            expiresAt: Date.parse("2026-10-03T06:00:00Z"),
        },
    ]);

    // call exactly the binding's documented methods, disposing each handle, and read Git with the read token
    expect([namespace.calls, fetched]).toEqual([
        [
            ["create", "git-site.repository-site"],
            ["create", "git-site.repository-site"],
            ["get", "git-site.repository-site"],
            ["createToken", "read", 3600],
            ["dispose"],
            ["get", "git-site.repository-site"],
            ["createToken", "write", 3600],
            ["dispose"],
            ["delete", "git-site.repository-site", true],
            ["delete", "git-site.repository-site", false],
        ],
        [[`${created}/info/refs?service=git-upload-pack`, "Bearer art_v1_1?expires=1791000000"]],
    ]);
});

test("refuse a repository Artifacts created under another name, and a token with an unreadable expiry", async () => {
    // rename created repositories in one namespace, and issue tokens without a date in another
    const renaming = new TestArtifactsNamespace({ rename: true });
    const undated = new TestArtifactsNamespace({ expiresAt: "soon" });
    const storage = new ArtifactsStorage(undated, "git-site", unreachable);
    await storage.create("repository-site");

    // refuse both answers as bad gateways
    expect([
        await refusal(
            new ArtifactsStorage(renaming, "git-site", unreachable).create("repository-site"),
        ),
        await refusal(storage.open("repository-site", "read")),
    ]).toEqual([
        [
            "BAD_GATEWAY",
            "ARTIFACT_NAME_MISMATCH",
            "expected cloudflare artifacts to create git-site.repository-site, got renamed",
        ],
        [
            "BAD_GATEWAY",
            "TOKEN_EXPIRY_INVALID",
            "cloudflare artifacts issued a token with an invalid expiry: soon",
        ],
    ]);
});

test("provision a Git storage below its resource in the bound namespace, connect to it and destroy only its repositories across pages", async () => {
    // provision two resources in one namespace, each named by its identifier
    const namespace = new TestArtifactsNamespace();
    const provider = artifactsProvider(namespace, "git", unreachable);
    const provision = present(provider.provision, "the artifacts provisioner");
    const resource = ResourceId.parse("git-01996ab0-0000-7000-8000-000000000003");
    const other = ResourceId.parse("git-01996ab0-0000-7000-8000-000000000004");
    const record = { id: resource, scope: "space", spec: {}, reference: resource };
    const placement = await provision.provision(record);

    // connect to the first resource through its binding, and create repositories in both
    const connector = ArtifactsStorage.connector(namespace, unreachable);
    const binding = { resource, kind: "git", provider: ARTIFACTS_PROVIDER, ...placement };
    await using storage = await connector.connect(
        binding,
        repositoryStorage,
        new Simulation({ seed: 1, start: 0 }).environment("storage"),
    );
    const neighbour = new ArtifactsStorage(namespace, other, unreachable);
    for (const id of ["repository-a", "repository-b", "repository-c"]) {
        await storage.create(id);
    }
    await neighbour.create("repository-a");

    // destroy the first resource's repositories, listing pages of two
    namespace.calls.length = 0;
    await provision.destroy(record);
    expect([placement, storage.provider, namespace.names(), namespace.calls]).toEqual([
        { reference: resource },
        "cloudflare-artifacts",
        [`${other}.repository-a`],
        [
            ["list", 200, undefined],
            ["list", 200, "2"],
            ["delete", `${resource}.repository-a`, true],
            ["delete", `${resource}.repository-b`, true],
            ["delete", `${resource}.repository-c`, true],
        ],
    ]);
});

test("check one commit of a leased repository out into a new working tree without its history", async () => {
    // write two commits on main into a local repository
    const storage = new LocalGitStorage(await temporary("destack-storage-"), host);
    const remote = await storage.create("repository-leased");
    const history = new History();
    history.commit("main", "README.md", "# Site\n");
    history.commit("main", "README.md", "# Site, again\n");
    const written = await history.write(fileURLToPath(remote));
    const first = nth(written, 0);

    // check the first commit out through a read lease, alone
    const lease = await storage.open("repository-leased", "read");
    const directory = await temporary("destack-leased-");
    await GitLease.checkout(lease, first, directory, host, new AbortController().signal);
    expect([
        await host.files.readTextFile(join(directory, "README.md")),
        await git(directory, "rev-parse", "HEAD"),
        await git(directory, "rev-list", "--count", "HEAD"),
    ]).toEqual(["# Site\n", first, "1"]);
});

/** Refuse a request, as these tests' storages create, list and delete without reaching Git. */
async function unreachable(request: Request): Promise<Response> {
    throw new TypeError(`the test reaches no Git remote, not ${request.url}`);
}

/** An Artifacts namespace storing repositories in memory as the Workers binding documents them. */
class TestArtifactsNamespace implements ArtifactsNamespace {
    /** The calls the storage made, in order. */
    readonly calls: unknown[][] = [];
    /** The repositories by name. */
    readonly #repositories = new Set<string>();
    /** The tokens issued so far. */
    #tokens = 0;
    /** How the namespace misanswers: renaming creates, and the expiry of its tokens. */
    readonly #answers: { readonly rename: boolean; readonly expiresAt: string };

    /** Answer as documented, or misanswer as asked. */
    constructor(answers: { readonly rename?: boolean; readonly expiresAt?: string } = {}) {
        this.#answers = {
            rename: answers.rename ?? false,
            expiresAt: answers.expiresAt ?? "2026-10-03T06:00:00Z",
        };
    }

    /** Create a repository, refusing a name in use. */
    async create(name: string) {
        this.calls.push(["create", name]);
        if (this.#repositories.has(name)) {
            throw artifactsError("ALREADY_EXISTS", name);
        }
        this.#repositories.add(name);
        const created = this.#answers.rename ? "renamed" : name;

        return { name: created, remote: TestArtifactsNamespace.remote(created) };
    }

    /** Open a repository's handle, refusing a missing one. */
    async get(name: string): Promise<ArtifactsRepositoryHandle> {
        // open a handle of a stored repository
        this.calls.push(["get", name]);
        if (!this.#repositories.has(name)) {
            throw artifactsError("NOT_FOUND", name);
        }

        // issue tokens and record the handle's disposal
        const calls = this.calls;
        const answers = this.#answers;
        const issue = () => {
            this.#tokens += 1;

            return `art_v1_${this.#tokens}?expires=1791000000`;
        };

        return {
            remote: TestArtifactsNamespace.remote(name),
            createToken: async (scope, ttl) => {
                calls.push(["createToken", scope, ttl]);

                return { plaintext: issue(), expiresAt: answers.expiresAt };
            },
            [Symbol.dispose]: () => {
                calls.push(["dispose"]);
            },
        };
    }

    /** List a page of two repositories, ordered by name. */
    async list(options: { readonly limit: number; readonly cursor?: string }) {
        this.calls.push(["list", options.limit, options.cursor]);
        const start = Number(options.cursor ?? "0");
        const names = this.names();
        const repos = names.slice(start, start + 2).map((name) => ({ name }));

        return start + 2 < names.length ? { repos, cursor: String(start + 2) } : { repos };
    }

    /** Delete a repository, answering whether it existed. */
    async delete(name: string) {
        const isDeleted = this.#repositories.delete(name);
        this.calls.push(["delete", name, isDeleted]);

        return isDeleted;
    }

    /** List the stored names in order. */
    names(): string[] {
        return [...this.#repositories].toSorted();
    }

    /** Build the remote the fake gives a repository. */
    static remote(name: string): string {
        return `https://acme.artifacts.cloudflare.net/git/sites/${name}.git`;
    }
}

/** Build the error the binding throws, with its code. */
function artifactsError(code: string, name: string): Error {
    return Object.assign(new Error(`${code}: ${name}`), { code });
}
