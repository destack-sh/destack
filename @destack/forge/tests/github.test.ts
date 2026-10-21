import { Secret } from "@destack/vault";
import { AuditCaller } from "@destack/object";
import { nth, found } from "@destack/schema";
import { expect, test } from "@destack/test";
import { ConnectionError } from "@destack/connection";
import { principal } from "@destack/access";
import { RequestId } from "@destack/sync";
import { GitHubSignature, GitHubApp } from "../src/github/index.ts";
import forgePackage from "../src/package.ts";
import { APP_ID, GITHUB_API, GitHubStandIn, WEBHOOK_SECRET } from "./fixture/github.ts";
import { auditedCalls } from "./fixture/audit.ts";
import { githubPrivateKey } from "./fixture/key.ts";
import { ids, INSTALLATION, OTHER_INSTALLATION, repositoryTest } from "./fixture/repository.ts";
import { Clock } from "@destack/environment";

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

/** Host acme/site at GitHub: main and next, an annotated v1 and a lightweight v0. */
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

/** The repository and installation objects GitHub's repository events send. */
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

/** The URL GitHub delivers the forge's webhook events to. */
const WEBHOOK_URL = `https://forge.test${forgePackage.adapters.github.path}`;

/** Sign a delivery as GitHub sends it to the webhook's route. */
async function deliver(id: string, event: string, payload: unknown, secret = WEBHOOK_SECRET) {
    const body = JSON.stringify(payload);
    const headers = await new GitHubSignature().sign(
        { id, event, body, sentAt: Date.now() },
        secret,
    );
    headers.set("content-type", "application/json");

    return new Request(WEBHOOK_URL, {
        method: "POST",
        headers,
        body,
    });
}

repositoryTest(
    "identify a repository and issue installation tokens limited to it and to the permissions each use needs",
    async () => {
        const github = new GitHubStandIn();
        site(github);
        const app = new GitHubApp({
            clock: Clock.system,
            id: APP_ID,
            key: Secret.of(githubPrivateKey()),
            secret: Secret.of(WEBHOOK_SECRET),
            api: GITHUB_API,
            fetch: github.fetch,
        });

        // identify the repository by name and lease writing to it by identifier
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
                    authorization: `Basic ${new TextEncoder().encode("x-access-token:ghs_4242_contents_write").toBase64()}`,
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
    },
);

test("read an installation from GitHub's setup redirect, refuse a redirect without one, and uninstall the app as itself", async () => {
    // point the app at the stand-in
    const github = new GitHubStandIn();
    const app = new GitHubApp({
        clock: Clock.system,
        id: APP_ID,
        key: Secret.of(githubPrivateKey()),
        secret: Secret.of(WEBHOOK_SECRET),
        api: GITHUB_API,
        fetch: github.fetch,
    });

    // read the installation, then uninstall it with the app's JWT
    const installation = app.installation({ installation_id: "4242", setup_action: "install" });
    await app.uninstall(installation);
    expect({ installation, requests: github.requests }).toEqual({
        installation: "4242",
        requests: ["DELETE api.github.test/app/installations/4242 app"],
    });

    // refuse a redirect naming no installation
    expect(() => app.installation({ setup_action: "install" })).toThrow(
        new ConnectionError(
            "INVALID_AUTHORIZATION",
            "the installation redirect has no installation",
        ),
    );
});

repositoryTest(
    "connect a GitHub repository, refresh it, and refresh it again as the system on push, create and delete deliveries",
    async ({ forge }) => {
        site(forge.github);
        const owner = forge.connect(ids.owner);
        const accountId = ids.account;

        // connect acme/site, and record GitHub's identifier once the connection's replica arrives
        const created = await owner.repository.create({
            spaceId: forge.space,
            accountId,
            requestId: RequestId.create(Date.now()),
            name: "site",
            hosting: "github",
            remote: "https://github.com/acme/site.git",
            connectionId: nth(forge.connections, 0),
        });
        const identified = await forge.identified(created.id);
        expect([
            created.provider,
            created.providerRepositoryId,
            identified.providerRepositoryId,
        ]).toEqual([null, null, "1296269"]);
        await owner.repository.refresh({
            spaceId: forge.space,
            id: created.id,
            requestId: RequestId.create(Date.now()),
        });
        const references = async () =>
            (
                await owner.reference.list({
                    spaceId: forge.space,
                    where: { parentId: created.id },
                    orderBy: { name: "asc" },
                })
            ).items.map((item) => [item.name, item.object, item.commit, item.deletedAt === null]);
        expect(await references()).toEqual([
            ["refs/heads/main", objects.first, objects.first, true],
            ["refs/heads/next", objects.second, objects.second, true],
            ["refs/tags/v0", objects.first, objects.first, true],
            ["refs/tags/v1", objects.release, objects.first, true],
        ]);

        // receive a branch push, a tag creation, a branch deletion, a ping and another installation's push, once the connection's replica arrived at the forge
        await forge.settle();
        const hosted = found(forge.github.repositories, "acme/site");
        hosted.references.set("refs/heads/main", { type: "commit", sha: objects.third });
        hosted.references.set("refs/tags/v2", { type: "tag", sha: objects.patch });
        hosted.tags.set(objects.patch, { type: "commit", sha: objects.third });
        hosted.references.delete("refs/heads/next");
        forge.github.requests.length = 0;
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
        const statuses: number[] = [];
        for (const request of deliveries) {
            statuses.push((await forge.http.fetch(request)).status);
        }

        // move main, add v2 at its peeled commit, delete next, and leave the other installation's push alone
        expect({ statuses, references: await references() }).toEqual({
            statuses: [204, 204, 204, 204, 204],
            references: [
                ["refs/heads/main", objects.third, objects.third, true],
                ["refs/heads/next", objects.second, objects.second, false],
                ["refs/tags/v0", objects.first, objects.first, true],
                ["refs/tags/v1", objects.release, objects.first, true],
                ["refs/tags/v2", objects.patch, objects.third, true],
            ],
        });

        // read the advertisement with a contents token once per delivery of the installation, audited as the system
        const refresh = [
            "POST api.github.test/app/installations/4242/access_tokens app",
            '  body {"repository_ids":[1296269],"permissions":{"contents":"read"}}',
            "GET github.com/acme/site.git/info/refs?service=git-upload-pack installation 4242",
        ];
        expect(forge.github.requests).toEqual([...refresh, ...refresh, ...refresh]);

        // audit the identification and each refresh as the forge's own components, and the first refresh's reference changes before it
        const success = "success";
        const target = { type: "repository", id: created.id };
        const refreshed = ["repository.refresh", target, success];
        expect(
            (await auditedCalls(forge.database))
                .filter((call) => call.execution.context.component !== undefined)
                .map(({ method, execution }) => [
                    method,
                    execution.target.type === "reference" ? "reference" : execution.target,
                    execution.outcome.kind,
                ]),
        ).toEqual([
            ["repository.identify", target, success],
            ["reference.createMany", { type: "scope", id: forge.space }, success],
            ["reference.updateMany", { type: "scope", id: forge.space }, success],
            refreshed,
            refreshed,
            refreshed,
        ]);

        // refuse a delivery signed with another secret before the forge sees it, and a read of the webhook
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
        const refused = await forge.http.fetch(forged);
        const read = await forge.http.fetch(new Request(WEBHOOK_URL));
        expect([
            [refused.status, await refused.json()],
            [read.status, await read.text()],
        ]).toEqual([
            [
                401,
                {
                    code: "UNAUTHORIZED",
                    status: 401,
                    message: "webhook signature is invalid",
                    defined: true,
                },
            ],
            [405, ""],
        ]);

        // open the repository as an anonymous Git remote without GitHub's identity or the installation
        const anonymous = await owner.repository.update({
            spaceId: forge.space,
            id: created.id,
            requestId: RequestId.create(Date.now()),
            hosting: "git",
            authentication: "anonymous",
            connectionId: null,
        });
        expect([
            anonymous.hosting,
            anonymous.providerRepositoryId,
            anonymous.authentication,
            anonymous.connectionId,
        ]).toEqual(["git", null, "anonymous", null]);
    },
);

repositoryTest(
    "issue checkout access to a GitHub repository as installation tokens limited to it and to pulling or pushing",
    async ({ forge }) => {
        site(forge.github);
        const owner = forge.connect(ids.owner);
        const accountId = ids.account;
        const created = await owner.repository.create({
            spaceId: forge.space,
            accountId,
            requestId: RequestId.create(Date.now()),
            name: "app",
            hosting: "github",
            remote: "https://github.com/acme/site.git",
            connectionId: nth(forge.connections, 0),
        });
        await forge.identified(created.id);

        // lease each token as Git's basic credential for the recorded remote
        forge.github.requests.length = 0;
        expect([
            await owner.repository.open({ spaceId: forge.space, id: created.id, mode: "read" }),
            await owner.repository.open({ spaceId: forge.space, id: created.id, mode: "write" }),
        ]).toEqual([
            lease("read", "ghs_4242_contents_read"),
            lease("write", "ghs_4242_contents_write"),
        ]);

        // issue each token for GitHub's repository identifier alone, with the contents permission of its mode
        expect(forge.github.requests).toEqual([
            "POST api.github.test/app/installations/4242/access_tokens app",
            '  body {"repository_ids":[1296269],"permissions":{"contents":"read"}}',
            "POST api.github.test/app/installations/4242/access_tokens app",
            '  body {"repository_ids":[1296269],"permissions":{"contents":"write"}}',
        ]);

        // audit each lease as the owner's call and its success, without the credential
        const actor = { subject: principal.user.reference("universe", ids.owner) };
        const target = { type: "repository", id: created.id };
        expect(
            (await auditedCalls(forge.database))
                .filter(
                    (call) =>
                        call.method === "repository.open" &&
                        call.execution.target.id === created.id,
                )
                .map((call) => [
                    AuditCaller.actor(call.execution.context.caller),
                    call.execution.target,
                    call.execution.details,
                    call.execution.outcome,
                ]),
        ).toEqual([
            [actor, target, {}, { kind: "success" }],
            [actor, target, {}, { kind: "success" }],
        ]);
    },
);

/** Build the lease of a GitHub installation token for the recorded remote. */
function lease(mode: "read" | "write", password: string) {
    return {
        url: "https://github.com/acme/site.git",
        mode,
        headers: {
            authorization: `Basic ${new TextEncoder().encode(`x-access-token:${password}`).toBase64()}`,
        },
        expiresAt: Date.parse("2026-09-27T13:00:00Z"),
    };
}
