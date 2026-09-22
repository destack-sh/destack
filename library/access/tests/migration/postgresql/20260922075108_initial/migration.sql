CREATE TABLE "example_item" (
	"id" text PRIMARY KEY,
	"scope" text NOT NULL,
	"owner" text NOT NULL,
	"parent" text,
	"row" bigint NOT NULL,
	"column" bigint NOT NULL,
	"locked" bigint NOT NULL,
	"team" bigint NOT NULL,
	"protected" bigint NOT NULL
);
--> statement-breakpoint
CREATE TABLE "example_item_parent_ancestor" (
	"scope" text NOT NULL,
	"ancestor" text NOT NULL,
	"descendant" text NOT NULL,
	"depth" bigint NOT NULL,
	CONSTRAINT "example_item_parent_ancestor_path" UNIQUE("scope","ancestor","descendant")
);
--> statement-breakpoint
CREATE TABLE "example_item_parent_ancestor_revision" (
	"scope" text PRIMARY KEY,
	"revision" bigint NOT NULL
);
--> statement-breakpoint
CREATE INDEX "example_item_parent_ancestor_descendant" ON "example_item_parent_ancestor" ("scope","descendant","ancestor");
--> statement-breakpoint
CREATE INDEX "example_item_parent_ancestor_parent" ON "example_item"("scope", "parent");
--> statement-breakpoint
CREATE FUNCTION "example_item_parent_ancestor_before"() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
            INSERT INTO "example_item_parent_ancestor_revision"(scope, revision) VALUES (CASE WHEN TG_OP = 'DELETE' THEN OLD."scope" ELSE NEW."scope" END, 1)
            ON CONFLICT (scope) DO UPDATE SET revision = "example_item_parent_ancestor_revision".revision + 1;
            IF TG_OP = 'DELETE' THEN
                IF EXISTS (SELECT 1 FROM "example_item" WHERE "scope" = OLD."scope" AND "parent" = OLD."id") THEN RAISE EXCEPTION 'tree node has children'; END IF;
                RETURN OLD;
            END IF;
            IF TG_OP = 'UPDATE' THEN
                IF NEW."id" IS DISTINCT FROM OLD."id" OR NEW."scope" IS DISTINCT FROM OLD."scope" THEN RAISE EXCEPTION 'tree identity is immutable'; END IF;
                IF EXISTS (SELECT 1 FROM "example_item_parent_ancestor" WHERE scope = OLD."scope" AND ancestor = OLD."id" AND descendant = NEW."parent") THEN RAISE EXCEPTION 'tree move creates a cycle'; END IF;
            END IF;
            IF NEW."parent" IS NOT NULL AND NOT EXISTS (SELECT 1 FROM "example_item" WHERE "scope" = NEW."scope" AND "id" = NEW."parent") THEN RAISE EXCEPTION 'tree parent is missing'; END IF;
            RETURN NEW;
        END $$;
--> statement-breakpoint
CREATE FUNCTION "example_item_parent_ancestor_after"() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
            IF TG_OP = 'INSERT' THEN INSERT INTO "example_item_parent_ancestor"(scope, ancestor, descendant, depth)
        VALUES (NEW."scope", NEW."id", NEW."id", 0);
        INSERT INTO "example_item_parent_ancestor"(scope, ancestor, descendant, depth)
        SELECT scope, ancestor, NEW."id", depth + 1 FROM "example_item_parent_ancestor"
        WHERE scope = NEW."scope" AND descendant = NEW."parent";
            ELSIF TG_OP = 'DELETE' THEN DELETE FROM "example_item_parent_ancestor" WHERE scope = OLD."scope" AND (ancestor = OLD."id" OR descendant = OLD."id");
            ELSIF NEW."parent" IS DISTINCT FROM OLD."parent" THEN DELETE FROM "example_item_parent_ancestor" WHERE scope = OLD."scope"
        AND descendant IN (SELECT descendant FROM "example_item_parent_ancestor" WHERE scope = OLD."scope" AND ancestor = OLD."id")
        AND ancestor IN (SELECT ancestor FROM "example_item_parent_ancestor" WHERE scope = OLD."scope" AND descendant = OLD."id" AND ancestor <> OLD."id");
        INSERT INTO "example_item_parent_ancestor"(scope, ancestor, descendant, depth)
        SELECT above.scope, above.ancestor, below.descendant, above.depth + below.depth + 1
        FROM "example_item_parent_ancestor" above CROSS JOIN "example_item_parent_ancestor" below
        WHERE above.scope = NEW."scope" AND below.scope = NEW."scope"
          AND above.descendant = NEW."parent" AND below.ancestor = NEW."id";
            END IF;
            RETURN NULL;
        END $$;
--> statement-breakpoint
CREATE TRIGGER "example_item_parent_ancestor_check" BEFORE INSERT OR UPDATE OF "id", "scope", "parent" OR DELETE ON "example_item" FOR EACH ROW EXECUTE FUNCTION "example_item_parent_ancestor_before"();
--> statement-breakpoint
CREATE TRIGGER "example_item_parent_ancestor_maintain" AFTER INSERT OR UPDATE OF "parent" OR DELETE ON "example_item" FOR EACH ROW EXECUTE FUNCTION "example_item_parent_ancestor_after"();
