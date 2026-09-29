import type { Select } from "@destack/db";
import { relation, union } from "@destack/access";
import { defineObject, field, method } from "@destack/object";
import { identifier, schema } from "@destack/schema";
import { RESIDENCIES } from "./region.ts";
import { user } from "./user.ts";
import { sudo } from "./sudo.ts";

/** An organisation owning accounts. */
export const organisation = defineObject({
    name: "organisation",
    tier: "global",
    plural: "organisations",
    scope: "universe",
    isScope: true,
    fields: {
        /** The display name. */
        name: field.string(schema.string().min(1).max(200)),
        /** The logo image URL. */
        image: field.string(schema.string().url()).optional(),
        /** The jurisdiction the organisation's home space keeps its data in, chosen at creation. */
        residency: field.enum(RESIDENCIES),
        /** The organisation's home space. */
        home: field.string(identifier("space")).optional(),
    },
    recoverable: { within: { days: 30 }, by: "delete" },
    relations: {
        owner: { subjects: [user], grantedBy: "own" },
        member: { subjects: [user] },
    },
    permissions: {
        read: union(relation("owner"), relation("member")),
        update: relation("owner"),
        delete: relation("owner"),
        share: relation("owner"),
        own: relation("owner"),
    },
    shareable: { by: "share" },
    reserved: ["own"],
    elevated: { delete: sudo, own: sudo },
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("update", {
            fields: ["name", "image", "residency"],
            creator: "owner",
        }),
        update: method.update("update", { fields: ["name", "image"] }),
    },
});
/** A persisted organisation. */
export type Organisation = Select<typeof organisation.table>;
