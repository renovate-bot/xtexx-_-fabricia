//! Database schema maintenance things.

use std::{
	fmt::Display,
	ops::{Deref, DerefMut},
};

use diesel::{
	AppearsOnTable, Expression,
	deserialize::{self, FromSql, FromSqlRow},
	expression::{AsExpression, NonAggregate},
	pg::{Pg, PgValue},
	query_builder::{QueryFragment, QueryId},
	serialize::{self, IsNull, Output, ToSql},
	sql_types::{Binary, Bool, Jsonb, SqlType, VarChar},
	sqlite::{Sqlite, SqliteValue},
};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[diesel(postgres_type(oid = 2950, array_oid = 2951))]
#[diesel(sqlite_type(name = "Binary"))]
pub struct XUuid;

#[derive(Debug, AsExpression, FromSqlRow, Clone, Copy, PartialEq, Eq)]
#[diesel(sql_type = XUuid)]
pub struct XUuidVal(pub Uuid);

impl Deref for XUuidVal {
	type Target = Uuid;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl DerefMut for XUuidVal {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl AsRef<Uuid> for XUuidVal {
	fn as_ref(&self) -> &Uuid {
		&self.0
	}
}

impl AsMut<Uuid> for XUuidVal {
	fn as_mut(&mut self) -> &mut Uuid {
		&mut self.0
	}
}

impl FromSql<XUuid, Pg> for XUuidVal {
	fn from_sql(value: PgValue<'_>) -> deserialize::Result<Self> {
		Ok(XUuidVal(Uuid::from_slice(value.as_bytes())?))
	}
}

impl ToSql<XUuid, Pg> for XUuidVal {
	fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
		<Uuid as ToSql<diesel::sql_types::Uuid, Pg>>::to_sql(self, out)
	}
}

impl FromSql<XUuid, Sqlite> for XUuidVal {
	fn from_sql(value: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
		let value = <Vec<u8> as FromSql<Binary, Sqlite>>::from_sql(value)?;
		Ok(XUuidVal(Uuid::from_slice(value.as_slice())?))
	}
}

impl ToSql<XUuid, Sqlite> for XUuidVal {
	fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
		<[u8; 16] as ToSql<Binary, Sqlite>>::to_sql(self.as_bytes(), out)
	}
}

impl Display for XUuidVal {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		Display::fmt(&self.0, f)
	}
}

#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[diesel(postgres_type(oid = 3802, array_oid = 3807))]
#[diesel(sqlite_type(name = "Text"))]
pub struct XJson;

#[derive(Debug, AsExpression, FromSqlRow, Clone, PartialEq, Eq)]
#[diesel(sql_type = XJson)]
pub struct XJsonVal(pub serde_json::Value);

impl Deref for XJsonVal {
	type Target = serde_json::Value;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl DerefMut for XJsonVal {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl AsRef<serde_json::Value> for XJsonVal {
	fn as_ref(&self) -> &serde_json::Value {
		&self.0
	}
}

impl AsMut<serde_json::Value> for XJsonVal {
	fn as_mut(&mut self) -> &mut serde_json::Value {
		&mut self.0
	}
}

impl FromSql<XJson, Pg> for XJsonVal {
	fn from_sql(value: PgValue<'_>) -> deserialize::Result<Self> {
		Ok(XJsonVal(
			<serde_json::Value as FromSql<Jsonb, Pg>>::from_sql(value)?,
		))
	}
}

impl ToSql<XJson, Pg> for XJsonVal {
	fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
		<serde_json::Value as ToSql<Jsonb, Pg>>::to_sql(self, out)
	}
}

impl FromSql<XJson, Sqlite> for XJsonVal {
	fn from_sql(value: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
		let value = <String as FromSql<VarChar, Sqlite>>::from_sql(value)?;
		let value = serde_json::from_str(&value)?;
		Ok(XJsonVal(value))
	}
}

impl ToSql<XJson, Sqlite> for XJsonVal {
	fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
		out.set_value(serde_json::to_string(self.as_ref())?);
		Ok(IsNull::No)
	}
}

impl Display for XJsonVal {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		Display::fmt(&self.0, f)
	}
}

pub trait WherePredicate<T>
where
	Self: Send + AppearsOnTable<T> + QueryId,
	Self: QueryFragment<Pg> + QueryFragment<Sqlite>,
	Self: Expression<SqlType = Bool> + NonAggregate,
{
}

impl<T, V> WherePredicate<V> for T
where
	Self: Send + AppearsOnTable<V> + QueryId,
	Self: QueryFragment<Pg> + QueryFragment<Sqlite>,
	Self: Expression<SqlType = Bool> + NonAggregate,
{
}
