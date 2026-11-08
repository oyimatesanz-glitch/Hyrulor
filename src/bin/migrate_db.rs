//! Migra una base de datos SQLite en texto plano a SQLCipher.
//!
//! Uso:
//!   migrate_db [origen] [destino]
//!
//! Por defecto: `hyory.db` → `hyory.db.enc`.
//!
//! La clave se toma de `HYORY_DB_KEY` o del archivo de clave habitual. Si el
//! destino es nuevo y no hay clave, se genera una con permisos `0600`.
//! El destino no debe existir: la migración nunca sobrescribe datos.

#[path = "../db_key.rs"]
mod db_key;

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OpenFlags};

fn main() {
    if let Err(error) = run() {
        eprintln!("❌ Migración fallida: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    // sqlite-vec debe estar registrado para poder copiar y verificar docs_vec.
    unsafe {
        rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(
            sqlite_vec::sqlite3_vec_init as *const (),
        )));
    }

    let mut args = std::env::args().skip(1);
    let first_arg = args.next();

    // Modo de solo lectura para comprobar una base ya cifrada:
    //   migrate_db --verify [ruta]
    if first_arg.as_deref() == Some("--verify") {
        let path = PathBuf::from(args.next().unwrap_or_else(|| "hyory.db".to_owned()));
        return verify(&path);
    }

    let source = PathBuf::from(first_arg.unwrap_or_else(|| "hyory.db".to_owned()));
    let target = PathBuf::from(args.next().unwrap_or_else(|| "hyory.db.enc".to_owned()));

    if !source.is_file() {
        return Err(format!("no existe la base de datos de origen: {}", source.display()).into());
    }
    if source == target {
        return Err("el origen y el destino deben ser archivos distintos".into());
    }
    if target.exists() {
        return Err(format!(
            "el destino {} ya existe; bórralo o indica otra ruta",
            target.display()
        )
        .into());
    }

    // ATTACH hereda los flags de la conexión principal (incluido CREATE): el
    // destino debe poder crearse. El origen ya existe, así que CREATE es inocuo.
    let source_conn = Connection::open(&source)?;
    let integrity: String =
        source_conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err(format!("integrity_check del origen devolvió: {integrity}").into());
    }
    let before = counts(&source_conn)?;

    let key = resolve_key(&target)?;

    // sqlcipher_export copia esquema, datos, triggers y tablas virtuales. Es
    // preferible a recrear el esquema a mano porque evita cualquier deriva.
    let export_result = (|| -> Result<(), rusqlite::Error> {
        source_conn.execute(
            "ATTACH DATABASE ?1 AS encrypted KEY ?2",
            params![target.to_string_lossy().as_ref(), format!("x'{key}'")],
        )?;
        source_conn.execute_batch("SELECT sqlcipher_export('encrypted');")?;
        source_conn.execute_batch("DETACH DATABASE encrypted;")
    })();

    if let Err(error) = export_result {
        drop(source_conn);
        let _ = std::fs::remove_file(&target);
        return Err(
            format!("sqlcipher_export falló ({error}); no se ha modificado el origen").into(),
        );
    }
    drop(source_conn);

    // Verificación independiente: abrir el destino con la clave y comprobar
    // integridad, recuentos y acceso a las tablas virtuales.
    let target_conn = Connection::open(&target)?;
    db_key::apply(&target_conn, &key)?;

    let integrity: String =
        target_conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err(format!("integrity_check del destino devolvió: {integrity}").into());
    }

    // Garantiza que el índice FTS5 externo coincide con `convs` aunque el
    // export haya copiado las shadow tables tal cual.
    target_conn.execute_batch("INSERT INTO conv_fts(conv_fts) VALUES('rebuild');")?;
    target_conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;

    let after = counts(&target_conn)?;
    if before != after {
        return Err(format!(
            "los recuentos no coinciden: origen {before:?}, destino {after:?}. \
             El destino se conserva en {} para inspección",
            target.display()
        )
        .into());
    }

    // Comprueba que el destino es realmente ilegible sin clave.
    drop(target_conn);
    if plaintext_is_readable(&target) {
        return Err(format!(
            "el destino {} sigue siendo legible sin clave; migración abortada",
            target.display()
        )
        .into());
    }

    // La base contiene datos sensibles aunque esté cifrada: 0600.
    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o600))?;

    let key_path = db_key::key_path();
    println!("✅ Migración completada");
    println!("   Origen:       {}", source.display());
    println!("   Destino:      {} (SQLCipher AES-256)", target.display());
    println!("   Clave:        {} (0600)", key_path.display());
    println!(
        "   Datos:        {} conversaciones, {} documentos, {} vectores",
        after.0, after.1, after.2
    );
    println!();
    println!("Siguiente paso (detén el daemon antes):");
    println!("   mv {} {}.old", source.display(), source.display());
    println!("   mv {} {}", target.display(), source.display());
    println!("   ./hyrulor.sh start");
    Ok(())
}

fn verify(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !path.is_file() {
        return Err(format!("no existe la base de datos: {}", path.display()).into());
    }

    let key = db_key::resolve_for_database(path)?;
    let connection = Connection::open(path)?;
    db_key::apply(&connection, &key)?;

    let cipher_version: String = connection
        .query_row("PRAGMA cipher_version", [], |row| row.get(0))
        .unwrap_or_else(|_| "desconocida".to_owned());
    let integrity: String = connection
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .unwrap_or_else(|_| "desconocida".to_owned());

    let (convs, docs, vectors) = counts(&connection)?;
    let fts_entries: i64 =
        connection.query_row("SELECT count(*) FROM conv_fts", [], |row| row.get(0))?;
    let vector_probes: i64 = {
        let embedding_json = format!(
            "[{}]",
            (0..1024).map(|_| "0.0").collect::<Vec<_>>().join(",")
        );
        connection.query_row(
            "SELECT count(*) FROM docs_vec AS v
             WHERE v.embedding MATCH vec_f32(?1) AND v.k = 3",
            params![embedding_json],
            |row| row.get(0),
        )?
    };

    if plaintext_is_readable(path) {
        return Err(format!("la base {} sigue legible sin clave", path.display()).into());
    }

    println!("✅ Base verificada");
    println!("   SQLCipher: {cipher_version}");
    println!("   Integridad: {integrity}");
    println!("   Recuentos: {convs} conversaciones, {docs} documentos, {vectors} vectores");
    println!("   FTS consultable: {fts_entries}");
    println!("   Búsqueda vectorial: {vector_probes}");
    println!("   Texto plano: bloqueado");
    Ok(())
}

/// Resuelve la clave que usará el daemon y la persiste si vino del entorno.
fn resolve_key(target: &Path) -> Result<String, Box<dyn std::error::Error>> {
    if let Some(value) = std::env::var_os(db_key::KEY_ENV) {
        let key = db_key::parse_key(&value.to_string_lossy())
            .ok_or("HYORY_DB_KEY inválida: se esperan exactamente 64 caracteres hexadecimales")?;
        db_key::ensure_key_file(&db_key::key_path(), &key)?;
        return Ok(key);
    }

    Ok(db_key::resolve_for_database(target)?)
}

fn counts(connection: &Connection) -> Result<(i64, i64, i64), rusqlite::Error> {
    let convs = connection.query_row("SELECT count(*) FROM convs", [], |row| row.get(0))?;
    let docs = connection.query_row("SELECT count(*) FROM docs", [], |row| row.get(0))?;
    let vectors = connection.query_row("SELECT count(*) FROM docs_vec", [], |row| row.get(0))?;
    Ok((convs, docs, vectors))
}

/// Abre el archivo con SQLite normal (sin clave) para confirmar que SQLCipher
/// ya no permite leerlo como base en texto plano.
fn plaintext_is_readable(path: &Path) -> bool {
    match Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY) {
        Ok(connection) => connection
            .query_row("SELECT count(*) FROM sqlite_master", [], |row| {
                row.get::<_, i64>(0)
            })
            .is_ok(),
        Err(_) => false,
    }
}
