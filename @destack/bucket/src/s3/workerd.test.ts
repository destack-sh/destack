import { modulePlugin } from "@destack/package/vite";
import { testHost } from "@destack/test/host";
import { rolldown } from "rolldown";
import { WORKERD_COMPATIBILITY_DATE } from "@destack/package/build";
import { Miniflare } from "miniflare";
import { fileURLToPath } from "node:url";
import { expect, onTestFinished, test } from "@destack/test";

test("sign, presign and verify the documented S3 examples in workerd without Node.js compatibility", async () => {
    // bundle the worker as Wrangler resolves workerd modules
    const bundle = await rolldown({
        input: fileURLToPath(new URL("./test/worker.ts", import.meta.url)),
        platform: "browser",
        resolve: { conditionNames: ["workerd", "worker", "browser"] },
        plugins: [modulePlugin(testHost().files)],
    });
    const { output } = await bundle.generate({ format: "esm" });
    await bundle.close();
    const [chunk] = output;

    // run it without nodejs_compat
    const worker = new Miniflare({
        modules: true,
        script: chunk.code,
        compatibilityDate: WORKERD_COMPATIBILITY_DATE,
    });
    onTestFinished(() => worker.dispose());
    const response = await worker.dispatchFetch("https://sign.test/");

    // answer the documentation's signature and presigned URL, both verified
    expect([response.status, await response.json()]).toEqual([
        200,
        {
            authorization:
                "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request, SignedHeaders=host;range;x-amz-content-sha256;x-amz-date, Signature=f0e8bdb87c964420e857bd35b5d6ed310bd44f0170aba48dd91039c6036bdb41",
            url: "https://examplebucket.s3.amazonaws.com/test.txt?X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=AKIAIOSFODNN7EXAMPLE%2F20130524%2Fus-east-1%2Fs3%2Faws4_request&X-Amz-Date=20130524T000000Z&X-Amz-Expires=86400&X-Amz-SignedHeaders=host&X-Amz-Signature=aeeed9bbccd4d02ee5c0109b86d86835f995330da4c265957d157751f604d404",
            verified: ["AKIAIOSFODNN7EXAMPLE", "AKIAIOSFODNN7EXAMPLE"],
        },
    ]);
});
