import type { ObjectType } from "@destack/object";
import type { ObjectAccess } from "@destack/object/client";
import { useClient, useView } from "../page/view.ts";
import { ViewContext } from "../declare/context.ts";

/** Read and change object types of the view's space by key, as its space permissions grant. */
export function useSpace<const Objects extends Readonly<Record<string, ObjectType>>>(
    objects: Objects,
): ObjectAccess<Objects> {
    return useClient(ViewContext.scope(useView(), "space")).of(objects);
}

/** Read and change object types of the view's account by key, as its account permissions grant. */
export function useAccount<const Objects extends Readonly<Record<string, ObjectType>>>(
    objects: Objects,
): ObjectAccess<Objects> {
    return useClient(ViewContext.scope(useView(), "account")).of(objects);
}

/** Read and change object types of the person's home by key. */
export function useHome<const Objects extends Readonly<Record<string, ObjectType>>>(
    objects: Objects,
): ObjectAccess<Objects> {
    return useClient(ViewContext.scope(useView(), "home")).of(objects);
}
