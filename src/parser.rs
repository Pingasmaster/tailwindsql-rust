use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum JoinType {
    Inner,
    Left,
    Right,
}

impl JoinType {
    #[must_use]
    pub const fn as_sql(&self) -> &'static str {
        match self {
            Self::Inner => "INNER",
            Self::Left => "LEFT",
            Self::Right => "RIGHT",
        }
    }
}

#[derive(Debug, Clone)]
pub struct JoinConfig {
    pub table: String,
    pub parent_column: String,
    pub child_column: String,
    pub columns: Vec<String>,
    pub join_type: JoinType,
}

#[derive(Debug, Clone)]
pub struct OrderBy {
    pub field: String,
    pub direction: OrderDirection,
}

#[derive(Debug, Clone, Copy)]
pub enum OrderDirection {
    Asc,
    Desc,
}

impl OrderDirection {
    #[must_use]
    pub const fn as_sql(&self) -> &'static str {
        match self {
            Self::Asc => "ASC",
            Self::Desc => "DESC",
        }
    }
}

#[derive(Debug, Clone)]
pub struct QueryConfig {
    pub table: String,
    pub columns: Vec<String>,
    pub where_clauses: Vec<(String, String)>,
    pub limit: Option<i64>,
    pub order_by: Option<OrderBy>,
    pub joins: Vec<JoinConfig>,
}

#[derive(Debug, Clone, Copy)]
enum ParserState {
    Column,
    WhereField,
    WhereValue,
    Limit,
    OrderByField,
    OrderByDir,
}

#[must_use]
pub fn parse_class_name(class_name: &str) -> Option<QueryConfig> {
    if !class_name.starts_with("db-") {
        return None;
    }

    let parts: Vec<&str> = class_name.trim().strip_prefix("db-")?.split('-').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return None;
    }

    let mut config = QueryConfig {
        table: parts[0].to_string(),
        columns: Vec::new(),
        where_clauses: Vec::new(),
        limit: None,
        order_by: None,
        joins: Vec::new(),
    };

    let mut state = ParserState::Column;
    let mut current_where_field = String::new();
    let mut i = 1;

    while i < parts.len() {
        let part = parts[i];

        if part == "where" {
            state = ParserState::WhereField;
            i += 1;
            continue;
        }

        if part == "limit" {
            state = ParserState::Limit;
            i += 1;
            continue;
        }

        if part == "orderby" {
            state = ParserState::OrderByField;
            i += 1;
            continue;
        }

        match state {
            ParserState::Column => {
                if !matches!(part, "where" | "limit" | "orderby") {
                    config.columns.push(part.to_string());
                }
            }
            ParserState::WhereField => {
                current_where_field = part.to_string();
                state = ParserState::WhereValue;
            }
            ParserState::WhereValue => {
                config
                    .where_clauses
                    .push((current_where_field.clone(), part.to_string()));
                state = ParserState::WhereField;
            }
            ParserState::Limit => {
                if let Ok(limit) = part.parse::<i64>() {
                    config.limit = Some(limit);
                }
                state = ParserState::Column;
            }
            ParserState::OrderByField => {
                config.order_by = Some(OrderBy {
                    field: part.to_string(),
                    direction: OrderDirection::Asc,
                });
                state = ParserState::OrderByDir;
            }
            ParserState::OrderByDir => {
                if let Some(order_by) = config.order_by.as_mut() {
                    if part == "asc" {
                        order_by.direction = OrderDirection::Asc;
                    } else if part == "desc" {
                        order_by.direction = OrderDirection::Desc;
                    }
                }
                state = ParserState::Column;
            }
        }

        i += 1;
    }

    Some(config)
}

#[must_use]
pub fn parse_class_names(class_names: &str) -> Option<QueryConfig> {
    for class_name in class_names.split_whitespace() {
        let trimmed = class_name.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(config) = parse_class_name(trimmed) {
            return Some(config);
        }
    }
    None
}

#[must_use]
pub fn parse_join_param(param: &str) -> Option<JoinConfig> {
    let parts: Vec<&str> = param.split(':').collect();
    if parts.len() < 2 {
        return None;
    }

    let table = parts[0].to_string();
    let on_clause = parts[1];
    let select_cols = parts.get(2).copied().unwrap_or("");
    let join_type = parts.get(3).copied().unwrap_or("left");

    let mut on_parts = on_clause.split('-');
    let parent_column = on_parts.next().unwrap_or("id").to_string();
    let fallback_child = format!("{table}_id");
    let child_column = on_parts.next().unwrap_or(fallback_child.as_str()).to_string();

    let columns = if select_cols.is_empty() {
        Vec::new()
    } else {
        select_cols
            .split(',')
            .map(str::trim)
            .filter(|c| !c.is_empty())
            .map(ToString::to_string)
            .collect()
    };

    let join_type = match join_type {
        "inner" => JoinType::Inner,
        "right" => JoinType::Right,
        _ => JoinType::Left,
    };

    Some(JoinConfig {
        table,
        parent_column,
        child_column,
        columns,
        join_type,
    })
}

#[must_use]
pub fn join_config_from_parts(
    table: &str,
    on: &str,
    select: Option<&str>,
    join_type: Option<&str>,
) -> JoinConfig {
    let mut on_parts = on.split('-');
    let parent_column = on_parts.next().unwrap_or("id").to_string();
    let fallback_child = format!("{table}_id");
    let child_column = on_parts.next().unwrap_or(fallback_child.as_str()).to_string();

    let columns = select
        .unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    let join_type = match join_type.unwrap_or("left") {
        "inner" => JoinType::Inner,
        "right" => JoinType::Right,
        _ => JoinType::Left,
    };

    JoinConfig {
        table: table.to_string(),
        parent_column,
        child_column,
        columns,
        join_type,
    }
}

#[must_use]
pub fn config_with_join(mut config: QueryConfig, join: JoinConfig) -> QueryConfig {
    config.joins.push(join);
    config
}

#[must_use]
pub fn where_as_map(config: &QueryConfig) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for (field, value) in &config.where_clauses {
        map.insert(field.clone(), value.clone());
    }
    map
}
