# TailwindSQL

> Like TailwindCSS, but for SQL queries - now rewritten in Rust.

## What is this?

TailwindSQL lets you write SQL queries using Tailwind-style class names. It ships as a Rust web app with a live playground, examples, and a database explorer.

```html
<!-- Fetch and render a user's name -->
<DB className="db-users-name-where-id-1" />
<!-- Renders: "Ada Lovelace" -->

<!-- Render products as a list -->
<DB className="db-products-title-limit-5" as="ul" />

<!-- Order by price and show as table -->
<DB className="db-products-orderby-price-desc" as="table" />
```

## Features

- Tailwind-style query syntax
- Rust + Axum server
- SQLite (via rusqlite)
- Interactive playground with live results
- Multiple render modes (text, list, table, JSON)
- Database explorer UI

## Syntax

```
db-{table}-{column}-where-{field}-{value}-limit-{n}-orderby-{field}-{asc|desc}
```

### Examples

| Class Name | SQL Query |
|------------|-----------|
| `db-users` | `SELECT * FROM users` |
| `db-users-name` | `SELECT name FROM users` |
| `db-users-where-id-1` | `SELECT * FROM users WHERE id = 1` |
| `db-posts-title-limit-10` | `SELECT title FROM posts LIMIT 10` |
| `db-products-orderby-price-desc` | `SELECT * FROM products ORDER BY price DESC` |

## Getting Started

### Prerequisites

- Rust (stable toolchain)

### Run locally

```bash
# Seed the database with demo data
cargo run --bin seed

# Start the server
cargo run
```

Open http://localhost:3000 for the playground and examples.
Open http://localhost:3000/explorer for the database explorer.

## How It Works

1. Parser (`src/parser.rs`) - Parses Tailwind-style class names into query configs
2. Query Builder (`src/query_builder.rs`) - Builds parameterized SQL safely
3. Web UI (`templates/` + `static/`) - Renders the landing page and playground
4. API (`/api/query`, `/api/schema`) - Powers the playground and explorer

## Project Structure

```
tailwindsql/
- src/
  - main.rs          # Axum server
  - parser.rs        # Class name parser
  - query_builder.rs # SQL query builder
  - db.rs            # SQLite setup + seeding
  - render.rs        # HTML rendering helpers
- static/            # CSS + JS assets
- templates/         # HTML templates
- README.md
```

## Why?

This project was built to explore CSS-driven database queries - now with a Rust backend.

## License

MIT
