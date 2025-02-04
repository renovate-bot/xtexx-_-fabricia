-- Branch
CREATE TABLE `branch`(
	`id` INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
	`name` VARCHAR(32) NOT NULL,
	`state` SMALLINT NOT NULL DEFAULT 0,
	`status` VARCHAR(256) NOT NULL DEFAULT '',
	`priority` SMALLINT NOT NULL DEFAULT 100,
	`total_srcpkgs` INT NOT NULL DEFAULT 0,
	`commitish` VARCHAR(64) NULL DEFAULT NULL
);
CREATE UNIQUE INDEX `branch_id` ON `branch` (`id`);
CREATE UNIQUE INDEX `branch_name` ON `branch` (`name`);
CREATE INDEX `branch_state` ON `branch` (`state`);
CREATE INDEX `branch_priority` ON `branch` (`priority` DESC);
-- Package
CREATE TABLE `pkg`(
	`id` UUID NOT NULL PRIMARY KEY,
	`branch` BIGINT NOT NULL,
	`name` VARCHAR(32) NOT NULL,
	`section` VARCHAR(32) NOT NULL,
	`state` SMALLINT NOT NULL,
	`status` VARCHAR(256) NOT NULL,
	`data` JSONB NOT NULL
);
CREATE UNIQUE INDEX `pkg_id` ON `pkg` (`id`);
CREATE UNIQUE INDEX `pkg_br_name` ON `pkg` (`branch`, `name`);
CREATE INDEX `pkg_state` ON `pkg` (`state`);
CREATE INDEX `pkg_br_state` ON `pkg` (`branch`, `state`);
-- Package + Target
CREATE TABLE `pkg_target`(
	`id` UUID NOT NULL PRIMARY KEY,
	`branch` BIGINT NOT NULL,
	`package` UUID NOT NULL,
	`target` BIGINT NOT NULL,
	`state` SMALLINT NOT NULL,
	`data` JSONB NOT NULL
);
CREATE UNIQUE INDEX `pkg_target_id` ON `pkg_target` (`id`);
CREATE INDEX `pkg_target_br` ON `pkg_target` (`branch`);
CREATE UNIQUE INDEX `pkg_target_br_tgt` ON `pkg_target` (`package`, `target`);
CREATE INDEX `pkg_target_state` ON `pkg_target` (`state`);
-- Lightweight Job Queue
CREATE TABLE `job_queue`(
	`id` UUID NOT NULL PRIMARY KEY,
	`kind` VARCHAR NOT NULL,
	`data` JSONB NOT NULL,
	`priority` SMALLINT NOT NULL,
	`started_at` TIMESTAMP NULL
);
CREATE UNIQUE INDEX `job_queue_id` ON `job_queue` (`id`);
CREATE INDEX `job_queue_poll` ON `job_queue` (`kind`, (`started_at` IS NULL), `priority` DESC);
