CREATE TABLE `example_item` (
	`id` text PRIMARY KEY,
	`scope` text NOT NULL,
	`owner` text NOT NULL,
	`parent` text,
	`row` integer NOT NULL,
	`column` integer NOT NULL,
	`locked` integer NOT NULL,
	`team` integer NOT NULL,
	`protected` integer NOT NULL,
	CONSTRAINT "example_item_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `example_item_parent_ancestor` (
	`scope` text NOT NULL,
	`ancestor` text NOT NULL,
	`descendant` text NOT NULL,
	`depth` integer NOT NULL,
	CONSTRAINT `example_item_parent_ancestor_path` UNIQUE(`scope`,`ancestor`,`descendant`)
);
--> statement-breakpoint
CREATE TABLE `example_item_parent_ancestor_revision` (
	`scope` text PRIMARY KEY,
	`revision` integer NOT NULL,
	CONSTRAINT "example_item_parent_ancestor_revision_scope_not_null" CHECK("scope" IS NOT NULL)
);
--> statement-breakpoint
CREATE INDEX `example_item_parent_ancestor_descendant` ON `example_item_parent_ancestor` (`scope`,`descendant`,`ancestor`);
--> statement-breakpoint
CREATE INDEX "example_item_parent_ancestor_parent" ON "example_item"("scope", "parent");
--> statement-breakpoint
CREATE TRIGGER "example_item_parent_ancestor_insert_check" BEFORE INSERT ON "example_item" BEGIN
                SELECT RAISE(ABORT, 'tree parent is missing') WHERE NEW."parent" IS NOT NULL AND NOT EXISTS (SELECT 1 FROM "example_item" WHERE "scope" = NEW."scope" AND "id" = NEW."parent");
                SELECT RAISE(ABORT, 'tree identity already exists') WHERE EXISTS (SELECT 1 FROM "example_item_parent_ancestor" WHERE scope = NEW."scope" AND descendant = NEW."id");
            END;
--> statement-breakpoint
CREATE TRIGGER "example_item_parent_ancestor_insert" AFTER INSERT ON "example_item" BEGIN INSERT INTO "example_item_parent_ancestor"(scope, ancestor, descendant, depth)
        VALUES (NEW."scope", NEW."id", NEW."id", 0);
        INSERT INTO "example_item_parent_ancestor"(scope, ancestor, descendant, depth)
        SELECT scope, ancestor, NEW."id", depth + 1 FROM "example_item_parent_ancestor"
        WHERE scope = NEW."scope" AND descendant = NEW."parent"; END;
--> statement-breakpoint
CREATE TRIGGER "example_item_parent_ancestor_move_check" BEFORE UPDATE OF "id", "scope", "parent" ON "example_item" BEGIN
                SELECT RAISE(ABORT, 'tree identity is immutable') WHERE NEW."id" IS NOT OLD."id" OR NEW."scope" IS NOT OLD."scope";
                SELECT RAISE(ABORT, 'tree parent is missing') WHERE NEW."parent" IS NOT NULL AND NOT EXISTS (SELECT 1 FROM "example_item" WHERE "scope" = NEW."scope" AND "id" = NEW."parent");
                SELECT RAISE(ABORT, 'tree move creates a cycle') WHERE EXISTS (SELECT 1 FROM "example_item_parent_ancestor" WHERE scope = OLD."scope" AND ancestor = OLD."id" AND descendant = NEW."parent");
            END;
--> statement-breakpoint
CREATE TRIGGER "example_item_parent_ancestor_move" AFTER UPDATE OF "parent" ON "example_item"
                WHEN NEW."parent" IS NOT OLD."parent" BEGIN DELETE FROM "example_item_parent_ancestor" WHERE scope = OLD."scope"
        AND descendant IN (SELECT descendant FROM "example_item_parent_ancestor" WHERE scope = OLD."scope" AND ancestor = OLD."id")
        AND ancestor IN (SELECT ancestor FROM "example_item_parent_ancestor" WHERE scope = OLD."scope" AND descendant = OLD."id" AND ancestor <> OLD."id");
        INSERT INTO "example_item_parent_ancestor"(scope, ancestor, descendant, depth)
        SELECT above.scope, above.ancestor, below.descendant, above.depth + below.depth + 1
        FROM "example_item_parent_ancestor" above CROSS JOIN "example_item_parent_ancestor" below
        WHERE above.scope = NEW."scope" AND below.scope = NEW."scope"
          AND above.descendant = NEW."parent" AND below.ancestor = NEW."id"; END;
--> statement-breakpoint
CREATE TRIGGER "example_item_parent_ancestor_delete_check" BEFORE DELETE ON "example_item" BEGIN
                SELECT RAISE(ABORT, 'tree node has children') WHERE EXISTS (SELECT 1 FROM "example_item" WHERE "scope" = OLD."scope" AND "parent" = OLD."id");
            END;
--> statement-breakpoint
CREATE TRIGGER "example_item_parent_ancestor_delete" AFTER DELETE ON "example_item" BEGIN DELETE FROM "example_item_parent_ancestor" WHERE scope = OLD."scope" AND (ancestor = OLD."id" OR descendant = OLD."id"); END;
