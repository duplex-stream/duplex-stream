CREATE TABLE `alternatives` (
	`id` text PRIMARY KEY NOT NULL,
	`decision_id` text NOT NULL,
	`description` text NOT NULL,
	`why_rejected` text NOT NULL,
	FOREIGN KEY (`decision_id`) REFERENCES `decisions`(`id`) ON UPDATE no action ON DELETE cascade
);
--> statement-breakpoint
CREATE TABLE `conversations` (
	`id` text PRIMARY KEY NOT NULL,
	`org_id` text NOT NULL,
	`workspace_id` text NOT NULL,
	`source` text NOT NULL,
	`source_path` text NOT NULL,
	`project_path` text NOT NULL,
	`session_id` text,
	`message_count` integer NOT NULL,
	`created_at` text NOT NULL,
	`extracted_at` text
);
--> statement-breakpoint
CREATE TABLE `decision_appearances` (
	`id` text PRIMARY KEY NOT NULL,
	`decision_id` text NOT NULL,
	`message_start` integer NOT NULL,
	`message_end` integer NOT NULL,
	`type` text NOT NULL,
	`context` text,
	FOREIGN KEY (`decision_id`) REFERENCES `decisions`(`id`) ON UPDATE no action ON DELETE cascade
);
--> statement-breakpoint
CREATE TABLE `decision_dependencies` (
	`id` text PRIMARY KEY NOT NULL,
	`from_decision_id` text NOT NULL,
	`to_decision_ref` text NOT NULL,
	FOREIGN KEY (`from_decision_id`) REFERENCES `decisions`(`id`) ON UPDATE no action ON DELETE cascade
);
--> statement-breakpoint
CREATE TABLE `decisions` (
	`id` text PRIMARY KEY NOT NULL,
	`conversation_id` text NOT NULL,
	`org_id` text NOT NULL,
	`workspace_id` text NOT NULL,
	`title` text NOT NULL,
	`summary` text NOT NULL,
	`reasoning` text NOT NULL,
	`status` text NOT NULL,
	`confidence` real NOT NULL,
	`extracted_at` text NOT NULL,
	FOREIGN KEY (`conversation_id`) REFERENCES `conversations`(`id`) ON UPDATE no action ON DELETE cascade
);
--> statement-breakpoint
CREATE TABLE `messages` (
	`id` text PRIMARY KEY NOT NULL,
	`conversation_id` text NOT NULL,
	`index` integer NOT NULL,
	`role` text NOT NULL,
	`content` text NOT NULL,
	`thinking` text,
	`timestamp` text,
	FOREIGN KEY (`conversation_id`) REFERENCES `conversations`(`id`) ON UPDATE no action ON DELETE cascade
);
