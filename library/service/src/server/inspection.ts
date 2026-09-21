import { implement } from "../server/handler.ts";
import { inspection } from "../procedure/inspection.ts";
import { inspectService } from "../inspect/service.ts";
import type { ServiceDefinition } from "../declare/service.ts";
import type { DocumentOptions } from "../openapi/document.ts";

/** Generate reflection once and serve the same description used by build inspection. */
export async function implementInspection(definition: ServiceDefinition, options: DocumentOptions) {
    // generate the description once before accepting inspection requests
    const description = await inspectService(definition, options);
    const implementation = implement(inspection);

    return implementation.router({ get: implementation.get.handler(() => description) });
}
