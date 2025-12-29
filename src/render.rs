use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt::{self, Write};

pub type RowData = BTreeMap<String, Value>;

#[derive(Debug, Clone, Copy)]
pub enum RenderAs {
    Span,
    Div,
    Ul,
    Ol,
    Table,
    Json,
    Code,
}

impl RenderAs {
    #[must_use]
    pub fn parse(value: &str) -> Self {
        match value {
            "div" => Self::Div,
            "ul" => Self::Ul,
            "ol" => Self::Ol,
            "table" => Self::Table,
            "json" => Self::Json,
            "code" => Self::Code,
            _ => Self::Span,
        }
    }
}

#[must_use]
pub fn render_results(results: &[RowData], columns: &[String], render_as: RenderAs) -> String {
    if results.is_empty() {
        return "<span class=\"text-gray-400 italic\">No results</span>".to_string();
    }

    let mut display_columns = if columns.is_empty() {
        columns_from_results(results)
    } else {
        columns.to_vec()
    };

    if display_columns.is_empty() {
        display_columns = columns_from_results(results);
    }

    if display_columns.len() == 1 && results.len() == 1 {
        return render_single_value(results, &display_columns[0]);
    }

    if display_columns.len() == 1 {
        return render_single_column(results, &display_columns[0], render_as);
    }

    match render_as {
        RenderAs::Table => render_table(results, &display_columns),
        RenderAs::Json | RenderAs::Code => render_json_block(results),
        RenderAs::Ul => render_row_list(results, "ul", "list-disc list-inside"),
        RenderAs::Ol => render_row_list(results, "ol", "list-decimal list-inside"),
        _ => render_default_rows(results, &display_columns),
    }
}

fn columns_from_results(results: &[RowData]) -> Vec<String> {
    results
        .first()
        .map(|row| row.keys().cloned().collect())
        .unwrap_or_default()
}

fn render_single_value(results: &[RowData], column: &str) -> String {
    let value = results[0].get(column);
    format!("<span>{}</span>", format_value(value))
}

fn render_single_column(results: &[RowData], column: &str, render_as: RenderAs) -> String {
    let values = results
        .iter()
        .map(|row| format_value(row.get(column)))
        .collect::<Vec<_>>();

    match render_as {
        RenderAs::Ul => render_list("ul", "list-disc list-inside", values.iter()),
        RenderAs::Ol => render_list("ol", "list-decimal list-inside", values.iter()),
        RenderAs::Json | RenderAs::Code => {
            let json = serde_json::to_string_pretty(&values).unwrap_or_default();
            format!(
                "<code class=\"font-mono text-xs sm:text-sm bg-black/40 text-green-400 p-2 sm:p-3 rounded block overflow-x-auto\">{}</code>",
                escape_html(&json)
            )
        }
        _ => format!("<span>{}</span>", values.join(", ")),
    }
}

fn render_table(results: &[RowData], columns: &[String]) -> String {
    let headers = if columns.is_empty() {
        columns_from_results(results)
    } else {
        columns.to_vec()
    };

    let mut html = String::new();
    push_html(
        &mut html,
        format_args!(
            "<div class=\"overflow-x-auto -mx-2 sm:mx-0\"><table class=\"border-collapse border border-white/10 text-xs sm:text-sm w-full min-w-[400px]\"><thead><tr class=\"bg-white/5\">"
        ),
    );

    for header in &headers {
        let escaped = escape_html(header);
        push_html(
            &mut html,
            format_args!(
                "<th class=\"border border-white/10 px-2 sm:px-3 py-1.5 sm:py-2 text-left font-semibold text-cyan-400 whitespace-nowrap\">{escaped}</th>"
            ),
        );
    }

    push_html(&mut html, format_args!("</tr></thead><tbody>"));

    for row in results {
        push_html(&mut html, format_args!("<tr class=\"hover:bg-white/5 transition-colors\">"));
        for header in &headers {
            let value = format_value(row.get(header));
            push_html(
                &mut html,
                format_args!(
                    "<td class=\"border border-white/10 px-2 sm:px-3 py-1.5 sm:py-2 text-slate-300 break-words max-w-[150px] sm:max-w-none\">{value}</td>"
                ),
            );
        }
        push_html(&mut html, format_args!("</tr>"));
    }

    push_html(&mut html, format_args!("</tbody></table></div>"));
    html
}

fn render_json_block(results: &[RowData]) -> String {
    let json = serde_json::to_string_pretty(results).unwrap_or_default();
    format!(
        "<code class=\"font-mono text-xs sm:text-sm bg-black/40 text-green-400 p-2 sm:p-3 rounded block whitespace-pre overflow-x-auto\">{}</code>",
        escape_html(&json)
    )
}

fn render_row_list(results: &[RowData], tag: &str, class_name: &str) -> String {
    let mut items = Vec::with_capacity(results.len());
    for row in results {
        let json = serde_json::to_string(row).unwrap_or_default();
        items.push(escape_html(&json));
    }
    render_list(tag, class_name, items.iter())
}

fn render_list<'a>(tag: &str, class_name: &str, items: impl Iterator<Item = &'a String>) -> String {
    let mut html = String::new();
    push_html(
        &mut html,
        format_args!("<{tag} class=\"{class_name}\">"),
    );

    for item in items {
        push_html(&mut html, format_args!("<li>{item}</li>"));
    }

    push_html(&mut html, format_args!("</{tag}>"));
    html
}

fn render_default_rows(results: &[RowData], columns: &[String]) -> String {
    let headers = if columns.is_empty() {
        columns_from_results(results)
    } else {
        columns.to_vec()
    };

    let mut html = String::new();
    for row in results {
        let mut line = String::new();
        for (idx, header) in headers.iter().enumerate() {
            if idx > 0 {
                line.push_str(", ");
            }
            line.push_str(&format_value(row.get(header)));
        }
        push_html(&mut html, format_args!("<div>{line}</div>"));
    }

    format!("<div>{html}</div>")
}

fn format_value(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::Bool(v)) => v.to_string(),
        Some(Value::Number(num)) => num.to_string(),
        Some(Value::String(s)) => escape_html(s),
        Some(other) => escape_html(&other.to_string()),
    }
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

fn push_html(buf: &mut String, args: fmt::Arguments<'_>) {
    buf.write_fmt(args)
        .expect("writing to String should not fail");
}
