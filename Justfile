dev-pgsql:
	mkdir -p data/pgsql
	podman run \
		--name fabricia-postgres \
		-e POSTGRES_USER=fabricia \
		-e POSTGRES_PASSWORD=fabriciadev \
		-v $(pwd)/data/pgsql:/var/lib/postgresql/data \
		--replace \
		-p 19432:5432 \
		docker.io/library/postgres:alpine

dev-valkey:
	mkdir -p data/pgsql
	podman run \
		--name fabricia-valkey \
		--replace \
		-p 16379:6379 \
		docker.io/valkey/valkey:alpine

[working-directory: 'data']
dev-crayon:
	RUST_BACKTRACE=1 cargo run -p fabricia-crayon
