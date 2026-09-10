use anyhow::Result;
use banshee_domain::{Asset, AssetFolder, CandidateClip, EditDecisionList, Project, UsageEvent};
use directories::ProjectDirs;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

const SERVICE: &str = "Banshee Video Editor";

pub struct Store {
    connection: Mutex<Connection>,
    root: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub autosave_seconds: u32,
    pub model: String,
    pub admin_cost_sync: bool,
    pub prompt: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            autosave_seconds: 15,
            model: "gpt-5.6-terra".into(),
            admin_cost_sync: false,
            prompt: banshee_domain::DEFAULT_HIGHLIGHTS_PROMPT.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStats {
    pub spend_today: f64,
    pub spend_points: Vec<SpendPoint>,
    pub videos_today: u32,
    pub videos_yesterday: u32,
    pub videos_week: u32,
    pub videos_month: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpendPoint {
    pub label: String,
    pub value: f64,
}

impl Store {
    pub fn open_default() -> Result<Self> {
        let root = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                ProjectDirs::from("com", "Banshee", "Banshee Video Editor")
                    .expect("LocalAppData")
                    .data_local_dir()
                    .to_path_buf()
            })
            .join("Banshee Video Editor");
        Self::open(root)
    }

    pub fn open(root: PathBuf) -> Result<Self> {
        fs::create_dir_all(root.join("projects"))?;
        fs::create_dir_all(root.join("library"))?;
        fs::create_dir_all(root.join("cache"))?;
        let connection = Connection::open(root.join("banshee.sqlite3"))?;
        connection.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA foreign_keys=ON;
             CREATE TABLE IF NOT EXISTS projects (id TEXT PRIMARY KEY, data TEXT NOT NULL, updated_at TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS candidates (id TEXT PRIMARY KEY, project_id TEXT NOT NULL, data TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS edits (candidate_id TEXT PRIMARY KEY, project_id TEXT NOT NULL, data TEXT NOT NULL, saved_at TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS assets (id TEXT PRIMARY KEY, data TEXT NOT NULL, created_at TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS asset_folders (id TEXT PRIMARY KEY, data TEXT NOT NULL, created_at TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS usage_events (id TEXT PRIMARY KEY, data TEXT NOT NULL, created_at TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS render_events (id INTEGER PRIMARY KEY AUTOINCREMENT, project_id TEXT NOT NULL, created_at TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS settings (id INTEGER PRIMARY KEY CHECK (id = 1), data TEXT NOT NULL);"
        )?;
        Ok(Self {
            connection: Mutex::new(connection),
            root,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn project_dir(&self, id: &str) -> Result<PathBuf> {
        let path = self.root.join("projects").join(id);
        fs::create_dir_all(path.join("thumbnails"))?;
        fs::create_dir_all(path.join("exports"))?;
        fs::create_dir_all(path.join("temp"))?;
        Ok(path)
    }

    pub fn library_dir(&self) -> Result<PathBuf> {
        let path = self.root.join("library");
        fs::create_dir_all(path.join("thumbnails"))?;
        Ok(path)
    }

    pub fn save_project(&self, project: &Project) -> Result<()> {
        let data = serde_json::to_string(project)?;
        self.connection.lock().unwrap().execute(
            "INSERT INTO projects(id,data,updated_at) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET data=excluded.data, updated_at=excluded.updated_at",
            params![project.id, data, project.updated_at.to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn delete_project(&self, id: &str) -> Result<()> {
        let mut connection = self.connection.lock().unwrap();
        let transaction = connection.transaction()?;
        transaction.execute("DELETE FROM edits WHERE project_id=?1", [id])?;
        transaction.execute("DELETE FROM candidates WHERE project_id=?1", [id])?;
        transaction.execute("DELETE FROM usage_events WHERE json_extract(data,'$.projectId')=?1", [id])?;
        transaction.execute("DELETE FROM render_events WHERE project_id=?1", [id])?;
        transaction.execute("DELETE FROM projects WHERE id=?1", [id])?;
        transaction.commit()?;
        let project_dir = self.root.join("projects").join(id);
        if project_dir.is_dir() {
            fs::remove_dir_all(project_dir)?;
        }
        Ok(())
    }

    pub fn projects(&self) -> Result<Vec<Project>> {
        let connection = self.connection.lock().unwrap();
        let mut statement =
            connection.prepare("SELECT data FROM projects ORDER BY updated_at DESC")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        rows.map(|row| serde_json::from_str(&row?).map_err(Into::into))
            .collect()
    }

    pub fn project(&self, id: &str) -> Result<Project> {
        let data: String = self.connection.lock().unwrap().query_row(
            "SELECT data FROM projects WHERE id=?1",
            [id],
            |row| row.get(0),
        )?;
        Ok(serde_json::from_str(&data)?)
    }

    pub fn save_candidates(&self, project_id: &str, clips: &[CandidateClip]) -> Result<()> {
        let mut connection = self.connection.lock().unwrap();
        let transaction = connection.transaction()?;
        transaction.execute("DELETE FROM candidates WHERE project_id=?1", [project_id])?;
        for clip in clips {
            transaction.execute(
                "INSERT INTO candidates(id,project_id,data) VALUES(?1,?2,?3)",
                params![clip.id, project_id, serde_json::to_string(clip)?],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn candidates(&self, project_id: &str) -> Result<Vec<CandidateClip>> {
        let connection = self.connection.lock().unwrap();
        let mut statement = connection.prepare("SELECT data FROM candidates WHERE project_id=?1 ORDER BY json_extract(data,'$.score') DESC")?;
        let rows = statement.query_map([project_id], |row| row.get::<_, String>(0))?;
        rows.map(|row| serde_json::from_str(&row?).map_err(Into::into))
            .collect()
    }

    pub fn save_candidate(&self, candidate: &CandidateClip) -> Result<()> {
        self.connection.lock().unwrap().execute(
            "UPDATE candidates SET data=?2 WHERE id=?1",
            params![candidate.id, serde_json::to_string(candidate)?],
        )?;
        Ok(())
    }

    pub fn candidate(&self, id: &str) -> Result<CandidateClip> {
        let data: String = self.connection.lock().unwrap().query_row(
            "SELECT data FROM candidates WHERE id=?1",
            [id],
            |row| row.get(0),
        )?;
        Ok(serde_json::from_str(&data)?)
    }

    pub fn delete_candidate(&self, id: &str) -> Result<()> {
        let connection = self.connection.lock().unwrap();
        let candidate: Option<CandidateClip> = connection
            .query_row("SELECT data FROM candidates WHERE id=?1", [id], |row| row.get::<_, String>(0))
            .optional()?
            .map(|data| serde_json::from_str(&data))
            .transpose()?;
        let project_id: Option<String> = connection
            .query_row("SELECT project_id FROM candidates WHERE id=?1", [id], |row| row.get(0))
            .optional()?;
        drop(connection);
        let mut connection = self.connection.lock().unwrap();
        let transaction = connection.transaction()?;
        transaction.execute("DELETE FROM edits WHERE candidate_id=?1", [id])?;
        transaction.execute("DELETE FROM candidates WHERE id=?1", [id])?;
        transaction.commit()?;
        if let (Some(project_id), Some(candidate)) = (project_id, candidate) {
            if let Some(path) = candidate.thumbnail_path {
                let safe_root = self.root.join("projects").join(project_id);
                let thumbnail = PathBuf::from(path);
                if thumbnail.starts_with(&safe_root) && thumbnail.is_file() {
                    let _ = fs::remove_file(thumbnail);
                }
            }
        }
        Ok(())
    }

    pub fn save_edit(&self, edit: &EditDecisionList) -> Result<()> {
        self.connection.lock().unwrap().execute(
            "INSERT INTO edits(candidate_id,project_id,data,saved_at) VALUES(?1,?2,?3,datetime('now')) ON CONFLICT(candidate_id) DO UPDATE SET data=excluded.data,saved_at=excluded.saved_at",
            params![edit.candidate_id, edit.project_id, serde_json::to_string(edit)?],
        )?;
        Ok(())
    }

    pub fn edit(&self, candidate_id: &str) -> Result<Option<EditDecisionList>> {
        let connection = self.connection.lock().unwrap();
        let mut statement = connection.prepare("SELECT data FROM edits WHERE candidate_id=?1")?;
        let mut rows = statement.query([candidate_id])?;
        Ok(match rows.next()? {
            Some(row) => Some(serde_json::from_str(&row.get::<_, String>(0)?)?),
            None => None,
        })
    }

    pub fn add_asset(&self, asset: &Asset) -> Result<()> {
        self.connection.lock().unwrap().execute(
            "INSERT OR REPLACE INTO assets(id,data,created_at) VALUES(?1,?2,?3)",
            params![
                asset.id,
                serde_json::to_string(asset)?,
                asset.created_at.to_rfc3339()
            ],
        )?;
        Ok(())
    }

    pub fn asset(&self, id: &str) -> Result<Asset> {
        let data: String = self.connection.lock().unwrap().query_row(
            "SELECT data FROM assets WHERE id=?1", [id], |row| row.get(0),
        )?;
        Ok(serde_json::from_str(&data)?)
    }

    pub fn delete_asset(&self, id: &str) -> Result<()> {
        let asset = self.asset(id)?;
        self.connection.lock().unwrap().execute("DELETE FROM assets WHERE id=?1", [id])?;
        if let Some(path) = asset.thumbnail_path {
            let thumbnail = PathBuf::from(path);
            let safe_root = self.root.join("library");
            if thumbnail.starts_with(&safe_root) && thumbnail.is_file() {
                let _ = fs::remove_file(thumbnail);
            }
        }
        Ok(())
    }

    pub fn assets(&self) -> Result<Vec<Asset>> {
        let connection = self.connection.lock().unwrap();
        let mut statement =
            connection.prepare("SELECT data FROM assets ORDER BY created_at DESC")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        rows.map(|row| serde_json::from_str(&row?).map_err(Into::into))
            .collect()
    }

    pub fn add_asset_folder(&self, folder: &AssetFolder) -> Result<()> {
        self.connection.lock().unwrap().execute(
            "INSERT INTO asset_folders(id,data,created_at) VALUES(?1,?2,?3)",
            params![
                folder.id,
                serde_json::to_string(folder)?,
                folder.created_at.to_rfc3339()
            ],
        )?;
        Ok(())
    }

    pub fn asset_folders(&self) -> Result<Vec<AssetFolder>> {
        let connection = self.connection.lock().unwrap();
        let mut statement =
            connection.prepare("SELECT data FROM asset_folders ORDER BY created_at")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        rows.map(|row| serde_json::from_str(&row?).map_err(Into::into))
            .collect()
    }

    pub fn add_usage(&self, event: &UsageEvent) -> Result<()> {
        self.connection.lock().unwrap().execute(
            "INSERT INTO usage_events(id,data,created_at) VALUES(?1,?2,?3)",
            params![
                event.id,
                serde_json::to_string(event)?,
                event.created_at.to_rfc3339()
            ],
        )?;
        Ok(())
    }

    pub fn record_render(&self, project_id: &str) -> Result<()> {
        self.connection.lock().unwrap().execute(
            "INSERT INTO render_events(project_id,created_at) VALUES(?1,datetime('now'))",
            [project_id],
        )?;
        Ok(())
    }

    pub fn settings(&self) -> Result<AppSettings> {
        let connection = self.connection.lock().unwrap();
        let mut statement = connection.prepare("SELECT data FROM settings WHERE id=1")?;
        let mut rows = statement.query([])?;
        Ok(match rows.next()? {
            Some(row) => serde_json::from_str(&row.get::<_, String>(0)?)?,
            None => AppSettings::default(),
        })
    }

    pub fn save_settings(&self, settings: &AppSettings) -> Result<()> {
        self.connection.lock().unwrap().execute(
            "INSERT INTO settings(id,data) VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET data=excluded.data",
            [serde_json::to_string(settings)?],
        )?;
        Ok(())
    }

    pub fn dashboard_stats(&self) -> Result<DashboardStats> {
        let connection = self.connection.lock().unwrap();
        let mut points = Vec::new();
        for offset in (0..7).rev() {
            let modifier = format!("-{} days", offset);
            let label: String = connection.query_row(
                "SELECT strftime('%d.%m',date('now',?1))",
                [&modifier],
                |r| r.get(0),
            )?;
            let day: String =
                connection.query_row("SELECT date('now',?1)", [&modifier], |r| r.get(0))?;
            let mut statement =
                connection.prepare("SELECT data FROM usage_events WHERE date(created_at)=?1")?;
            let events = statement.query_map([day], |row| row.get::<_, String>(0))?;
            let value = events
                .filter_map(|row| row.ok())
                .filter_map(|data| serde_json::from_str::<UsageEvent>(&data).ok())
                .map(|event| event.estimated_cost_usd)
                .sum();
            points.push(SpendPoint { label, value });
        }
        let spend_today = points.last().map(|p| p.value).unwrap_or(0.0);
        let count = |modifier: &str| -> Result<u32> {
            Ok(connection.query_row(
                "SELECT COUNT(*) FROM render_events WHERE created_at >= datetime('now', ?1)",
                [modifier],
                |r| r.get::<_, u32>(0),
            )?)
        };
        let videos_today = count("start of day")?;
        let videos_yesterday: u32 = connection.query_row(
            "SELECT COUNT(*) FROM render_events WHERE created_at >= datetime('now','start of day','-1 day') AND created_at < datetime('now','start of day')", [], |r| r.get(0)
        )?;
        Ok(DashboardStats {
            spend_today,
            spend_points: points,
            videos_today,
            videos_yesterday,
            videos_week: count("-7 days")?,
            videos_month: count("-30 days")?,
        })
    }
}

pub fn set_secret(account: &str, value: &str) -> Result<()> {
    keyring::Entry::new(SERVICE, account)?.set_password(value)?;
    Ok(())
}

pub fn get_secret(account: &str) -> Option<String> {
    keyring::Entry::new(SERVICE, account)
        .ok()?
        .get_password()
        .ok()
}

pub fn delete_secret(account: &str) -> Result<()> {
    if let Ok(entry) = keyring::Entry::new(SERVICE, account) {
        let _ = entry.delete_credential();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn settings_round_trip() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("banshee-test-{suffix}"));
        let store = Store::open(root.clone()).unwrap();
        let mut settings = AppSettings::default();
        settings.autosave_seconds = 30;
        store.save_settings(&settings).unwrap();
        assert_eq!(store.settings().unwrap().autosave_seconds, 30);
        drop(store);
        let _ = fs::remove_dir_all(root);
    }
}
