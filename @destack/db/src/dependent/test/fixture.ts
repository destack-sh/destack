import { defineTable, text } from "../../index.ts";

/** Remarks naming their subject's type and identifier. */
export const remark = defineTable("dependent_remark", {
    /** The remark's identifier. */
    id: text("id").primaryKey(),
    /** The type of the remarked row. */
    subjectType: text("subject_type").notNull(),
    /** The identifier of the remarked row. */
    subjectId: text("subject_id").notNull(),
});

/** Posts deleting their remarks. */
export const post = defineTable(
    "dependent_post",
    {
        /** The post's identifier. */
        id: text("id").primaryKey(),
    },
    {
        dependents: [
            {
                from: () => remark,
                key: "subjectId",
                where: { subjectType: "post" },
                onDelete: "cascade",
            },
        ],
    },
);

/** Sections deleting with their post. */
export const section = defineTable(
    "dependent_section",
    {
        /** The section's identifier. */
        id: text("id").primaryKey(),
        /** The post holding the section. */
        postId: text("post_id")
            .notNull()
            .references(() => post.id, { onDelete: "cascade" }),
    },
    {
        dependents: [
            {
                from: () => remark,
                key: "subjectId",
                where: { subjectType: "section" },
                onDelete: "cascade",
            },
        ],
    },
);

/** Photos that cannot be deleted while remarked. */
export const photo = defineTable(
    "dependent_photo",
    {
        /** The photo's identifier. */
        id: text("id").primaryKey(),
    },
    {
        dependents: [
            {
                from: () => remark,
                key: "subjectId",
                where: { subjectType: "photo" },
                onDelete: "restrict",
            },
        ],
    },
);
