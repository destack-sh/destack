CREATE TABLE `tree_node` (
	`id` text PRIMARY KEY,
	`scope` text NOT NULL,
	`parent` text,
	CONSTRAINT "tree_node_id_not_null" CHECK("id" IS NOT NULL)
);
