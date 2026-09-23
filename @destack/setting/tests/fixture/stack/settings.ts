import { defineSettingAssignment } from "../../../src/declare/index.ts";
import { editor } from "../settings/index.ts";

/** A source-managed personal choice importing the app's declaration. */
export const editorAssignment = defineSettingAssignment(
    editor,
    {
        kind: "user",
        user: {
            kind: "user",
            authority: "global",
            id: "user-019f5530-8000-7000-8000-000000000003",
        },
    },
    "vim",
);
