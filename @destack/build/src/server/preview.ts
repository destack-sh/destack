import { implement, type ServiceContext } from "@destack/service/server";
import { preview } from "../service/preview.ts";
import { PreviewPool } from "../preview/index.ts";

/** Implement preview procedures against authenticated lifecycle state. */
export function implementPreview(store: PreviewPool) {
    const service = implement({ preview }).$context<ServiceContext>();

    return service.preview.router({
        start: service.preview.start.handler(({ input, context }) =>
            store.start(context.requireCaller().id, input),
        ),
        get: service.preview.get.handler(({ input, context }) =>
            store.get(context.requireCaller().id, input.id),
        ),
        list: service.preview.list.handler(({ context }) => store.list(context.requireCaller().id)),
        watch: service.preview.watch.handler(({ input, context, signal }) =>
            store.watch(context.requireCaller().id, input.id, signal),
        ),
        stop: service.preview.stop.handler(({ input, context }) =>
            store.stop(context.requireCaller().id, input.id),
        ),
    });
}
