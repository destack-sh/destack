import { none, Policy } from "@destack/access";
import type {} from "@destack/package/import-meta";

/** A space's bucket, a resource holding files, whose roles grant its administration and data plane. */
export const bucket = new Policy(import.meta.destack.package, {
    name: "bucket",
    permissions: {
        read: none(),
        update: none(),
        delete: none(),

        list: none(),
        download: none(),
        upload: none(),
        remove: none(),
    },
    administration: ["read", "update", "delete"],
});
