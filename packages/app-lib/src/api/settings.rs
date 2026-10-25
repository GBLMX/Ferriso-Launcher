//! Theseus settings management interface

pub use crate::{
    State,
    state::{Hooks, MemorySettings, Settings, WindowSize},
};

/// Gets entire settings
#[tracing::instrument]
pub async fn get() -> crate::Result<Settings> {
    let state = State::get().await?;
    let settings = Settings::get(&state.pool).await?;
    Ok(settings)
}

/// Reads persisted settings before [`State::init`] has run.
///
/// Native frontends use this to seed their first frame, so theme and locale do
/// not briefly fall back to defaults while the rest of the launcher boots.
/// This is a read-only read: it does not create directories, back up, migrate,
/// or otherwise race the real [`State::init`] path.
#[tracing::instrument]
pub async fn get_startup(app_identifier: &str) -> crate::Result<Settings> {
    use std::time::Duration;

    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    let settings_dir =
        crate::state::DirectoryInfo::initial_settings_dir_path(app_identifier)
            .ok_or_else(|| {
                crate::ErrorKind::FSError(
                    "Could not find valid settings dir".to_string(),
                )
            })?;
    let db_path = settings_dir.join("app.db");
    if !db_path.is_file() {
        return Err(crate::ErrorKind::FSError(format!(
            "Settings database does not exist: {}",
            db_path.display()
        ))
        .into());
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&db_path)
                .read_only(true)
                .busy_timeout(Duration::from_secs(5)),
        )
        .await?;
    let settings = Settings::get(&pool).await;
    pool.close().await;
    settings
}

/// Sets entire settings
#[tracing::instrument]
pub async fn set(settings: Settings) -> crate::Result<()> {
    let state = State::get().await?;
    settings.update(&state.pool).await?;

    Ok(())
}

#[tracing::instrument]
pub async fn cancel_directory_change(
    app_identifier: &str,
) -> crate::Result<()> {
    // This is called to handle state initialization errors due to folder migrations
    // failing, so fetching a DB connection pool from `State::get` is not reliable here
    let pool = crate::state::db::connect(app_identifier).await?;
    let mut settings = Settings::get(&pool).await?;

    if let Some(prev_custom_dir) = settings.prev_custom_dir {
        settings.prev_custom_dir = None;
        settings.custom_dir = Some(prev_custom_dir);
    }

    settings.update(&pool).await?;

    Ok(())
}
