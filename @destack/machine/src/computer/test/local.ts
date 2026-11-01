import { through } from "@destack/access";
import { defineObject, device, field } from "@destack/object";
import { Package } from "@destack/package";
import { schema } from "@destack/schema";

/** The package of the bookmarks this computer's native device stores. */
export const desk = Package.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000c5",
    name: "@acme/desk",
    version: "2026.10.0",
});

/** A link a person stores on this computer, as a desktop stores its local objects. */
export const bookmark = defineObject(
    {
        name: "bookmark",
        plural: "bookmarks",
        scope: device,
        storage: "local",
        fields: {
            /** The link. */
            link: field.string(schema.string().min(1)),
        },
        permissions: { read: through("device", "use"), declare: through("device", "use") },
    },
    (method) => ({
        declare: method.create("declare", { fields: ["link"] }),
    }),
    { package: desk },
);
