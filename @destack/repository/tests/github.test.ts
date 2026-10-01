import { AuditCaller } from "@destack/audit";
import { expect, test } from "@destack/test";
import { TEST_DIALECTS } from "@destack/db/test";
import { principal } from "@destack/access";
import { Condition } from "@destack/db/query";
import { RequestId } from "@destack/service/request";
import { WEBHOOK_SIGNATURES } from "@destack/service/trigger";
import { GitHubApp } from "../src/github/index.ts";
import { APP_ID, GITHUB_API, GitHubStandIn } from "./fixture/github.ts";
import { githubPrivateKey } from "./fixture/key.ts";
import { ids, INSTALLATION, OTHER_INSTALLATION, RepositoryFixture } from "./fixture/repository.ts";

/** Commits and tag objects of the fixture GitHub repository. */
const objects = {
    first: "aa218f56b14c9653891f9e74264a383fa43fefbd",
    second: "6dcb09b5b57875f334f61aebed695e2e4193db5e",
    third: "0d1a26e67d8f5eaf1f6ba5c57fc3c7d91ac0fd1c",
    release: "940bd336248efae0f9ee5bc7b2d5c985887b16ac",
    patch: "c3d0be41ecbe669545ee3e94d31ed9a4bc91ee3c",
};

/** GitHub's identifier of the fixture repository. */
const REPOSITORY_ID = 1296269;

/** The secret the fixture GitHub App signs webhook deliveries with. */
const WEBHOOK_SECRET = "It's a Secret to Everybody";

/** Keep acme/site at GitHub: main and next, an annotated v1 and a lightweight v0. */
function site(github: GitHubStandIn) {
    github.add({
        id: REPOSITORY_ID,
        fullName: "acme/site",
        installation: INSTALLATION,
        defaultBranch: "main",
        references: new Map([
            ["refs/heads/main", { type: "commit", sha: objects.first }],
            ["refs/heads/next", { type: "commit", sha: objects.second }],
            ["refs/tags/v0", { type: "commit", sha: objects.first }],
            ["refs/tags/v1", { type: "tag", sha: objects.release }],
        ]),
        tags: new Map([[objects.release, { type: "commit", sha: objects.first }]]),
    });
}

/** The repository and installation objects GitHub's repository events carry. */
function source(installation: number) {
    return {
        repository: {
            id: REPOSITORY_ID,
            node_id: "MDEwOlJlcG9zaXRvcnkxMjk2MjY5",
            name: "site",
            full_name: "acme/site",
            private: true,
            owner: { login: "acme", id: 9919, type: "Organization" },
            html_url: "https://github.com/acme/site",
            default_branch: "main",
            master_branch: "main",
        },
        sender: { login: "ada", id: 583231, type: "User" },
        installation: { id: installation, node_id: "MDIzOkludGVncmF0aW9uSW5zdGFsbGF0aW9uNDI0Mg==" },
    };
}

/** Sign a delivery as GitHub sends it to the webhook's route. */
async function deliver(id: string, event: string, payload: unknown, secret = WEBHOOK_SECRET) {
    const body = JSON.stringify(payload);
    const headers = await WEBHOOK_SIGNATURES.github.sign(
        { id, event, body, sentAt: Date.now() },
        secret,
    );
    headers.set("content-type", "application/json");

    return new Request("https://hooks.destack.test/webhooks/github", {
        method: "POST",
        headers,
        body,
    });
}

test("identify a repository and mint installation tokens limited to it and to the permissions each use needs", async () => {
    const github = new GitHubStandIn();
    site(github);
    const app = new GitHubApp({
        id: APP_ID,
        key: await githubPrivateKey(),
        api: GITHUB_API,
        fetch: github.fetch,
    });

    // identify the repository by name, then lease writing to it by identifier
    const fullName = GitHubApp.fullName("https://github.com/acme/site.git");
    expect([
        await app.repository(String(INSTALLATION), fullName),
        await app.open(
            String(INSTALLATION),
            String(REPOSITORY_ID),
            "https://github.com/acme/site.git",
            "write",
        ),
    ]).toEqual([
        { id: String(REPOSITORY_ID), fullName: "acme/site" },
        {
            url: "https://github.com/acme/site.git",
            mode: "write",
            headers: {
                authorization: `Basic ${new TextEncoder().encode("x-access-token:ghs_4242_2").toBase64()}`,
            },
            expiresAt: Date.parse("2026-09-27T13:00:00Z"),
        },
    ]);

    // call the app endpoint with the app JWT and the repository with a metadata token
    expect(github.requests).toEqual([
        "POST api.github.test/app/installations/4242/access_tokens app",
        '  body {"repositories":["site"],"permissions":{"metadata":"read"}}',
        "GET api.github.test/repos/acme/site installation 4242",
        "POST api.github.test/app/installations/4242/access_tokens app",
        '  body {"repository_ids":[1296269],"permissions":{"contents":"write"}}',
    ]);

    // refuse remotes outside github.com
    expect(() => GitHubApp.fullName("https://gitlab.com/acme/site.git")).toThrow(
        "not a github.com repository: https://gitlab.com/acme/site.git",
    );
});

test.each(TEST_DIALECTS)(
    "connect a GitHub repository, refresh it, and refresh it again as the system on push, create and delete deliveries on %s",
    async (dialect) => {
        const region = await RepositoryFixture.open(dialect);
        site(region.github);
        const owner = region.connect(ids.owner);
        const accountId = ids.account;

        // connect acme/site through the account's installation, recording GitHub's identifier
        const created = await owner.repository.create({
            accountId,
            requestId: RequestId.create(),
            name: "site",
            hosting: "github",
            remote: "https://github.com/acme/site.git",
            connectedAccountId: ids.connection,
        });
        expect([created.provider, created.providerRepositoryId]).toEqual([null, "1296269"]);
        await owner.repository.refresh({
            accountId,
            id: created.id,
            requestId: RequestId.create(),
        });
        const references = async () =>
            (
                await owner.reference.list({
                    accountId,
                    where: Condition.eq("parentId", created.id),
                    order: [{ column: "name", direction: "asc" }],
                })
            ).items.map((item) => [item.name, item.object, item.commit, item.deletedAt === null]);
        expect(await references()).toEqual([
            ["refs/heads/main", objects.first, objects.first, true],
            ["refs/heads/next", objects.second, objects.second, true],
            ["refs/tags/v0", objects.first, objects.first, true],
            ["refs/tags/v1", objects.release, objects.first, true],
        ]);

        // receive a branch push, a tag creation, a branch deletion, a ping and another installation's push
        const hosted = region.github.repositories.get("acme/site")!;
        hosted.references.set("refs/heads/main", { type: "commit", sha: objects.third });
        hosted.references.set("refs/tags/v2", { type: "tag", sha: objects.patch });
        hosted.tags.set(objects.patch, { type: "commit", sha: objects.third });
        hosted.references.delete("refs/heads/next");
        region.github.requests.length = 0;
        const deliveries = [
            await deliver("5a6f3c10-9b8e-11f0-8c1e-6a7c2d3b1e01", "push", {
                ref: "refs/heads/main",
                before: objects.first,
                after: objects.third,
                created: false,
                deleted: false,
                forced: false,
                base_ref: null,
                compare: `https://github.com/acme/site/compare/${objects.first.slice(0, 12)}...${objects.third.slice(0, 12)}`,
                commits: [{ id: objects.third, message: "write README.md", distinct: true }],
                head_commit: { id: objects.third, message: "write README.md", distinct: true },
                pusher: { name: "ada", email: "ada@example.test" },
                ...source(INSTALLATION),
            }),
            await deliver("5a6f3c10-9b8e-11f0-8c1e-6a7c2d3b1e02", "create", {
                ref: "v2",
                ref_type: "tag",
                master_branch: "main",
                description: null,
                pusher_type: "user",
                ...source(INSTALLATION),
            }),
            await deliver("5a6f3c10-9b8e-11f0-8c1e-6a7c2d3b1e03", "delete", {
                ref: "next",
                ref_type: "branch",
                pusher_type: "user",
                ...source(INSTALLATION),
            }),
            await deliver("5a6f3c10-9b8e-11f0-8c1e-6a7c2d3b1e04", "ping", {
                zen: "Keep it logically awesome.",
                hook_id: 30,
                hook: { type: "App", id: 30, events: ["create", "delete", "push"] },
                ...source(INSTALLATION),
            }),
            await deliver("5a6f3c10-9b8e-11f0-8c1e-6a7c2d3b1e05", "push", {
                ref: "refs/heads/main",
                before: objects.third,
                after: objects.second,
                created: false,
                deleted: false,
                forced: true,
                base_ref: null,
                commits: [],
                head_commit: null,
                ...source(OTHER_INSTALLATION),
            }),
        ];
        for (const request of deliveries) {
            await region.server.receive(request, WEBHOOK_SECRET);
        }

        // move main, add v2 at its peeled commit, delete next, and leave the other installation's push alone
        expect(await references()).toEqual([
            ["refs/heads/main", objects.third, objects.third, true],
            ["refs/heads/next", objects.second, objects.second, false],
            ["refs/tags/v0", objects.first, objects.first, true],
            ["refs/tags/v1", objects.release, objects.first, true],
            ["refs/tags/v2", objects.patch, objects.third, true],
        ]);

        // read the advertisement with a contents token once per delivery of the installation, audited as the system
        const refresh = [
            "POST api.github.test/app/installations/4242/access_tokens app",
            '  body {"repository_ids":[1296269],"permissions":{"contents":"read"}}',
            "GET github.com/acme/site.git/info/refs?service=git-upload-pack installation 4242",
        ];
        expect(region.github.requests).toEqual([...refresh, ...refresh, ...refresh]);

        // audit each refresh as the system, and the first one's reference changes before it
        const success = { kind: "success" };
        const refreshed = ["repository.refresh", { type: "repository", id: created.id }, success];
        expect(
            (await region.journal.read())
                .filter(
                    (call) => AuditCaller.actor(call.execution!.context.caller).type === "system",
                )
                .map((call) => [
                    call.method,
                    call.execution!.targets.repository ?? call.execution!.targets.reference!.type,
                    call.execution!.outcome,
                ]),
        ).toEqual([
            ["reference.create", "reference", success],
            ["reference.update", "reference", success],
            ["reference.update", "reference", success],
            refreshed,
            refreshed,
            refreshed,
        ]);

        // refuse a delivery signed with another secret before it reaches the workload
        const forged = await deliver(
            "5a6f3c10-9b8e-11f0-8c1e-6a7c2d3b1e06",
            "delete",
            {
                ref: "main",
                ref_type: "branch",
                pusher_type: "user",
                ...source(INSTALLATION),
            },
            "another secret",
        );
        expect(
            await region.server
                .receive(forged, WEBHOOK_SECRET)
                .catch((error: { code: string; message: string }) => [error.code, error.message]),
        ).toEqual(["UNAUTHORIZED", "webhook signature does not match"]);

        // reach the repository as an anonymous Git remote without GitHub's identity or the installation
        const anonymous = await owner.repository.update({
            accountId,
            id: created.id,
            requestId: RequestId.create(),
            hosting: "git",
            authentication: "anonymous",
            connectedAccountId: null,
        });
        expect([
            anonymous.hosting,
            anonymous.providerRepositoryId,
            anonymous.authentication,
            anonymous.connectedAccountId,
        ]).toEqual(["git", null, "anonymous", null]);
    },
);

test.each(TEST_DIALECTS)(
    "issue checkout access to a GitHub repository as installation tokens limited to it and to pulling or pushing on %s",
    async (dialect) => {
        const region = await RepositoryFixture.open(dialect);
        site(region.github);
        const owner = region.connect(ids.owner);
        const accountId = ids.account;
        const created = await owner.repository.create({
            accountId,
            requestId: RequestId.create(),
            name: "site",
            hosting: "github",
            remote: "https://github.com/acme/site.git",
            connectedAccountId: ids.connection,
        });

        // lease each token as Git's basic credential for the recorded remote
        region.github.requests.length = 0;
        const lease = (mode: "read" | "write", password: string) => ({
            url: "https://github.com/acme/site.git",
            mode,
            headers: {
                authorization: `Basic ${new TextEncoder().encode(`x-access-token:${password}`).toBase64()}`,
            },
            expiresAt: Date.parse("2026-09-27T13:00:00Z"),
        });
        expect([
            await owner.repository.open({ accountId, id: created.id, mode: "read" }),
            await owner.repository.open({ accountId, id: created.id, mode: "write" }),
        ]).toEqual([lease("read", "ghs_4242_2"), lease("write", "ghs_4242_3")]);

        // mint each token for GitHub's repository identifier alone, with the contents permission of its mode
        expect(region.github.requests).toEqual([
            "POST api.github.test/app/installations/4242/access_tokens app",
            '  body {"repository_ids":[1296269],"permissions":{"contents":"read"}}',
            "POST api.github.test/app/installations/4242/access_tokens app",
            '  body {"repository_ids":[1296269],"permissions":{"contents":"write"}}',
        ]);

        // audit each lease as the owner's call and its success, without the credential
        const actor = { type: "subject", subject: principal.user.reference("universe", ids.owner) };
        const target = { repository: { type: "repository", id: created.id } };
        expect(
            (await region.journal.read())
                .filter((call) => call.method === "repository.open")
                .map((call) => [
                    AuditCaller.actor(call.execution!.context.caller),
                    call.execution!.targets,
                    call.execution!.details,
                    call.execution!.outcome,
                ]),
        ).toEqual([
            [actor, target, {}, { kind: "success" }],
            [actor, target, {}, { kind: "success" }],
        ]);
    },
);
