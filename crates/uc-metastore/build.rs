//! Rebuild when a migration is added: `embed_migrations!` reads the
//! `migrations` folder at compile time, and only the files it embedded are
//! tracked.
fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
