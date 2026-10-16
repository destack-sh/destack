import { nth, found, schema } from "@destack/schema";
import { githubPublicKey } from "./key.ts";
import { verifyToken } from "./token.ts";

/** The fixture GitHub App's client ID. */
export const APP_ID = "Iv23liFixtureApp";

/** The REST API base the stand-in serves. */
export const GITHUB_API = new URL("https://api.github.test");

/** The secret the fixture GitHub App signs webhook deliveries with. */
export const WEBHOOK_SECRET = "It's a Secret to Everybody";

/** The longest lifetime GitHub accepts for an app's JWT: ten minutes. */
const APP_TOKEN_SECONDS = 600;

/** The host serving the stand-in's Git remotes. */
const GITHUB_HOST = "github.com";

/** The lifetime claims of an app JWT, in epoch seconds. */
const JwtLifetime = schema.looseObject({ iat: schema.number(), exp: schema.number() });

/** The permissions an installation token request grants. */
const Grant = schema.looseObject({ permissions: schema.record(schema.string(), schema.string()) });

/** A Git object as GitHub's REST API describes it. */
interface GitObject {
    /** The object type: commit or tag. */
    readonly type: "commit" | "tag";
    /** The object name. */
    readonly sha: string;
}

/** One repository the stand-in hosts. */
export interface StandInRepository {
    /** GitHub's numeric repository identifier. */
    readonly id: number;
    /** The owner and name. */
    readonly fullName: string;
    /** The installation the repository is installed in. */
    readonly installation: number;
    /** The default branch's short name. */
    readonly defaultBranch: string;
    /** The references by full name. */
    readonly references: Map<string, GitObject>;
    /** The annotated tag objects by name with their tagged objects. */
    readonly tags: Map<string, GitObject>;
}

/** A stand-in for the GitHub endpoints the platform calls, recording each request. */
export class GitHubStandIn {
    /** The repositories by owner and name. */
    readonly repositories = new Map<string, StandInRepository>();
    /** Each request as `METHOD path?query credential`, in order. */
    readonly requests: string[] = [];
    /** The installation each issued token belongs to. */
    readonly #tokens = new Map<string, number>();

    /** Take a repository. */
    add(repository: StandInRepository): void {
        this.repositories.set(repository.fullName, repository);
    }

    /** Serve one request as GitHub does. */
    readonly fetch = async (input: RequestInfo | URL, options?: RequestInit): Promise<Response> => {
        // record the request with the credential it presents, as a bearer or as Git's basic password
        const request = new Request(input, options);
        const url = new URL(request.url);
        const secret = presentedSecret(request.headers.get("authorization"));
        const installation = secret === undefined ? undefined : this.#tokens.get(secret);
        const credential = installation === undefined ? "app" : `installation ${installation}`;
        this.requests.push(
            `${request.method} ${url.host}${url.pathname}${url.search} ${credential}`,
        );

        // match a Git advertisement or an installation token request
        const advertised = /^\/([^/]+\/[^/]+)\.git\/info\/refs$/u.exec(url.pathname);
        const token = /^\/app\/installations\/(\d+)\/access_tokens$/u.exec(url.pathname);
        const installed = /^\/app\/installations\/(\d+)$/u.exec(url.pathname);

        // advertise a repository's references to its installation's tokens over Git's smart HTTP
        if (url.host === GITHUB_HOST && advertised !== null) {
            return this.#advertise(nth(advertised, 1), installation);
        }
        // refuse API requests without GitHub's API version and a user agent
        else if (
            request.headers.get("x-github-api-version") !== "2022-11-28" ||
            request.headers.get("user-agent") === null
        ) {
            return Response.json({ message: "missing version or user agent" }, { status: 400 });
        }
        // create an installation token for an app JWT
        else if (request.method === "POST" && token !== null) {
            return this.#issue(request, secret, Number(token[1]));
        }
        // uninstall the app for an app JWT
        else if (request.method === "DELETE" && installed !== null) {
            return this.#uninstall(secret);
        }
        // read a repository its installation opens
        else {
            return this.#read(url, installation);
        }
    };

    /** Advertise a repository's references to its installation's tokens. */
    #advertise(fullName: string, installation: number | undefined): Response {
        // hide repositories of other installations
        const hosted = this.repositories.get(fullName);
        if (hosted === undefined || hosted.installation !== installation) {
            return new Response("Repository not found.", { status: 404 });
        }

        return new Response(advertise(hosted), {
            headers: { "content-type": "application/x-git-upload-pack-advertisement" },
        });
    }

    /** Uninstall the app for a verified app JWT. */
    async #uninstall(secret: string | undefined): Promise<Response> {
        if (secret === undefined || !(await isAppToken(secret))) {
            return Response.json({ message: "bad jwt" }, { status: 401 });
        }

        return new Response(null, { status: 204 });
    }

    /** Create an installation token for a verified app JWT. */
    async #issue(
        request: Request,
        secret: string | undefined,
        installation: number,
    ): Promise<Response> {
        // verify the app's JWT and its lifetime
        if (secret === undefined) {
            throw new TypeError("an installation token request presents no app jwt");
        }
        if (!(await isAppToken(secret))) {
            return Response.json({ message: "bad jwt" }, { status: 401 });
        }

        // issue a token of the installation named by the permissions it grants, recording the request's body
        const body: unknown = await request.json();
        const granted = Object.entries(Grant.parse(body).permissions).map(
            ([name, level]) => `${name}_${level}`,
        );
        const issued = `ghs_${installation}_${granted.join("_")}`;
        this.#tokens.set(issued, installation);
        this.requests.push(`  body ${JSON.stringify(body)}`);

        return Response.json(
            {
                token: issued,
                expires_at: "2026-09-27T13:00:00Z",
                permissions: { contents: "read", metadata: "read" },
                repository_selection: "selected",
            },
            { status: 201 },
        );
    }

    /** Read the repository of a path for its installation's tokens. */
    #read(url: URL, installation: number | undefined): Response {
        // find the repository the installation opens
        const path = /^\/repos\/([^/]+\/[^/]+)(\/.*)?$/u.exec(url.pathname);
        const repository = path === null ? undefined : this.repositories.get(nth(path, 1));
        if (path === null || repository === undefined || repository.installation !== installation) {
            return Response.json({ message: "Not Found" }, { status: 404 });
        }

        // read the repository itself
        if (path[2] === undefined) {
            return Response.json({
                id: repository.id,
                node_id: `R_${repository.id}`,
                name: repository.fullName.split("/")[1],
                full_name: repository.fullName,
                private: true,
                default_branch: repository.defaultBranch,
            });
        }
        // hide the paths below it
        else {
            return Response.json({ message: "Not Found" }, { status: 404 });
        }
    }
}

/** Encode a repository's upload-pack advertisement like GitHub. */
function advertise(repository: StandInRepository): Uint8Array<ArrayBuffer> {
    // name HEAD's commit and branch and each reference, following tag objects to their commit
    const head = `refs/heads/${repository.defaultBranch}`;
    const lines = [
        `${found(repository.references, head).sha} HEAD\0symref=HEAD:${head} agent=git/github-7c1a`,
    ];
    for (const name of [...repository.references.keys()].toSorted()) {
        let object = found(repository.references, name);
        lines.push(`${object.sha} ${name}`);
        if (object.type === "tag") {
            while (object.type === "tag") {
                object = found(repository.tags, object.sha);
            }
            lines.push(`${object.sha} ${name}^{}`);
        }
    }

    // frame the service line, a flush, the references and a closing flush as pkt-lines
    return new TextEncoder().encode(
        `${packet("# service=git-upload-pack")}0000${lines.map(packet).join("")}0000`,
    );
}

/** Frame a line as a pkt-line with its newline. */
function packet(line: string): string {
    const length = new TextEncoder().encode(line).length + 5;

    return `${length.toString(16).padStart(4, "0")}${line}\n`;
}

/** Read the secret an authorization header presents, as a bearer or as Git's basic password. */
function presentedSecret(authorization: string | null): string | undefined {
    // present nothing without a header
    if (authorization === null) {
        return undefined;
    }

    // read the bearer token, or the password of a basic credential
    const [scheme, presented] = authorization.split(" ");
    if (presented === undefined) {
        throw new TypeError(`authorization ${scheme} presents no credential`);
    }
    const password =
        scheme === "Basic" ? new TextDecoder().decode(Uint8Array.fromBase64(presented)) : undefined;

    return password === undefined ? presented : nth(password.split(":"), 1);
}

/** Report whether a bearer is the app's JWT, issued by the app for at most ten minutes. */
async function isAppToken(secret: string): Promise<boolean> {
    const { claims } = await verifyToken(secret, await githubPublicKey());
    const lifetime = JwtLifetime.safeParse(claims);

    return (
        claims["iss"] === APP_ID &&
        lifetime.success &&
        lifetime.data.exp - lifetime.data.iat <= APP_TOKEN_SECONDS
    );
}
