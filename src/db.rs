use rand::seq::SliceRandom;
use rand::Rng;
use rusqlite::{params, Connection};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to join blocking task")]
    Join,
    #[error("seed data missing: {0}")]
    SeedData(&'static str),
}

pub struct DbInit {
    pub path: PathBuf,
    pub seeded: bool,
    pub connection: Connection,
}

const FIRST_NAMES: &[&str] = &[
    "Ada", "Alan", "Grace", "Linus", "Margaret", "Dennis", "Bjarne", "Guido",
    "Brendan", "Ryan", "James", "Ken", "Brian", "Tim", "Vint", "Donald",
    "Barbara", "Frances", "Jean", "Radia", "Sophie", "Shafi", "Fei-Fei",
    "John", "Steve", "Bill", "Elon", "Jeff", "Mark", "Larry", "Sergey",
    "Satya", "Sundar", "Jensen", "Lisa", "Susan", "Marissa", "Sheryl", "Ginni",
];

const LAST_NAMES: &[&str] = &[
    "Lovelace", "Turing", "Hopper", "Torvalds", "Hamilton", "Ritchie", "Stroustrup",
    "van Rossum", "Eich", "Dahl", "Gosling", "Thompson", "Kernighan", "Berners-Lee",
    "Cerf", "Knuth", "Liskov", "Allen", "Bartik", "Perlman", "Wilson", "Goldwasser",
    "Li", "McCarthy", "Wozniak", "Gates", "Musk", "Bezos", "Zuckerberg", "Page",
    "Brin", "Nadella", "Pichai", "Huang", "Su", "Wojcicki", "Mayer", "Sandberg", "Rometty",
];

const ROLES: &[&str] = &[
    "admin", "developer", "designer", "manager", "analyst", "engineer", "lead", "intern",
];
const STATUSES: &[&str] = &["active", "inactive", "pending", "verified"];
const AVATARS: &[&str] = &[
    "coder", "builder", "hacker", "explorer", "penguin", "snake", "coffee", "diamond", "crab", "bolt",
    "leaf", "rocket", "robot", "chip", "spark",
];

const PRODUCT_ADJECTIVES: &[&str] = &[
    "Premium", "Ultra", "Pro", "Elite", "Essential", "Classic", "Modern", "Smart", "Wireless", "Ergonomic",
];
const PRODUCT_NOUNS: &[&str] = &[
    "Keyboard", "Monitor", "Mouse", "Headphones", "Webcam", "Microphone", "Desk", "Chair", "Lamp", "Hub", "Cable", "Stand", "Dock", "Speaker", "Tablet",
];
const PRODUCT_DESCRIPTIONS: &[&str] = &[
    "High-quality build with premium materials",
    "Perfect for professionals and enthusiasts",
    "Award-winning design and performance",
    "Industry-leading technology",
    "Sleek and modern aesthetic",
    "Built for comfort and productivity",
    "Next-generation features",
    "Eco-friendly and sustainable",
];
const CATEGORIES: &[&str] = &[
    "electronics", "furniture", "accessories", "audio", "lighting", "peripherals", "storage", "networking",
];

const POST_TITLES: &[&str] = &[
    "Why {} is the Future of {}",
    "Getting Started with {}",
    "10 Tips for Better {}",
    "The Complete Guide to {}",
    "How I Built {} with {}",
    "Understanding {} in {}",
    "{} vs {}: Which is Better?",
    "Mastering {} for Beginners",
    "Advanced {} Techniques",
    "The State of {} in 2024",
];
const TECH_TERMS: &[&str] = &[
    "React", "TypeScript", "Rust", "Go", "Python", "JavaScript", "SQL", "GraphQL",
    "Docker", "Kubernetes", "AWS", "Machine Learning", "AI", "Web Development",
    "Cloud Computing", "DevOps", "Microservices", "REST APIs", "WebAssembly", "Edge Computing",
];

/// Initialize or create the `SQLite` database, seeding it when missing.
///
/// # Errors
/// Returns `DbError` if the database cannot be opened, copied, or seeded.
pub fn init_db() -> Result<DbInit, DbError> {
    let (path, copied) = resolve_db_path()?;
    let should_seed = !path.exists() && !copied;

    let mut connection = Connection::open(&path)?;
    let _ = connection.pragma_update(None, "journal_mode", "WAL");

    let seeded = if should_seed {
        seed_database(&mut connection)?;
        true
    } else {
        false
    };

    Ok(DbInit {
        path,
        seeded,
        connection,
    })
}

fn resolve_db_path() -> Result<(PathBuf, bool), DbError> {
    let is_vercel = env::var("VERCEL").ok().as_deref() == Some("1") || env::var("VERCEL_ENV").is_ok();
    let project_db = env::current_dir()?.join("tailwindsql.db");

    if is_vercel {
        let tmp_db = PathBuf::from("/tmp/tailwindsql.db");
        let copied = if !tmp_db.exists() && project_db.exists() {
            copy_db_files(&project_db, &tmp_db)?;
            true
        } else {
            false
        };
        return Ok((tmp_db, copied));
    }

    Ok((project_db, false))
}

fn copy_db_files(src: &Path, dst: &Path) -> Result<(), DbError> {
    fs::copy(src, dst)?;

    let wal_src = src.with_extension("db-wal");
    let wal_dst = dst.with_extension("db-wal");
    if wal_src.exists() {
        let _ = fs::copy(wal_src, wal_dst);
    }

    let shm_src = src.with_extension("db-shm");
    let shm_dst = dst.with_extension("db-shm");
    if shm_src.exists() {
        let _ = fs::copy(shm_src, shm_dst);
    }

    Ok(())
}

/// Seed the demo database with sample users, products, and posts.
///
/// # Errors
/// Returns `DbError` if schema creation or inserts fail.
pub fn seed_database(conn: &mut Connection) -> Result<(), DbError> {
    println!("TailwindSQL Database Seeder");
    println!("================================\n");

    create_schema(conn)?;

    let mut rng = rand::thread_rng();
    seed_users(conn, &mut rng)?;
    seed_products(conn, &mut rng)?;
    seed_posts(conn, &mut rng)?;

    print_summary(conn)?;

    Ok(())
}

fn create_schema(conn: &Connection) -> Result<(), DbError> {
    conn.execute_batch(
        "
        DROP TABLE IF EXISTS posts;
        DROP TABLE IF EXISTS products;
        DROP TABLE IF EXISTS users;

        CREATE TABLE users (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          name TEXT NOT NULL,
          email TEXT UNIQUE NOT NULL,
          role TEXT NOT NULL,
          avatar TEXT,
          status TEXT DEFAULT 'active',
          created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE products (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          title TEXT NOT NULL,
          description TEXT,
          price REAL NOT NULL,
          category TEXT NOT NULL,
          stock INTEGER DEFAULT 0,
          rating REAL DEFAULT 0,
          created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE posts (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          title TEXT NOT NULL,
          content TEXT,
          author_id INTEGER,
          likes INTEGER DEFAULT 0,
          views INTEGER DEFAULT 0,
          published INTEGER DEFAULT 0,
          created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
          FOREIGN KEY (author_id) REFERENCES users(id)
        );
        ",
    )?;

    Ok(())
}

fn seed_users(conn: &mut Connection, rng: &mut impl Rng) -> Result<(), DbError> {
    println!("Seeding 1000 users...");

    let tx = conn.transaction()?;
    let mut stmt = tx.prepare("INSERT INTO users (name, email, role, avatar, status) VALUES (?, ?, ?, ?, ?)")?;
    let mut used_emails = HashSet::new();

    for i in 0..1000 {
        let first = choose_str(FIRST_NAMES, rng, "first_names")?;
        let last = choose_str(LAST_NAMES, rng, "last_names")?;
        let name = format!("{first} {last}");

        let mut email = format!("{}.{}{}@example.com", first.to_lowercase(), last.to_lowercase(), i);
        while used_emails.contains(&email) {
            let suffix: i32 = rng.gen_range(1..=999);
            email = format!(
                "{}.{}{}{}@example.com",
                first.to_lowercase(),
                last.to_lowercase(),
                i,
                suffix
            );
        }
        used_emails.insert(email.clone());

        let role = choose_str(ROLES, rng, "roles")?;
        let avatar = choose_str(AVATARS, rng, "avatars")?;
        let status = choose_str(STATUSES, rng, "statuses")?;

        stmt.execute(params![name, email, role, avatar, status])?;
    }

    drop(stmt);
    tx.commit()?;

    Ok(())
}

fn seed_products(conn: &mut Connection, rng: &mut impl Rng) -> Result<(), DbError> {
    println!("Seeding 1000 products...");

    let tx = conn.transaction()?;
    let mut stmt = tx.prepare(
        "INSERT INTO products (title, description, price, category, stock, rating) VALUES (?, ?, ?, ?, ?, ?)",
    )?;

    for i in 0..1000 {
        let adj = choose_str(PRODUCT_ADJECTIVES, rng, "product_adjectives")?;
        let noun = choose_str(PRODUCT_NOUNS, rng, "product_nouns")?;
        let index = i + 1;
        let title = format!("{adj} {noun} {index}");
        let description = choose_str(PRODUCT_DESCRIPTIONS, rng, "product_descriptions")?;
        let price = random_float(rng, 9.99, 999.99, 2);
        let category = choose_str(CATEGORIES, rng, "categories")?;
        let stock = rng.gen_range(0..=500);
        let rating = random_float(rng, 1.0, 5.0, 1);

        stmt.execute(params![title, description, price, category, stock, rating])?;
    }

    drop(stmt);
    tx.commit()?;

    Ok(())
}

fn seed_posts(conn: &mut Connection, rng: &mut impl Rng) -> Result<(), DbError> {
    println!("Seeding 1000 posts...");

    let tx = conn.transaction()?;
    let mut stmt = tx.prepare(
        "INSERT INTO posts (title, content, author_id, likes, views, published) VALUES (?, ?, ?, ?, ?, ?)",
    )?;

    for _ in 0..1000 {
        let title_template = choose_str(POST_TITLES, rng, "post_titles")?;
        let term1 = choose_str(TECH_TERMS, rng, "tech_terms")?;
        let term2 = choose_str(TECH_TERMS, rng, "tech_terms")?;
        let title = title_template.replacen("{}", term1, 1).replacen("{}", term2, 1);
        let content = format!(
            "This is an in-depth article about {term1} and its applications in modern software development. We'll explore best practices, common pitfalls, and advanced techniques."
        );
        let author_id = rng.gen_range(1..=1000);
        let likes = rng.gen_range(0..=10000);
        let views = likes + rng.gen_range(100..=50000);
        let published = i32::from(rng.gen_bool(0.8));

        stmt.execute(params![title, content, author_id, likes, views, published])?;
    }

    drop(stmt);
    tx.commit()?;

    Ok(())
}

fn print_summary(conn: &Connection) -> Result<(), DbError> {
    let user_count: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))?;
    let product_count: i64 = conn.query_row("SELECT COUNT(*) FROM products", [], |row| row.get(0))?;
    let post_count: i64 = conn.query_row("SELECT COUNT(*) FROM posts", [], |row| row.get(0))?;

    println!("\nDatabase seeded successfully!\n");
    println!("Summary:");
    println!("   - Users: {user_count}");
    println!("   - Products: {product_count}");
    println!("   - Posts: {post_count}");
    println!("\nReady to query with TailwindSQL!\n");

    Ok(())
}

fn choose_str<'a>(items: &'a [&'a str], rng: &mut impl Rng, label: &'static str) -> Result<&'a str, DbError> {
    items.choose(rng).copied().ok_or(DbError::SeedData(label))
}

fn random_float(rng: &mut impl Rng, min: f64, max: f64, decimals: u32) -> f64 {
    let value = rng.gen_range(min..=max);
    let exponent = i32::try_from(decimals).unwrap_or(0);
    let factor = 10_f64.powi(exponent);
    (value * factor).round() / factor
}
