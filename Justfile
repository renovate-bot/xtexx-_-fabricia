dev-pgsql-start:
	mkdir -p data/pgsql
	podman run \
		--name fabricia-postgres \
		-e POSTGRES_USER=fabricia \
		-e POSTGRES_PASSWORD=fabriciadev \
		-v $(pwd)/data/pgsql:/var/lib/postgresql/data \
		-d --replace \
		--user $(whoami) \
		-p 19432:5432 \
		docker.io/library/postgres:alpine

dev-pgsql-stop:
	podman container rm -f fabricia-postgres

[working-directory: 'data']
dev-crayon:
	RUST_BACKTRACE=1 cargo run -p fabricia-crayon
