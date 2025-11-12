import { Elysia } from "elysia";

export type DemoEncodedProfile = {
    id: number;
    username: string;
    tags: string[];
    source: string;
};

export const parserFeature = new Elysia({ name: "parser" }).onParse(
    async ({ request, contentType }) => {
        if (contentType !== "application/elysia-demo") return;

        const raw = await request.text();
        const [idRaw, usernameRaw = "anonymous", tagsRaw = ""] = raw.split("|");
        const id = Number.parseInt(idRaw ?? "0", 10) || Date.now();
        const tags = tagsRaw
            .split(",")
            .map((tag) => tag.trim())
            .filter(Boolean);

        const payload: DemoEncodedProfile = {
            id,
            username: usernameRaw,
            tags,
            source: "elysia-demo"
        };

        return payload;
    }
);
