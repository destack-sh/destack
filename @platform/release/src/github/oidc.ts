import type { Environment } from "@destack/environment";
import { schema } from "@destack/schema";

/** The OIDC token GitHub Actions mints for a job, by the runner's request variables. */
export const ActionsToken = {
    /** Ask the runner for the job's OIDC token for an audience, in a job with id-token: write. */
    async request(audience: string, environment: Environment<"process">): Promise<string> {
        // read the runner's request URL and token, which only jobs with id-token: write have
        const { variables } = environment.process;
        const url = variables["ACTIONS_ID_TOKEN_REQUEST_URL"];
        const token = variables["ACTIONS_ID_TOKEN_REQUEST_TOKEN"];
        if (url === undefined || token === undefined) {
            throw new Error(
                "no OIDC token request: run in a GitHub Actions job with id-token: write",
            );
        }

        // ask for the token for the audience
        const requested = new URL(url);
        requested.searchParams.set("audience", audience);
        const response = await environment.network.fetch(
            new Request(requested, { headers: { authorization: `Bearer ${token}` } }),
        );
        if (!response.ok) {
            await response.body?.cancel();
            throw new Error(`the runner refused the job's OIDC token: ${response.status}`);
        }

        // read the token from the runner's JSON reply
        const { value } = schema
            .looseObject({ value: schema.string().min(1) })
            .parse(await response.json());

        return value;
    },
};
