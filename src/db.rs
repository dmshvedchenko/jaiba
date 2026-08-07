use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use chrono::Utc;
use keepass::db::{EntryId, EntryMut, EntryRef, Times, fields};
use keepass::{Database, DatabaseKey};

fn resolve_totp(e: &EntryRef) -> String {
    if let Some(otp) = e.get_raw_otp_value() {
        if !otp.trim().is_empty() {
            return otp.to_string();
        }
    }

    let Some(seed) = e.get("TOTP Seed") else {
        return String::new();
    };
    let seed = seed.trim();
    if seed.is_empty() {
        return String::new();
    }

    let (period, digits) = e
        .get("TOTP Settings")
        .and_then(|settings| settings.split_once(';'))
        .and_then(|(p, d)| Some((p.trim().parse::<u32>().ok()?, d.trim().parse::<u32>().ok()?)))
        .unwrap_or((30, 6));

    let label = e.get_title().unwrap_or("");
    let user = e.get_username().unwrap_or("");
    let encoded_label = urlencoding_encode(&format!("{label}:{user}"));
    let encoded_issuer = urlencoding_encode(label);

    format!(
        "otpauth://totp/{encoded_label}?secret={seed}&period={period}&digits={digits}&issuer={encoded_issuer}"
    )
}

pub struct TotpCode {
    pub code: String,
    pub valid_for: std::time::Duration,
}

pub fn current_totp_code(raw: &str) -> Option<TotpCode> {
    if raw.trim().is_empty() {
        return None;
    }

    let totp: keepass::db::TOTP = raw.parse().ok()?;
    let otp_code = totp.value_now().ok()?;

    Some(TotpCode {
        code: otp_code.code,
        valid_for: otp_code.valid_for,
    })
}

fn urlencoding_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[derive(Clone, Default)]
pub struct Entry {
    pub id: Option<EntryId>,
    pub name: String,
    pub user: String,
    pub password: String,
    pub url: String,
    pub totp: String,
    pub date_last_modify: String,
    pub password_reuse_count: u32,
    pub duplicate_user_count: u32,
}

fn format_days_ago(dt: chrono::NaiveDateTime) -> String {
    let days = (Utc::now().naive_utc() - dt).num_days();

    match days {
        d if d <= 0 => "today".to_string(),
        1 => "1 day ago".to_string(),
        d => format!("{d} days ago"),
    }
}

pub fn unlock_database(
    path: &Path,
    password: &str,
) -> anyhow::Result<(Database, DatabaseKey, Vec<Entry>)> {
    let mut file =
        fs::File::open(path).with_context(|| format!("couldn't open {}", path.display()))?;

    let key = DatabaseKey::new().with_password(password);
    let db = Database::open(&mut file, key.clone()).context("failed to unlock database")?;

    let entries = db
        .iter_all_entries()
        .map(|e| {
            let date_last_modify = e
                .times
                .last_modification
                .map(format_days_ago)
                .unwrap_or_default();

            Entry {
                id: Some(e.id()),
                name: e.get_title().unwrap_or("(no title)").to_string(),
                user: e.get_username().unwrap_or_default().to_string(),
                password: e.get_password().unwrap_or_default().to_string(),
                url: e.get_url().unwrap_or_default().to_string(),
                totp: resolve_totp(&e),
                date_last_modify,
                password_reuse_count: 0,
                duplicate_user_count: 0,
            }
        })
        .collect();

    Ok((db, key, entries))
}

pub fn create_database(
    path: &Path,
    password: &str,
) -> anyhow::Result<(Database, DatabaseKey, Vec<Entry>)> {
    if path.exists() {
        anyhow::bail!("a file already exists at {}", path.display());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("couldn't create {}", parent.display()))?;
    }

    let db = Database::new();
    let key = DatabaseKey::new().with_password(password);

    write_to_disk(path, &key, &db)?;

    Ok((db, key, Vec::new()))
}

pub fn save_database(
    path: &Path,
    key: &DatabaseKey,
    db: &mut Database,
    entries: &mut [Entry],
) -> anyhow::Result<()> {
    for entry in entries.iter_mut() {
        write_entry(db, entry)?;
    }

    write_to_disk(path, key, db)
}

fn write_to_disk(path: &Path, key: &DatabaseKey, db: &Database) -> anyhow::Result<()> {
    let tmp_path = sibling_tmp_path(path);

    {
        let mut file = fs::File::create(&tmp_path)
            .with_context(|| format!("couldn't create {}", tmp_path.display()))?;

        db.save(&mut file, key.clone())
            .map_err(|err| anyhow::anyhow!("failed to write database: {err}"))?;
    }

    fs::rename(&tmp_path, path).with_context(|| format!("couldn't replace {}", path.display()))?;

    Ok(())
}

fn write_entry(db: &mut Database, entry: &mut Entry) -> anyhow::Result<()> {
    match entry.id {
        Some(id) => {
            let mut e = db
                .entry_mut(id)
                .context("entry no longer exists in the database")?;
            apply_fields(&mut e, entry);
        }
        None => {
            let mut root = db.root_mut();
            let mut e = root.add_entry();
            apply_fields(&mut e, entry);
            entry.id = Some(e.id());
        }
    }

    Ok(())
}

pub fn delete_entry(
    path: &Path,
    key: &DatabaseKey,
    db: &mut Database,
    id: EntryId,
) -> anyhow::Result<()> {
    db.entry_mut(id)
        .context("entry no longer exists in the database")?
        .remove();

    write_to_disk(path, key, db)
}

fn apply_fields(e: &mut EntryMut<'_>, entry: &Entry) {
    e.set_unprotected(fields::TITLE, entry.name.clone());
    e.set_unprotected(fields::USERNAME, entry.user.clone());
    e.set_protected(fields::PASSWORD, entry.password.clone());
    e.set_unprotected(fields::URL, entry.url.clone());
    e.set_unprotected(fields::OTP, entry.totp.clone());
    e.times.last_modification = Some(Times::now());
}

fn sibling_tmp_path(path: &Path) -> PathBuf {
    let mut name: OsString = path.as_os_str().to_owned();
    name.push(".tmp");
    PathBuf::from(name)
}

pub fn calculate_warnings(entries: &mut Vec<Entry>) {
    let mut password_counts: HashMap<String, u32> = HashMap::new();
    let mut user_counts: HashMap<String, u32> = HashMap::new();

    for entry in entries.iter() {
        if !entry.password.trim().is_empty() {
            *password_counts.entry(entry.password.clone()).or_insert(0) += 1;
        }

        if !entry.user.trim().is_empty() {
            *user_counts.entry(entry.user.clone()).or_insert(0) += 1;
        }
    }

    for entry in entries.iter_mut() {
        entry.password_reuse_count = if entry.password.trim().is_empty() {
            0
        } else {
            password_counts.get(&entry.password).copied().unwrap_or(0)
        };

        entry.duplicate_user_count = if entry.user.trim().is_empty() {
            0
        } else {
            user_counts.get(&entry.user).copied().unwrap_or(0)
        };
    }
}
