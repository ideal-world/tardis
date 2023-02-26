#![feature(prelude_import)]
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
use tardis::basic::dto::TardisContext;
use tardis::db::reldb_client::TardisActiveModel;
use tardis::db::sea_orm;
use tardis::db::sea_orm::sea_query::TableCreateStatement;
use tardis::db::sea_orm::*;
use tardis::DeriveCreateTable;
fn main() {}
#[sea_orm(table_name = "examples")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub number: i64,
    pub can_be_null: Option<String>,
    pub _bool: bool,
    pub own_paths: String,
}
#[automatically_derived]
impl ::core::clone::Clone for Model {
    #[inline]
    fn clone(&self) -> Model {
        Model {
            id: ::core::clone::Clone::clone(&self.id),
            number: ::core::clone::Clone::clone(&self.number),
            can_be_null: ::core::clone::Clone::clone(&self.can_be_null),
            _bool: ::core::clone::Clone::clone(&self._bool),
            own_paths: ::core::clone::Clone::clone(&self.own_paths),
        }
    }
}
#[automatically_derived]
impl ::core::fmt::Debug for Model {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::debug_struct_field5_finish(
            f,
            "Model",
            "id",
            &&self.id,
            "number",
            &&self.number,
            "can_be_null",
            &&self.can_be_null,
            "_bool",
            &&self._bool,
            "own_paths",
            &&self.own_paths,
        )
    }
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for Model {}
#[automatically_derived]
impl ::core::cmp::PartialEq for Model {
    #[inline]
    fn eq(&self, other: &Model) -> bool {
        self.id == other.id && self.number == other.number
            && self.can_be_null == other.can_be_null && self._bool == other._bool
            && self.own_paths == other.own_paths
    }
}
#[automatically_derived]
impl ::core::marker::StructuralEq for Model {}
#[automatically_derived]
impl ::core::cmp::Eq for Model {
    #[inline]
    #[doc(hidden)]
    #[no_coverage]
    fn assert_receiver_is_total_eq(&self) -> () {
        let _: ::core::cmp::AssertParamIsEq<String>;
        let _: ::core::cmp::AssertParamIsEq<i64>;
        let _: ::core::cmp::AssertParamIsEq<Option<String>>;
        let _: ::core::cmp::AssertParamIsEq<bool>;
    }
}
pub enum Column {
    Id,
    Number,
    CanBeNull,
    #[sea_orm(column_name = "bool")]
    Bool,
    OwnPaths,
}
#[automatically_derived]
impl ::core::marker::Copy for Column {}
#[automatically_derived]
impl ::core::clone::Clone for Column {
    #[inline]
    fn clone(&self) -> Column {
        *self
    }
}
#[automatically_derived]
impl ::core::fmt::Debug for Column {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::write_str(
            f,
            match self {
                Column::Id => "Id",
                Column::Number => "Number",
                Column::CanBeNull => "CanBeNull",
                Column::Bool => "Bool",
                Column::OwnPaths => "OwnPaths",
            },
        )
    }
}
#[allow(missing_docs)]
pub struct ColumnIter {
    idx: usize,
    back_idx: usize,
    marker: ::core::marker::PhantomData<()>,
}
impl ColumnIter {
    fn get(&self, idx: usize) -> Option<Column> {
        match idx {
            0usize => ::core::option::Option::Some(Column::Id),
            1usize => ::core::option::Option::Some(Column::Number),
            2usize => ::core::option::Option::Some(Column::CanBeNull),
            3usize => ::core::option::Option::Some(Column::Bool),
            4usize => ::core::option::Option::Some(Column::OwnPaths),
            _ => ::core::option::Option::None,
        }
    }
}
impl sea_orm::strum::IntoEnumIterator for Column {
    type Iterator = ColumnIter;
    fn iter() -> ColumnIter {
        ColumnIter {
            idx: 0,
            back_idx: 0,
            marker: ::core::marker::PhantomData,
        }
    }
}
impl Iterator for ColumnIter {
    type Item = Column;
    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        self.nth(0)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let t = if self.idx + self.back_idx >= 5usize {
            0
        } else {
            5usize - self.idx - self.back_idx
        };
        (t, Some(t))
    }
    fn nth(&mut self, n: usize) -> Option<<Self as Iterator>::Item> {
        let idx = self.idx + n + 1;
        if idx + self.back_idx > 5usize {
            self.idx = 5usize;
            None
        } else {
            self.idx = idx;
            self.get(idx - 1)
        }
    }
}
impl ExactSizeIterator for ColumnIter {
    fn len(&self) -> usize {
        self.size_hint().0
    }
}
impl DoubleEndedIterator for ColumnIter {
    fn next_back(&mut self) -> Option<<Self as Iterator>::Item> {
        let back_idx = self.back_idx + 1;
        if self.idx + back_idx > 5usize {
            self.back_idx = 5usize;
            None
        } else {
            self.back_idx = back_idx;
            self.get(5usize - self.back_idx)
        }
    }
}
impl Clone for ColumnIter {
    fn clone(&self) -> ColumnIter {
        ColumnIter {
            idx: self.idx,
            back_idx: self.back_idx,
            marker: self.marker.clone(),
        }
    }
}
#[automatically_derived]
impl Column {
    fn default_as_str(&self) -> &str {
        match self {
            Self::Id => "id",
            Self::Number => "number",
            Self::CanBeNull => "can_be_null",
            Self::Bool => "bool",
            Self::OwnPaths => "own_paths",
        }
    }
}
#[automatically_derived]
impl std::str::FromStr for Column {
    type Err = sea_orm::ColumnFromStrErr;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "id" | "id" => Ok(Column::Id),
            "number" | "number" => Ok(Column::Number),
            "can_be_null" | "canBeNull" => Ok(Column::CanBeNull),
            "bool" | "bool" => Ok(Column::Bool),
            "own_paths" | "ownPaths" => Ok(Column::OwnPaths),
            _ => Err(sea_orm::ColumnFromStrErr(s.to_owned())),
        }
    }
}
#[automatically_derived]
impl sea_orm::Iden for Column {
    fn unquoted(&self, s: &mut dyn std::fmt::Write) {
        s.write_fmt(format_args!("{0}", sea_orm::IdenStatic::as_str(self))).unwrap();
    }
}
#[automatically_derived]
impl sea_orm::IdenStatic for Column {
    fn as_str(&self) -> &str {
        self.default_as_str()
    }
}
#[automatically_derived]
impl sea_orm::prelude::ColumnTrait for Column {
    type EntityName = Entity;
    fn def(&self) -> sea_orm::prelude::ColumnDef {
        match self {
            Self::Id => {
                sea_orm::prelude::ColumnTypeTrait::def(
                    sea_orm::prelude::ColumnType::String(None),
                )
            }
            Self::Number => {
                sea_orm::prelude::ColumnTypeTrait::def(
                    sea_orm::prelude::ColumnType::BigInteger,
                )
            }
            Self::CanBeNull => {
                sea_orm::prelude::ColumnTypeTrait::def(
                        sea_orm::prelude::ColumnType::String(None),
                    )
                    .nullable()
            }
            Self::Bool => {
                sea_orm::prelude::ColumnTypeTrait::def(
                    sea_orm::prelude::ColumnType::Boolean,
                )
            }
            Self::OwnPaths => {
                sea_orm::prelude::ColumnTypeTrait::def(
                    sea_orm::prelude::ColumnType::String(None),
                )
            }
        }
    }
    fn select_as(
        &self,
        expr: sea_orm::sea_query::Expr,
    ) -> sea_orm::sea_query::SimpleExpr {
        match self {
            _ => sea_orm::prelude::ColumnTrait::select_enum_as(self, expr),
        }
    }
    fn save_as(&self, val: sea_orm::sea_query::Expr) -> sea_orm::sea_query::SimpleExpr {
        match self {
            _ => sea_orm::prelude::ColumnTrait::save_enum_as(self, val),
        }
    }
}
pub struct Entity;
#[automatically_derived]
impl ::core::marker::Copy for Entity {}
#[automatically_derived]
impl ::core::clone::Clone for Entity {
    #[inline]
    fn clone(&self) -> Entity {
        *self
    }
}
#[automatically_derived]
impl ::core::default::Default for Entity {
    #[inline]
    fn default() -> Entity {
        Entity {}
    }
}
#[automatically_derived]
impl ::core::fmt::Debug for Entity {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::write_str(f, "Entity")
    }
}
#[automatically_derived]
impl sea_orm::entity::EntityTrait for Entity {
    type Model = Model;
    type Column = Column;
    type PrimaryKey = PrimaryKey;
    type Relation = Relation;
}
#[automatically_derived]
impl sea_orm::Iden for Entity {
    fn unquoted(&self, s: &mut dyn std::fmt::Write) {
        s.write_fmt(format_args!("{0}", sea_orm::IdenStatic::as_str(self))).unwrap();
    }
}
#[automatically_derived]
impl sea_orm::IdenStatic for Entity {
    fn as_str(&self) -> &str {
        <Self as sea_orm::EntityName>::table_name(self)
    }
}
#[automatically_derived]
impl sea_orm::prelude::EntityName for Entity {
    fn schema_name(&self) -> Option<&str> {
        None
    }
    fn table_name(&self) -> &str {
        "examples"
    }
}
pub enum PrimaryKey {
    Id,
}
#[automatically_derived]
impl ::core::marker::Copy for PrimaryKey {}
#[automatically_derived]
impl ::core::clone::Clone for PrimaryKey {
    #[inline]
    fn clone(&self) -> PrimaryKey {
        *self
    }
}
#[automatically_derived]
impl ::core::fmt::Debug for PrimaryKey {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::write_str(f, "Id")
    }
}
#[allow(missing_docs)]
pub struct PrimaryKeyIter {
    idx: usize,
    back_idx: usize,
    marker: ::core::marker::PhantomData<()>,
}
impl PrimaryKeyIter {
    fn get(&self, idx: usize) -> Option<PrimaryKey> {
        match idx {
            0usize => ::core::option::Option::Some(PrimaryKey::Id),
            _ => ::core::option::Option::None,
        }
    }
}
impl sea_orm::strum::IntoEnumIterator for PrimaryKey {
    type Iterator = PrimaryKeyIter;
    fn iter() -> PrimaryKeyIter {
        PrimaryKeyIter {
            idx: 0,
            back_idx: 0,
            marker: ::core::marker::PhantomData,
        }
    }
}
impl Iterator for PrimaryKeyIter {
    type Item = PrimaryKey;
    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        self.nth(0)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let t = if self.idx + self.back_idx >= 1usize {
            0
        } else {
            1usize - self.idx - self.back_idx
        };
        (t, Some(t))
    }
    fn nth(&mut self, n: usize) -> Option<<Self as Iterator>::Item> {
        let idx = self.idx + n + 1;
        if idx + self.back_idx > 1usize {
            self.idx = 1usize;
            None
        } else {
            self.idx = idx;
            self.get(idx - 1)
        }
    }
}
impl ExactSizeIterator for PrimaryKeyIter {
    fn len(&self) -> usize {
        self.size_hint().0
    }
}
impl DoubleEndedIterator for PrimaryKeyIter {
    fn next_back(&mut self) -> Option<<Self as Iterator>::Item> {
        let back_idx = self.back_idx + 1;
        if self.idx + back_idx > 1usize {
            self.back_idx = 1usize;
            None
        } else {
            self.back_idx = back_idx;
            self.get(1usize - self.back_idx)
        }
    }
}
impl Clone for PrimaryKeyIter {
    fn clone(&self) -> PrimaryKeyIter {
        PrimaryKeyIter {
            idx: self.idx,
            back_idx: self.back_idx,
            marker: self.marker.clone(),
        }
    }
}
#[automatically_derived]
impl sea_orm::Iden for PrimaryKey {
    fn unquoted(&self, s: &mut dyn std::fmt::Write) {
        s.write_fmt(format_args!("{0}", sea_orm::IdenStatic::as_str(self))).unwrap();
    }
}
#[automatically_derived]
impl sea_orm::IdenStatic for PrimaryKey {
    fn as_str(&self) -> &str {
        match self {
            Self::Id => "id",
        }
    }
}
#[automatically_derived]
impl sea_orm::PrimaryKeyToColumn for PrimaryKey {
    type Column = Column;
    fn into_column(self) -> Self::Column {
        match self {
            Self::Id => Self::Column::Id,
        }
    }
    fn from_column(col: Self::Column) -> Option<Self> {
        match col {
            Self::Column::Id => Some(Self::Id),
            _ => None,
        }
    }
}
#[automatically_derived]
impl PrimaryKeyTrait for PrimaryKey {
    type ValueType = String;
    fn auto_increment() -> bool {
        false
    }
}
#[automatically_derived]
impl sea_orm::FromQueryResult for Model {
    fn from_query_result(
        row: &sea_orm::QueryResult,
        pre: &str,
    ) -> std::result::Result<Self, sea_orm::DbErr> {
        Ok(Self {
            id: row
                .try_get(
                    pre,
                    sea_orm::IdenStatic::as_str(
                            &<<Self as sea_orm::ModelTrait>::Entity as sea_orm::entity::EntityTrait>::Column::Id,
                        )
                        .into(),
                )?,
            number: row
                .try_get(
                    pre,
                    sea_orm::IdenStatic::as_str(
                            &<<Self as sea_orm::ModelTrait>::Entity as sea_orm::entity::EntityTrait>::Column::Number,
                        )
                        .into(),
                )?,
            can_be_null: row
                .try_get(
                    pre,
                    sea_orm::IdenStatic::as_str(
                            &<<Self as sea_orm::ModelTrait>::Entity as sea_orm::entity::EntityTrait>::Column::CanBeNull,
                        )
                        .into(),
                )?,
            _bool: row
                .try_get(
                    pre,
                    sea_orm::IdenStatic::as_str(
                            &<<Self as sea_orm::ModelTrait>::Entity as sea_orm::entity::EntityTrait>::Column::Bool,
                        )
                        .into(),
                )?,
            own_paths: row
                .try_get(
                    pre,
                    sea_orm::IdenStatic::as_str(
                            &<<Self as sea_orm::ModelTrait>::Entity as sea_orm::entity::EntityTrait>::Column::OwnPaths,
                        )
                        .into(),
                )?,
        })
    }
}
#[automatically_derived]
impl sea_orm::ModelTrait for Model {
    type Entity = Entity;
    fn get(
        &self,
        c: <Self::Entity as sea_orm::entity::EntityTrait>::Column,
    ) -> sea_orm::Value {
        match c {
            <Self::Entity as sea_orm::entity::EntityTrait>::Column::Id => {
                self.id.clone().into()
            }
            <Self::Entity as sea_orm::entity::EntityTrait>::Column::Number => {
                self.number.clone().into()
            }
            <Self::Entity as sea_orm::entity::EntityTrait>::Column::CanBeNull => {
                self.can_be_null.clone().into()
            }
            <Self::Entity as sea_orm::entity::EntityTrait>::Column::Bool => {
                self._bool.clone().into()
            }
            <Self::Entity as sea_orm::entity::EntityTrait>::Column::OwnPaths => {
                self.own_paths.clone().into()
            }
            _ => {
                ::core::panicking::panic_fmt(
                    format_args!("field does not exist on Model"),
                )
            }
        }
    }
    fn set(
        &mut self,
        c: <Self::Entity as sea_orm::entity::EntityTrait>::Column,
        v: sea_orm::Value,
    ) {
        match c {
            <Self::Entity as sea_orm::entity::EntityTrait>::Column::Id => {
                self.id = v.unwrap();
            }
            <Self::Entity as sea_orm::entity::EntityTrait>::Column::Number => {
                self.number = v.unwrap();
            }
            <Self::Entity as sea_orm::entity::EntityTrait>::Column::CanBeNull => {
                self.can_be_null = v.unwrap();
            }
            <Self::Entity as sea_orm::entity::EntityTrait>::Column::Bool => {
                self._bool = v.unwrap();
            }
            <Self::Entity as sea_orm::entity::EntityTrait>::Column::OwnPaths => {
                self.own_paths = v.unwrap();
            }
            _ => {
                ::core::panicking::panic_fmt(
                    format_args!("field does not exist on Model"),
                )
            }
        }
    }
}
pub struct ActiveModel {
    pub id: sea_orm::ActiveValue<String>,
    pub number: sea_orm::ActiveValue<i64>,
    pub can_be_null: sea_orm::ActiveValue<Option<String>>,
    pub _bool: sea_orm::ActiveValue<bool>,
    pub own_paths: sea_orm::ActiveValue<String>,
}
#[automatically_derived]
impl ::core::clone::Clone for ActiveModel {
    #[inline]
    fn clone(&self) -> ActiveModel {
        ActiveModel {
            id: ::core::clone::Clone::clone(&self.id),
            number: ::core::clone::Clone::clone(&self.number),
            can_be_null: ::core::clone::Clone::clone(&self.can_be_null),
            _bool: ::core::clone::Clone::clone(&self._bool),
            own_paths: ::core::clone::Clone::clone(&self.own_paths),
        }
    }
}
#[automatically_derived]
impl ::core::fmt::Debug for ActiveModel {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::debug_struct_field5_finish(
            f,
            "ActiveModel",
            "id",
            &&self.id,
            "number",
            &&self.number,
            "can_be_null",
            &&self.can_be_null,
            "_bool",
            &&self._bool,
            "own_paths",
            &&self.own_paths,
        )
    }
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for ActiveModel {}
#[automatically_derived]
impl ::core::cmp::PartialEq for ActiveModel {
    #[inline]
    fn eq(&self, other: &ActiveModel) -> bool {
        self.id == other.id && self.number == other.number
            && self.can_be_null == other.can_be_null && self._bool == other._bool
            && self.own_paths == other.own_paths
    }
}
#[automatically_derived]
impl std::default::Default for ActiveModel {
    fn default() -> Self {
        <Self as sea_orm::ActiveModelBehavior>::new()
    }
}
#[automatically_derived]
impl std::convert::From<<Entity as EntityTrait>::Model> for ActiveModel {
    fn from(m: <Entity as EntityTrait>::Model) -> Self {
        Self {
            id: sea_orm::ActiveValue::unchanged(m.id),
            number: sea_orm::ActiveValue::unchanged(m.number),
            can_be_null: sea_orm::ActiveValue::unchanged(m.can_be_null),
            _bool: sea_orm::ActiveValue::unchanged(m._bool),
            own_paths: sea_orm::ActiveValue::unchanged(m.own_paths),
        }
    }
}
#[automatically_derived]
impl sea_orm::IntoActiveModel<ActiveModel> for <Entity as EntityTrait>::Model {
    fn into_active_model(self) -> ActiveModel {
        self.into()
    }
}
#[automatically_derived]
impl sea_orm::ActiveModelTrait for ActiveModel {
    type Entity = Entity;
    fn take(
        &mut self,
        c: <Self::Entity as EntityTrait>::Column,
    ) -> sea_orm::ActiveValue<sea_orm::Value> {
        match c {
            <Self::Entity as EntityTrait>::Column::Id => {
                let mut value = sea_orm::ActiveValue::not_set();
                std::mem::swap(&mut value, &mut self.id);
                value.into_wrapped_value()
            }
            <Self::Entity as EntityTrait>::Column::Number => {
                let mut value = sea_orm::ActiveValue::not_set();
                std::mem::swap(&mut value, &mut self.number);
                value.into_wrapped_value()
            }
            <Self::Entity as EntityTrait>::Column::CanBeNull => {
                let mut value = sea_orm::ActiveValue::not_set();
                std::mem::swap(&mut value, &mut self.can_be_null);
                value.into_wrapped_value()
            }
            <Self::Entity as EntityTrait>::Column::Bool => {
                let mut value = sea_orm::ActiveValue::not_set();
                std::mem::swap(&mut value, &mut self._bool);
                value.into_wrapped_value()
            }
            <Self::Entity as EntityTrait>::Column::OwnPaths => {
                let mut value = sea_orm::ActiveValue::not_set();
                std::mem::swap(&mut value, &mut self.own_paths);
                value.into_wrapped_value()
            }
            _ => sea_orm::ActiveValue::not_set(),
        }
    }
    fn get(
        &self,
        c: <Self::Entity as EntityTrait>::Column,
    ) -> sea_orm::ActiveValue<sea_orm::Value> {
        match c {
            <Self::Entity as EntityTrait>::Column::Id => {
                self.id.clone().into_wrapped_value()
            }
            <Self::Entity as EntityTrait>::Column::Number => {
                self.number.clone().into_wrapped_value()
            }
            <Self::Entity as EntityTrait>::Column::CanBeNull => {
                self.can_be_null.clone().into_wrapped_value()
            }
            <Self::Entity as EntityTrait>::Column::Bool => {
                self._bool.clone().into_wrapped_value()
            }
            <Self::Entity as EntityTrait>::Column::OwnPaths => {
                self.own_paths.clone().into_wrapped_value()
            }
            _ => sea_orm::ActiveValue::not_set(),
        }
    }
    fn set(&mut self, c: <Self::Entity as EntityTrait>::Column, v: sea_orm::Value) {
        match c {
            <Self::Entity as EntityTrait>::Column::Id => {
                self.id = sea_orm::ActiveValue::set(v.unwrap());
            }
            <Self::Entity as EntityTrait>::Column::Number => {
                self.number = sea_orm::ActiveValue::set(v.unwrap());
            }
            <Self::Entity as EntityTrait>::Column::CanBeNull => {
                self.can_be_null = sea_orm::ActiveValue::set(v.unwrap());
            }
            <Self::Entity as EntityTrait>::Column::Bool => {
                self._bool = sea_orm::ActiveValue::set(v.unwrap());
            }
            <Self::Entity as EntityTrait>::Column::OwnPaths => {
                self.own_paths = sea_orm::ActiveValue::set(v.unwrap());
            }
            _ => {
                ::core::panicking::panic_fmt(
                    format_args!("This ActiveModel does not have this field"),
                )
            }
        }
    }
    fn not_set(&mut self, c: <Self::Entity as EntityTrait>::Column) {
        match c {
            <Self::Entity as EntityTrait>::Column::Id => {
                self.id = sea_orm::ActiveValue::not_set();
            }
            <Self::Entity as EntityTrait>::Column::Number => {
                self.number = sea_orm::ActiveValue::not_set();
            }
            <Self::Entity as EntityTrait>::Column::CanBeNull => {
                self.can_be_null = sea_orm::ActiveValue::not_set();
            }
            <Self::Entity as EntityTrait>::Column::Bool => {
                self._bool = sea_orm::ActiveValue::not_set();
            }
            <Self::Entity as EntityTrait>::Column::OwnPaths => {
                self.own_paths = sea_orm::ActiveValue::not_set();
            }
            _ => {}
        }
    }
    fn is_not_set(&self, c: <Self::Entity as EntityTrait>::Column) -> bool {
        match c {
            <Self::Entity as EntityTrait>::Column::Id => self.id.is_not_set(),
            <Self::Entity as EntityTrait>::Column::Number => self.number.is_not_set(),
            <Self::Entity as EntityTrait>::Column::CanBeNull => {
                self.can_be_null.is_not_set()
            }
            <Self::Entity as EntityTrait>::Column::Bool => self._bool.is_not_set(),
            <Self::Entity as EntityTrait>::Column::OwnPaths => {
                self.own_paths.is_not_set()
            }
            _ => {
                ::core::panicking::panic_fmt(
                    format_args!("This ActiveModel does not have this field"),
                )
            }
        }
    }
    fn default() -> Self {
        Self {
            id: sea_orm::ActiveValue::not_set(),
            number: sea_orm::ActiveValue::not_set(),
            can_be_null: sea_orm::ActiveValue::not_set(),
            _bool: sea_orm::ActiveValue::not_set(),
            own_paths: sea_orm::ActiveValue::not_set(),
        }
    }
    fn reset(&mut self, c: <Self::Entity as EntityTrait>::Column) {
        match c {
            <Self::Entity as EntityTrait>::Column::Id => self.id.reset(),
            <Self::Entity as EntityTrait>::Column::Number => self.number.reset(),
            <Self::Entity as EntityTrait>::Column::CanBeNull => self.can_be_null.reset(),
            <Self::Entity as EntityTrait>::Column::Bool => self._bool.reset(),
            <Self::Entity as EntityTrait>::Column::OwnPaths => self.own_paths.reset(),
            _ => {
                ::core::panicking::panic_fmt(
                    format_args!("This ActiveModel does not have this field"),
                )
            }
        }
    }
}
#[automatically_derived]
impl std::convert::TryFrom<ActiveModel> for <Entity as EntityTrait>::Model {
    type Error = sea_orm::DbErr;
    fn try_from(a: ActiveModel) -> Result<Self, sea_orm::DbErr> {
        if match a.id {
            sea_orm::ActiveValue::NotSet => true,
            _ => false,
        } {
            return Err(sea_orm::DbErr::AttrNotSet("id".to_owned()));
        }
        if match a.number {
            sea_orm::ActiveValue::NotSet => true,
            _ => false,
        } {
            return Err(sea_orm::DbErr::AttrNotSet("number".to_owned()));
        }
        if match a.can_be_null {
            sea_orm::ActiveValue::NotSet => true,
            _ => false,
        } {
            return Err(sea_orm::DbErr::AttrNotSet("can_be_null".to_owned()));
        }
        if match a._bool {
            sea_orm::ActiveValue::NotSet => true,
            _ => false,
        } {
            return Err(sea_orm::DbErr::AttrNotSet("_bool".to_owned()));
        }
        if match a.own_paths {
            sea_orm::ActiveValue::NotSet => true,
            _ => false,
        } {
            return Err(sea_orm::DbErr::AttrNotSet("own_paths".to_owned()));
        }
        Ok(Self {
            id: a.id.into_value().unwrap().unwrap(),
            number: a.number.into_value().unwrap().unwrap(),
            can_be_null: a.can_be_null.into_value().unwrap().unwrap(),
            _bool: a._bool.into_value().unwrap().unwrap(),
            own_paths: a.own_paths.into_value().unwrap().unwrap(),
        })
    }
}
#[automatically_derived]
impl sea_orm::TryIntoModel<<Entity as EntityTrait>::Model> for ActiveModel {
    fn try_into_model(self) -> Result<<Entity as EntityTrait>::Model, sea_orm::DbErr> {
        self.try_into()
    }
}
impl TardisActiveModel for ActiveModel {
    fn fill_ctx(&mut self, ctx: &TardisContext, is_insert: bool) {
        if is_insert {
            self.own_paths = Set(ctx.own_paths.to_string());
        }
    }
    ///调用macros自动生成的方法 create_table_statement
    fn create_table_statement(db: DbBackend) -> TableCreateStatement {
        create_table_statement(db)
    }
}
impl ActiveModelBehavior for ActiveModel {}
pub enum Relation {}
#[automatically_derived]
impl ::core::marker::Copy for Relation {}
#[automatically_derived]
impl ::core::clone::Clone for Relation {
    #[inline]
    fn clone(&self) -> Relation {
        *self
    }
}
#[automatically_derived]
impl ::core::fmt::Debug for Relation {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        unsafe { ::core::intrinsics::unreachable() }
    }
}
#[allow(missing_docs)]
pub struct RelationIter {
    idx: usize,
    back_idx: usize,
    marker: ::core::marker::PhantomData<()>,
}
impl RelationIter {
    fn get(&self, idx: usize) -> Option<Relation> {
        match idx {
            _ => ::core::option::Option::None,
        }
    }
}
impl sea_orm::strum::IntoEnumIterator for Relation {
    type Iterator = RelationIter;
    fn iter() -> RelationIter {
        RelationIter {
            idx: 0,
            back_idx: 0,
            marker: ::core::marker::PhantomData,
        }
    }
}
impl Iterator for RelationIter {
    type Item = Relation;
    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        self.nth(0)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let t = if self.idx + self.back_idx >= 0usize {
            0
        } else {
            0usize - self.idx - self.back_idx
        };
        (t, Some(t))
    }
    fn nth(&mut self, n: usize) -> Option<<Self as Iterator>::Item> {
        let idx = self.idx + n + 1;
        if idx + self.back_idx > 0usize {
            self.idx = 0usize;
            None
        } else {
            self.idx = idx;
            self.get(idx - 1)
        }
    }
}
impl ExactSizeIterator for RelationIter {
    fn len(&self) -> usize {
        self.size_hint().0
    }
}
impl DoubleEndedIterator for RelationIter {
    fn next_back(&mut self) -> Option<<Self as Iterator>::Item> {
        let back_idx = self.back_idx + 1;
        if self.idx + back_idx > 0usize {
            self.back_idx = 0usize;
            None
        } else {
            self.back_idx = back_idx;
            self.get(0usize - self.back_idx)
        }
    }
}
impl Clone for RelationIter {
    fn clone(&self) -> RelationIter {
        RelationIter {
            idx: self.idx,
            back_idx: self.back_idx,
            marker: self.marker.clone(),
        }
    }
}
#[automatically_derived]
impl sea_orm::entity::RelationTrait for Relation {
    fn def(&self) -> sea_orm::entity::RelationDef {
        match self {
            _ => {
                ::core::panicking::panic_fmt(format_args!("No RelationDef for Relation"))
            }
        }
    }
}
