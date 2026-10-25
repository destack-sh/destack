import type { Select } from "@destack/db";
import { none, permission, principal, relation, union } from "@destack/access";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { StackSource } from "./stack-source.ts";
import { AccountHandle } from "../address/handle.ts";
import { residency, ResidencyCode } from "./residency.ts";
import { sudo } from "../authentication/sudo.ts";
import { DisplayName } from "./display-name.ts";

/** A membership in an organization: a relation to its users, granted by its managers. */
const MEMBERSHIP = { subjects: [principal.user], grantedBy: "manage" };

/** The relations of an organization's members. */
export const MEMBERSHIPS = ["owner", "admin", "billing", "member", "guest"] as const;
/** A relation of an organization's members. */
export type Membership = (typeof MEMBERSHIPS)[number];

/** An organization with accounts, and the people who are its members. */
export const organization = defineObject(
    {
        name: "organization",
        plural: "organizations",
        scope: "universe",
        isScope: true,
        fields: {
            /** The display name. */
            name: field.string(DisplayName),
            /** The logo image URL. */
            image: field.string(schema.url()).optional(),
            /** What the organization is: a business, a family, a community or a school. */
            kind: field.enum(["business", "family", "community", "education"]).default("business"),
            /** The account the charges of the organization's accounts consolidate to. */
            payer: field.string(schema.identifier("account")).guardWrite("bill"),
        },
        recoverable: { within: { days: 30 }, by: "delete" },
        relations: {
            // add members by relation, at least one owner, and the machines publishing as the platform
            owner: { ...MEMBERSHIP, grantedBy: "administer", isRequired: true },
            admin: {
                subjects: [principal.user, residency.members("machine")],
                grantedBy: "manage",
            },
            billing: MEMBERSHIP,
            member: MEMBERSHIP,
            guest: MEMBERSHIP,
            // relate the servers of each residency of its accounts
            resident: { subjects: [residency.members("server")], grantedBy: "place" },
        },
        permissions: {
            read: union(
                relation("owner"),
                relation("admin"),
                relation("billing"),
                relation("member"),
                relation("guest"),
            ),
            view: union(relation("owner"), relation("admin"), relation("member")),
            manage: union(relation("owner"), relation("admin")),
            bill: union(relation("owner"), relation("billing")),
            administer: relation("owner"),
            list: union(permission("read"), relation("resident")),
            replicate: relation("resident"),
            update: permission("manage"),
            delete: permission("administer"),
            place: none(),
            // leave the organization's resources undeclared, as no space is its home
            provision: none(),
        },
        reserved: ["administer", "replicate", "place", "provision"],
        elevated: { delete: sudo, administer: sudo },
    },
    (method) => ({
        list: method.list("list"),
        create: method.create("update", {
            fields: ["name", "image", "kind"],
            input: schema.object({
                /** The handle the organization's account claims. */
                handle: AccountHandle,
                /** The jurisdiction the organization's account stores data in. */
                residencyId: ResidencyCode,
                /** The stack the home of the organization's account runs, none when absent. */
                stack: StackSource.exactOptional(),
            }),
            isPredicted: false,
        }),
        update: method.update("update", { fields: ["name", "image", "kind", "payer"] }),
    }),
);
/** A persisted organization. */
export type Organization = Select<typeof organization.table>;
