use rusqlite::types::Value;
use std::fmt::Write;
use thiserror::Error;

use crate::parser::QueryConfig;

#[derive(Debug, Error)]
pub enum QueryBuilderError {
    #[error("invalid identifier: {0}")]
    InvalidIdentifier(String),
}

#[derive(Debug, Clone)]
pub struct BuiltQuery {
    pub sql: String,
    pub params: Vec<Value>,
}

fn is_safe_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    for ch in chars {
        if !(ch.is_ascii_alphanumeric() || ch == '_') {
            return false;
        }
    }
    true
}

fn sanitize_identifier(name: &str) -> Result<&str, QueryBuilderError> {
    if !is_safe_identifier(name) {
        return Err(QueryBuilderError::InvalidIdentifier(name.to_string()));
    }
    Ok(name)
}

/// Build a parameterized SQL query from a parsed config.
///
/// # Errors
/// Returns `QueryBuilderError` if any identifier fails validation.
pub fn build_query(config: &QueryConfig) -> Result<BuiltQuery, QueryBuilderError> {
    let mut params: Vec<Value> = Vec::new();

    let table = sanitize_identifier(&config.table)?;
    let has_joins = !config.joins.is_empty();

    let mut select_columns: Vec<String> = Vec::new();

    if !config.columns.is_empty() {
        for column in &config.columns {
            let col = sanitize_identifier(column)?;
            if has_joins {
                select_columns.push(format!("{table}.{col}"));
            } else {
                select_columns.push(col.to_string());
            }
        }
    } else if has_joins {
        select_columns.push(format!("{table}.*"));
    } else {
        select_columns.push("*".to_string());
    }

    for join in &config.joins {
        let join_table = sanitize_identifier(&join.table)?;
        if join.columns.is_empty() {
            select_columns.push(format!("{join_table}.*"));
        } else {
            for col in &join.columns {
                let col = sanitize_identifier(col)?;
                select_columns.push(format!("{join_table}.{col}"));
            }
        }
    }

    let columns_sql = select_columns.join(", ");
    let mut sql = format!("SELECT {columns_sql} FROM {table}");

    for join in &config.joins {
        let join_table = sanitize_identifier(&join.table)?;
        let parent_col = sanitize_identifier(&join.parent_column)?;
        let child_col = sanitize_identifier(&join.child_column)?;
        let join_type = join.join_type.as_sql();
        write!(
            &mut sql,
            " {join_type} JOIN {join_table} ON {table}.{parent_col} = {join_table}.{child_col}"
        )
        .expect("writing to SQL buffer should not fail");
    }

    if !config.where_clauses.is_empty() {
        let mut conditions = Vec::new();
        for (field, value) in &config.where_clauses {
            let field = sanitize_identifier(field)?;
            let field_ref = if has_joins {
                format!("{table}.{field}")
            } else {
                field.to_string()
            };
            conditions.push(format!("{field_ref} = ?"));
            params.push(Value::Text(value.clone()));
        }
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }

    if let Some(order_by) = &config.order_by {
        let field = sanitize_identifier(&order_by.field)?;
        let field_ref = if has_joins {
            format!("{table}.{field}")
        } else {
            field.to_string()
        };
        write!(
            &mut sql,
            " ORDER BY {field_ref} {}",
            order_by.direction.as_sql()
        )
        .expect("writing to SQL buffer should not fail");
    }

    if let Some(limit) = config.limit {
        sql.push_str(" LIMIT ?");
        params.push(Value::Integer(limit));
    }

    Ok(BuiltQuery { sql, params })
}
