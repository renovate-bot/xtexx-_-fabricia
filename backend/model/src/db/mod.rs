use diesel::{
	QueryResult, Queryable, RunQueryDsl, Selectable, SelectableHelper, SqliteConnection,
	connection::{AnsiTransactionManager, SimpleConnection, TransactionManager},
	dsl::{AsSelect, Limit},
	expression::{AsExpression, TypedExpressionType},
	migration::MigrationVersion,
	pg::Pg,
	query_builder::{AsQuery, QueryId},
	query_dsl::methods::{ExecuteDsl, LimitDsl, LoadQuery, SelectDsl},
	sql_types::{self, HasSqlType, SqlType},
	sqlite::Sqlite,
};
use diesel_async::{
	AnsiTransactionManager as AsyncAnsiTransactionManager, AsyncPgConnection,
	RunQueryDsl as AsyncRunQueryDsl, SimpleAsyncConnection,
	TransactionManager as AsyncTransactionManager,
	async_connection_wrapper::AsyncConnectionWrapper,
	methods::{ExecuteDsl as AsyncExecuteDsl, LoadQuery as AsyncLoadQuery},
	pooled_connection::PoolableConnection,
};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use futures::future::{BoxFuture, FutureExt, ready};

pub mod schema;
pub mod utils;

/// A specialized SQL backend.
pub trait SqlBackend: diesel::backend::Backend
where
	Self: HasSqlType<utils::XJson>,
	Self: HasSqlType<utils::XUuid>,
	Self: HasSqlType<sql_types::Text>,
{
	type Connection;
}
impl SqlBackend for Pg {
	type Connection = AsyncPgConnection;
}
impl SqlBackend for Sqlite {
	type Connection = SqliteConnection;
}

/// A specialized SQL connection.
pub trait SqlConnection<DB: SqlBackend> {}
impl SqlConnection<Pg> for AsyncPgConnection {}
impl SqlConnection<Sqlite> for SqliteConnection {}

pub enum BoxedSqlConn {
	Pg(AsyncPgConnection),
	Sqlite(SqliteConnection),
}

impl BoxedSqlConn {
	/// Executes `SELECT 1` to test if the connection is ready for use.
	pub fn ping(&mut self) -> BoxFuture<Result<(), diesel::result::Error>> {
		match self {
			BoxedSqlConn::Pg(conn) => conn.batch_execute("SELECT 1").boxed(),
			BoxedSqlConn::Sqlite(conn) => ready(conn.batch_execute("SELECT 1")).boxed(),
		}
	}

	pub fn is_broken(&mut self) -> bool {
		match self {
			BoxedSqlConn::Pg(conn) => conn.is_broken(),
			BoxedSqlConn::Sqlite(conn) => {
				AnsiTransactionManager::is_broken_transaction_manager(conn)
			}
		}
	}
}

impl BoxedSqlConn {
	pub async fn transaction<R, E, F>(&mut self, callback: F) -> Result<R, E>
	where
		F: AsyncFnOnce(&mut Self) -> Result<R, E>,
		E: From<diesel::result::Error> + Send,
		R: Send,
	{
		match self {
			BoxedSqlConn::Pg(conn) => {
				AsyncAnsiTransactionManager::begin_transaction(conn).await?;
			}
			BoxedSqlConn::Sqlite(conn) => {
				AnsiTransactionManager::begin_transaction(conn)?;
			}
		}
		match callback(self).await {
			Ok(value) => {
				match self {
					BoxedSqlConn::Pg(conn) => {
						AsyncAnsiTransactionManager::commit_transaction(conn).await?;
					}
					BoxedSqlConn::Sqlite(conn) => {
						AnsiTransactionManager::commit_transaction(conn)?;
					}
				}
				Ok(value)
			}
			Err(user_error) => {
				let result = match self {
					BoxedSqlConn::Pg(conn) => {
						AsyncAnsiTransactionManager::rollback_transaction(conn).await
					}
					BoxedSqlConn::Sqlite(conn) => {
						AnsiTransactionManager::rollback_transaction(conn)
					}
				};
				match result {
					Ok(()) => Err(user_error),
					Err(diesel::result::Error::BrokenTransactionManager) => {
						// In this case we are probably more interested by the
						// original error, which likely caused this
						Err(user_error)
					}
					Err(rollback_error) => Err(rollback_error.into()),
				}
			}
		}
	}
}

impl<'query> BoxedSqlConn {
	/// Executes the given command, returning the number of rows affected.
	///
	/// `execute` is usually used in conjunction with [`insert_into`](diesel::insert_into()),
	/// [`update`](diesel::update()) and [`delete`](diesel::delete()) where the number of
	/// affected rows is often enough information.
	///
	/// When asking the database to return data from a query, [`load`](DslDispatchExt::load()) should
	/// probably be used instead.
	///
	/// Dispatches [RunQueryDsl::execute].
	pub fn execute<Q>(&mut self, query: Q) -> BoxFuture<'query, QueryResult<usize>>
	where
		Q: AsQuery,
		Q: AsyncExecuteDsl<AsyncPgConnection> + 'query,
		Q: ExecuteDsl<SqliteConnection>,
	{
		match self {
			BoxedSqlConn::Pg(conn) => AsyncExecuteDsl::execute(query, conn),
			BoxedSqlConn::Sqlite(conn) => ready(ExecuteDsl::execute(query, conn)).boxed(),
		}
	}

	/// Executes the given query, returning a [`Vec`] with the returned rows.
	///
	/// For insert, update, and delete operations where only a count of affected is needed,
	/// [`execute`] should be used instead.
	///
	/// Dispatches [RunQueryDsl::load].
	pub fn load<'conn, Q, U>(&'conn mut self, query: Q) -> BoxFuture<'query, QueryResult<Vec<U>>>
	where
		Q: Send,
		Q: AsyncLoadQuery<'query, AsyncPgConnection, U> + 'query,
		Q: LoadQuery<'query, SqliteConnection, U>,
		U: Send + 'query,
		'conn: 'query,
	{
		match self {
			BoxedSqlConn::Pg(conn) => AsyncRunQueryDsl::load(query, conn).boxed(),
			BoxedSqlConn::Sqlite(conn) => ready(RunQueryDsl::load(query, conn)).boxed(),
		}
	}

	/// Runs the command, and returns the affected row.
	///
	/// `Err(NotFound)` will be returned if the query affected 0 rows. You can
	/// call `.optional()` on the result of this if the command was optional to
	/// get back a `Result<Option<U>>`
	///
	/// Dispatches [RunQueryDsl::get_result].
	pub fn get_result<Q, U>(&'query mut self, query: Q) -> BoxFuture<'query, QueryResult<U>>
	where
		Q: AsQuery + Send,
		Q: AsyncLoadQuery<'query, AsyncPgConnection, U> + 'query,
		Q: LoadQuery<'query, SqliteConnection, U>,
		U: Send + 'query,
	{
		match self {
			BoxedSqlConn::Pg(conn) => AsyncRunQueryDsl::get_result(query, conn).boxed(),
			BoxedSqlConn::Sqlite(conn) => ready(RunQueryDsl::get_result(query, conn)).boxed(),
		}
	}

	/// Alias of [`load`][DslDispatchExt::load].
	///
	/// Dispatches [RunQueryDsl::load].
	#[inline]
	pub fn get_results<'conn, Q, U>(
		&'conn mut self,
		query: Q,
	) -> BoxFuture<'query, QueryResult<Vec<U>>>
	where
		Q: AsQuery + Send,
		Q: AsyncLoadQuery<'query, AsyncPgConnection, U> + 'query,
		Q: LoadQuery<'query, SqliteConnection, U>,
		U: Send + 'conn,
		'conn: 'query,
	{
		self.load(query)
	}

	/// Runs the command, and returns the affected row.
	///
	/// This method is equivalent to `.limit(1).get_result()`
	///
	/// `Err(NotFound)` will be returned if the query affected 0 rows. You can
	/// call `.optional()` on the result of this if the command was optional to
	/// get back a `Result<Option<U>>`
	///
	/// Dispatches [RunQueryDsl::first].
	#[inline]
	pub fn first<'conn, Q, U>(&'conn mut self, query: Q) -> BoxFuture<'query, QueryResult<U>>
	where
		Q: AsQuery + LimitDsl + Send,
		Limit<Q>: AsyncLoadQuery<'query, AsyncPgConnection, U> + Send + 'query,
		Limit<Q>: LoadQuery<'query, SqliteConnection, U>,
		U: Send + 'conn,
		'conn: 'query,
	{
		match self {
			BoxedSqlConn::Pg(conn) => AsyncRunQueryDsl::first(query, conn).boxed(),
			BoxedSqlConn::Sqlite(conn) => {
				ready(RunQueryDsl::get_result(LimitDsl::limit(query, 1), conn)).boxed()
			}
		}
	}

	pub fn load_select<'conn, Q, S, E>(
		&'conn mut self,
		query: Q,
	) -> BoxFuture<'query, QueryResult<Vec<S>>>
	where
		Q: SelectDsl<AsSelect<S, Pg>>,
		Q: SelectDsl<AsSelect<S, Sqlite>>,
		<Q as SelectDsl<AsSelect<S, Pg>>>::Output:
			AsyncLoadQuery<'query, AsyncPgConnection, S> + Send + 'query,
		<Q as SelectDsl<AsSelect<S, Sqlite>>>::Output: LoadQuery<'query, SqliteConnection, S>,
		S: Selectable<Pg> + Queryable<E, Pg>,
		S: Selectable<Sqlite> + Queryable<E, Sqlite>,
		<S as Selectable<Pg>>::SelectExpression: QueryId + AsExpression<E>,
		<S as Selectable<Sqlite>>::SelectExpression: QueryId + AsExpression<E>,
		S: Send + 'query,
		E: TypedExpressionType + SqlType,
		'conn: 'query,
	{
		match self {
			BoxedSqlConn::Pg(conn) => AsyncRunQueryDsl::load(
				<Q as SelectDsl<AsSelect<S, Pg>>>::select(
					query,
					<S as SelectableHelper<Pg>>::as_select(),
				),
				conn,
			)
			.boxed(),
			BoxedSqlConn::Sqlite(conn) => ready(RunQueryDsl::load(
				<Q as SelectDsl<AsSelect<S, Sqlite>>>::select(
					query,
					<S as SelectableHelper<Sqlite>>::as_select(),
				),
				conn,
			))
			.boxed(),
		}
	}

	/// Loads one row.
	///
	/// Note that caller must set limit to 1.
	pub fn load_one_select<'conn, Q, S, E>(
		&'conn mut self,
		query: Q,
	) -> BoxFuture<'query, QueryResult<S>>
	where
		Q: SelectDsl<AsSelect<S, Pg>>,
		Q: SelectDsl<AsSelect<S, Sqlite>>,
		<Q as SelectDsl<AsSelect<S, Pg>>>::Output:
			AsyncLoadQuery<'query, AsyncPgConnection, S> + Send + 'query,
		<Q as SelectDsl<AsSelect<S, Sqlite>>>::Output: LoadQuery<'query, SqliteConnection, S>,
		S: Selectable<Pg> + Queryable<E, Pg>,
		S: Selectable<Sqlite> + Queryable<E, Sqlite>,
		<S as Selectable<Pg>>::SelectExpression: QueryId + AsExpression<E>,
		<S as Selectable<Sqlite>>::SelectExpression: QueryId + AsExpression<E>,
		S: Send + 'query,
		E: TypedExpressionType + SqlType,
		'conn: 'query,
	{
		match self {
			BoxedSqlConn::Pg(conn) => AsyncRunQueryDsl::get_result(
				<Q as SelectDsl<AsSelect<S, Pg>>>::select(
					query,
					<S as SelectableHelper<Pg>>::as_select(),
				),
				conn,
			)
			.boxed(),
			BoxedSqlConn::Sqlite(conn) => ready(RunQueryDsl::get_result(
				<Q as SelectDsl<AsSelect<S, Sqlite>>>::select(
					query,
					<S as SelectableHelper<Sqlite>>::as_select(),
				),
				conn,
			))
			.boxed(),
		}
	}
}

const POSTGRESQL_MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/postgresql");
const SQLITE_MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/sqlite");

/// Run all pending migrations.
///
/// This is not async, so a spawn-blocking wrapper is required.
///
/// Dispatches [MigrationHarness::run_pending_migrations].
pub fn run_migrations(
	mut conn: BoxedSqlConn,
) -> diesel::migration::Result<Vec<MigrationVersion<'static>>> {
	match conn {
		BoxedSqlConn::Pg(conn) => {
			let mut async_wrapper: AsyncConnectionWrapper<AsyncPgConnection> =
				AsyncConnectionWrapper::from(conn);
			async_wrapper
				.run_pending_migrations(POSTGRESQL_MIGRATIONS)
				.map(|versions| {
					versions
						.into_iter()
						.map(|version| version.as_owned())
						.collect()
				})
		}
		BoxedSqlConn::Sqlite(_) => run_migrations_sqlite(&mut conn),
	}
}

/// Run migrations for SQLite.
///
/// This is only for running tests with in memory SQLite database,
/// to avoid taking over the connection ownership.
pub fn run_migrations_sqlite(
	conn: &mut BoxedSqlConn,
) -> diesel::migration::Result<Vec<MigrationVersion<'static>>> {
	match conn {
		BoxedSqlConn::Pg(_) => unreachable!(),
		BoxedSqlConn::Sqlite(conn) => {
			conn.run_pending_migrations(SQLITE_MIGRATIONS)
				.map(|versions| {
					versions
						.into_iter()
						.map(|version| version.as_owned())
						.collect()
				})
		}
	}
}

#[cfg(test)]

pub(crate) mod test {
	use diesel::Connection;

	use super::*;

	pub fn make_empty_test_db() -> BoxedSqlConn {
		BoxedSqlConn::Sqlite(SqliteConnection::establish(":memory:").unwrap())
	}

	#[test]
	fn test_sqlite_migrations() {
		let db = make_empty_test_db();
		run_migrations(db).unwrap();
	}
}
