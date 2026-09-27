import { build, stop } from "esbuild";
import { fileURLToPath } from "node:url";

/** The account explicitly selected for temporary hosted resources. */
const account = process.env["CLOUDFLARE_ACCOUNT_ID"];
/** The token used only for the Cloudflare management API. */
const token = process.env["CLOUDFLARE_COMPANY_API_TOKEN"];
if (!account || !token) {
    throw new Error("set CLOUDFLARE_ACCOUNT_ID and CLOUDFLARE_COMPANY_API_TOKEN");
}

/** The unique name shared by this run's Worker and bucket. */
const name = `ds-storage-${crypto.randomUUID()}`;
/** Authenticate requests to the temporary Worker. */
const secret = crypto.randomUUID();

/** Send one bounded management request without printing credentials. */
async function request<Value = unknown>(
    path: string,
    method = "GET",
    body?: RequestInit["body"],
): Promise<Value> {
    const response = await fetch(
        `https://api.cloudflare.com/client/v4/accounts/${account}/${path}`,
        {
            method,
            headers: {
                Authorization: `Bearer ${token}`,
                ...(typeof body === "string" ? { "Content-Type": "application/json" } : {}),
            },
            ...(method === "GET" ? {} : { body }),
            signal: AbortSignal.timeout(30000),
        },
    );
    const result = (await response.json()) as {
        success: boolean;
        errors: unknown[];
        result: Value;
    };
    if (!response.ok || !result.success) {
        throw new Error(`${method} ${path}: ${response.status} ${JSON.stringify(result.errors)}`);
    }

    return result.result;
}

/** The adapter and hosted checks, built without the local database dependency. */
const bundle = await build({
    entryPoints: [fileURLToPath(new URL("./worker.ts", import.meta.url))],
    bundle: true,
    write: false,
    format: "esm",
    platform: "neutral",
    target: "esnext",
    external: ["node:assert/strict"],
});
await stop();

/** Whether the test bucket exists and needs deletion. */
let isBucketCreated = false;
/** Whether the test Worker exists and needs deletion. */
let isWorkerCreated = false;
/** The failures collected across the run and cleanup. */
const failures: unknown[] = [];
try {
    // create isolated resources without modifying existing deployments
    const { subdomain } = await request<{ subdomain: string }>("workers/subdomain");
    await request("r2/buckets", "POST", JSON.stringify({ name, locationHint: "weur" }));
    isBucketCreated = true;
    console.log(`Created temporary bucket ${name}`);
    const form = new FormData();
    form.set(
        "metadata",
        JSON.stringify({
            main_module: "worker.js",
            compatibility_date: "2026-09-19",
            compatibility_flags: ["nodejs_compat"],
            bindings: [
                { type: "r2_bucket", name: "BUCKET", bucket_name: name },
                { type: "secret_text", name: "TOKEN", text: secret },
            ],
        }),
    );
    form.set(
        "worker.js",
        new Blob([bundle.outputFiles[0].text], { type: "application/javascript+module" }),
        "worker.js",
    );
    await request(`workers/scripts/${name}`, "PUT", form);
    isWorkerCreated = true;
    await request(`workers/scripts/${name}/subdomain`, "POST", JSON.stringify({ enabled: true }));

    // wait for routing propagation and verify the authorization guard before running the suite
    const url = `https://${name}.${subdomain}.workers.dev`;
    const deadline = Date.now() + 60000;
    while (true) {
        const ready = await fetch(url, { signal: AbortSignal.timeout(10000) });
        await ready.body?.cancel();
        if (ready.status === 401) {
            break;
        }
        if (ready.status !== 404 || Date.now() >= deadline) {
            throw new Error(`temporary Worker did not become ready: ${ready.status}`);
        }
        await new Promise((resolve) => setTimeout(resolve, 1000));
    }

    // invoke the authenticated suite with a strict execution timeout
    const response = await fetch(url, {
        method: "POST",
        headers: { Authorization: `Bearer ${secret}` },
        signal: AbortSignal.timeout(30000),
    });
    const result = await response.text();
    if (!response.ok || result !== '{"status":"passed"}') {
        throw new Error(`hosted storage check: ${response.status} ${result.slice(0, 2000)}`);
    }
    console.log("Hosted R2 check passed.");
} catch (error) {
    failures.push(error);
} finally {
    // attempt both deletions even when the test or either cleanup fails
    if (isWorkerCreated) {
        try {
            await request(`workers/scripts/${name}`, "DELETE");
            console.log(`Removed temporary Worker ${name}`);
        } catch (error) {
            failures.push(error);
        }
    }
    if (isBucketCreated) {
        try {
            // remove contents through the management API even if the Worker terminated
            while (true) {
                const files = await request<{ key: string }[]>(`r2/buckets/${name}/objects`);
                if (!files.length) {
                    break;
                }
                for (const file of files) {
                    await request(
                        `r2/buckets/${name}/objects/${encodeURIComponent(file.key)}`,
                        "DELETE",
                    );
                }
            }
            await request(`r2/buckets/${name}`, "DELETE");
            console.log(`Removed temporary bucket ${name}`);
        } catch (error) {
            failures.push(error);
        }
    }
}
if (failures.length) {
    throw new AggregateError(failures, "hosted storage verification failed");
}
