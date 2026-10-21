import { type HttpsBinding, type HttpsDescription, HttpsKind } from "../declare/https.ts";

/** Describe an HTTPS API dependency for the package manifest. */
export function describeHttps(binding: HttpsBinding): HttpsDescription {
    return HttpsKind.description.parse({
        name: binding.name,
        kind: binding.kind,
        spec: binding.spec,
    });
}
