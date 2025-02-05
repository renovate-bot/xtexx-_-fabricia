-- Branch
CREATE TABLE "branch"(
	"id" BIGSERIAL NOT NULL PRIMARY KEY,
	"name" VARCHAR(32) NOT NULL,
	"base" INTEGER NULL DEFAULT NULL,
	"status" SMALLINT NOT NULL DEFAULT 0,
	"status_msg" VARCHAR(256) NULL DEFAULT NULL,
	"priority" SMALLINT NOT NULL DEFAULT 100,
	"commit" BYTEA NULL DEFAULT NULL,
	"tracking" SMALLINT NOT NULL,
	"total_srcpkgs" INT NOT NULL DEFAULT 0
);
CREATE UNIQUE INDEX "branch_id" ON "branch" ("id");
CREATE UNIQUE INDEX "branch_name" ON "branch" ("name");
CREATE INDEX "branch_status" ON "branch" ("status");
CREATE INDEX "branch_priority" ON "branch" ("priority" DESC);
-- Package
CREATE TABLE "pkg"(
	"id" UUID NOT NULL PRIMARY KEY,
	"branch" BIGINT NOT NULL,
	"name" VARCHAR(32) NOT NULL,
	"section" VARCHAR(32) NOT NULL,
	"status" SMALLINT NOT NULL,
	"status_msg" VARCHAR(256) NULL DEFAULT NULL,
	"data" JSONB NOT NULL
);
CREATE UNIQUE INDEX "pkg_id" ON "pkg" ("id");
CREATE INDEX "pkg_branch" ON "pkg" ("branch");
CREATE UNIQUE INDEX "pkg_br_name" ON "pkg" ("branch", "name");
CREATE INDEX "pkg_status" ON "pkg" ("status");
CREATE INDEX "pkg_br_status" ON "pkg" ("branch", "status");
-- Package + Target
CREATE TABLE "pkg_target"(
	"id" UUID NOT NULL PRIMARY KEY,
	"branch" BIGINT NOT NULL,
	"package" UUID NOT NULL,
	"target" BIGINT NOT NULL,
	"status" SMALLINT NOT NULL,
	"data" JSONB NOT NULL
);
CREATE UNIQUE INDEX "pkg_target_id" ON "pkg_target" ("id");
CREATE INDEX "pkg_target_br" ON "pkg_target" ("branch");
CREATE UNIQUE INDEX "pkg_target_br_tgt" ON "pkg_target" ("package", "target");
CREATE INDEX "pkg_target_status" ON "pkg_target" ("status");
-- Lightweight Job Queue
CREATE TABLE "job_queue"(
	"id" UUID NOT NULL PRIMARY KEY,
	"kind" VARCHAR NOT NULL,
	"data" JSONB NOT NULL,
	"priority" SMALLINT NOT NULL,
	"started_at" TIMESTAMP NULL
);
CREATE UNIQUE INDEX "job_queue_id" ON "job_queue" ("id");
CREATE INDEX "job_queue_poll" ON "job_queue" ("kind", ("started_at" IS NULL), "priority" DESC);
