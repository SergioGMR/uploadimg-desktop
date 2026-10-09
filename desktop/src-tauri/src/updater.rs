use crate::{execute, update_trust, Model, Runtime};
use semver::Version;
use serde::Serialize;
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tauri::{AppHandle, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

pub const CURRENT_VERSION: &str = env!("UPLOADIMG_VERSION");
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const PUBLIC_KEY: &str = env!("UPLOADIMG_UPDATER_PUBLIC_KEY");
const FEED_BASE: &str =
    "https://raw.githubusercontent.com/SergioGMR/uploadimg-desktop/main/updates/";
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    state: &'static str,
    current_version: &'static str,
    offered_version: Option<String>,
    channel: &'static str,
    message: String,
    recovery_path: Option<String>,
    can_install: bool,
    blocked_reason: Option<String>,
}
pub struct UpdaterState(Arc<Mutex<Inner>>);
struct Inner {
    status: Status,
    offered: Option<Update>,
    busy: bool,
}
fn newer(current: &Version, offered: &Version) -> bool {
    offered.to_string().len() <= 128
        && (!current.pre.is_empty() || offered.pre.is_empty())
        && offered.cmp_precedence(current).is_gt()
}
fn channel(version: &Version) -> &'static str {
    if version.pre.is_empty() {
        "stable"
    } else {
        "beta"
    }
}
fn disabled_reason() -> Option<&'static str> {
    if !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some("Actualizaciones disponibles solo para macOS arm64.")
    } else if !update_trust::valid_key(PUBLIC_KEY) {
        Some("Actualizaciones desactivadas: falta una clave pública válida.")
    } else {
        None
    }
}
impl UpdaterState {
    pub fn new() -> Self {
        let reason = disabled_reason();
        Self(Arc::new(Mutex::new(Inner {
            status: Status {
                state: if reason.is_some() { "disabled" } else { "idle" },
                current_version: CURRENT_VERSION,
                offered_version: None,
                channel: channel(
                    &Version::parse(CURRENT_VERSION).expect("build validates VERSION"),
                ),
                message: reason
                    .unwrap_or("Busca actualizaciones cuando quieras.")
                    .into(),
                recovery_path: None,
                can_install: false,
                blocked_reason: None,
            },
            offered: None,
            busy: false,
        })))
    }
    pub fn status(&self, model: &Model) -> Result<Status, String> {
        let inner = self.0.lock().map_err(|_| "Actualizador no disponible.")?;
        Ok(project_status(
            &inner.status,
            inner.offered.is_some(),
            inner.busy,
            model,
        ))
    }
    fn claim_install(&self, version: &str) -> Result<(Flight, Update), String> {
        let mut inner = self.0.lock().map_err(|_| "Actualizador no disponible.")?;
        if inner.busy {
            return Err("Ya hay una operación de actualización en curso.".into());
        }
        if inner.status.state != "available" || inner.status.recovery_path.is_some() {
            return Err("Busca una actualización disponible antes de instalar.".into());
        }
        let update = inner
            .offered
            .as_ref()
            .filter(|update| update.version == version)
            .cloned()
            .ok_or("La versión ofrecida cambió; busca actualizaciones otra vez.")?;
        inner.busy = true;
        inner.status.state = "downloading";
        inner.status.message = "Descargando actualización firmada…".into();
        Ok((Flight(self.0.clone()), update))
    }
    fn claim(&self, phase: &'static str) -> Result<Flight, String> {
        let mut inner = self.0.lock().map_err(|_| "Actualizador no disponible.")?;
        if inner.status.state == "disabled" {
            return Err(inner.status.message.clone());
        }
        if inner.busy {
            return Err("Ya hay una operación de actualización en curso.".into());
        }
        if inner.status.recovery_path.is_some() {
            return Err("Recuperación manual de la aplicación necesaria.".into());
        }
        inner.busy = true;
        inner.status.state = phase;
        inner.status.message = if phase == "checking" {
            "Buscando actualizaciones…"
        } else {
            "Descargando actualización firmada…"
        }
        .into();
        Ok(Flight(self.0.clone()))
    }
    fn fail(&self, message: &str, recovery_path: Option<String>) {
        if let Ok(mut inner) = self.0.lock() {
            inner.status.state = "failed";
            inner.status.message = message.into();
            inner.status.recovery_path = recovery_path;
            inner.offered = None;
            inner.status.offered_version = None;
        }
    }
}
fn project_status(base: &Status, has_offer: bool, busy: bool, model: &Model) -> Status {
    let mut status = base.clone();
    status.blocked_reason = model.update_blocked_reason();
    if busy {
        status.blocked_reason = Some("Actualización en curso.".into());
    }
    if status.state == "disabled" {
        status.blocked_reason = Some(status.message.clone());
    }
    if status.recovery_path.is_some() {
        status.blocked_reason = Some("Recuperación manual de la aplicación necesaria.".into());
    }
    status.can_install =
        status.state == "available" && has_offer && !busy && status.blocked_reason.is_none();
    status
}
struct Flight(Arc<Mutex<Inner>>);
impl Drop for Flight {
    fn drop(&mut self) {
        if let Ok(mut inner) = self.0.lock() {
            inner.busy = false;
        }
    }
}
fn snapshot(app: &AppHandle) -> Result<Status, String> {
    let runtime = app.state::<Runtime>();
    let model = runtime
        .model
        .lock()
        .map_err(|_| "Estado local no disponible.")?;
    app.state::<UpdaterState>().status(&model)
}
pub fn status(app: &AppHandle) -> Result<Status, String> {
    snapshot(app)
}
fn approved_artifact(url: &reqwest::Url, version: &str) -> bool {
    url.scheme() == "https"
        && url.host_str() == Some("github.com")
        && url.username().is_empty()
        && url.password().is_none()
        && url.port_or_known_default() == Some(443)
        && url.path().starts_with(&format!(
            "/SergioGMR/uploadimg-desktop/releases/download/v{version}/"
        ))
        && url.query().is_none()
        && url.fragment().is_none()
}
fn redirect_allowed(url: &reqwest::Url) -> bool {
    url.scheme() == "https"
        && url.port_or_known_default() == Some(443)
        && url.username().is_empty()
        && url.password().is_none()
        && matches!(
            url.host_str(),
            Some(
                "raw.githubusercontent.com" | "github.com" | "release-assets.githubusercontent.com"
            )
        )
}
pub async fn check(app: AppHandle) -> Result<Status, String> {
    let state = app.state::<UpdaterState>();
    let flight = match state.claim("checking") {
        Ok(flight) => flight,
        Err(error) => return Err(error),
    };
    let channel = channel(&Version::parse(CURRENT_VERSION).expect("validated version"));
    let endpoint = format!("{FEED_BASE}{channel}.json")
        .parse()
        .expect("fixed HTTPS feed");
    let result = async {
        let updater = app
            .updater_builder()
            .pubkey(PUBLIC_KEY)
            .target("darwin-aarch64")
            .endpoints(vec![endpoint])?
            .timeout(REQUEST_TIMEOUT)
            .version_comparator(|_, offered| {
                newer(
                    &Version::parse(CURRENT_VERSION).expect("validated version"),
                    &offered.version,
                )
            })
            .configure_client(|client| {
                client
                    .https_only(true)
                    .connect_timeout(Duration::from_secs(5))
                    .read_timeout(REQUEST_TIMEOUT)
                    .redirect(reqwest::redirect::Policy::custom(|attempt| {
                        if attempt.previous().len() >= 5 || !redirect_allowed(attempt.url()) {
                            return attempt.error("Updater redirect rejected");
                        }
                        // Feed redirects remain on its fixed host; artifact redirects may use GitHub's asset host.
                        if attempt.previous().first().and_then(|url| url.host_str())
                            == Some("raw.githubusercontent.com")
                            && attempt.url().host_str() != Some("raw.githubusercontent.com")
                        {
                            attempt.error("Feed origin redirect rejected")
                        } else {
                            attempt.follow()
                        }
                    }))
            })
            .build()?;
        updater.check().await
    }
    .await;
    match result {
        Ok(offered) => {
            let mut inner = state.0.lock().map_err(|_| "Actualizador no disponible.")?;
            if let Some(update) = offered {
                if !approved_artifact(&update.download_url, &update.version) {
                    drop(inner);
                    state.fail(
                        "El feed ofrece un archivo fuera de la Release HTTPS autorizada.",
                        None,
                    );
                } else {
                    inner.status.state = "available";
                    inner.status.offered_version = Some(update.version.clone());
                    inner.status.message = format!("Versión {} disponible.", update.version);
                    inner.offered = Some(update);
                }
            } else {
                inner.status.state = "upToDate";
                inner.status.offered_version = None;
                inner.offered = None;
                inner.status.message = "La versión instalada está actualizada.".into();
            }
        }
        Err(_) => state.fail(
            "No se pudo consultar el feed de actualizaciones. Puedes volver a intentarlo.",
            None,
        ),
    }
    drop(flight);
    snapshot(&app)
}
fn budget_exceeded(total: u64, advertised: Option<u64>) -> bool {
    total > update_trust::MAX_ARCHIVE_BYTES as u64
        || advertised.is_some_and(|n| n > update_trust::MAX_ARCHIVE_BYTES as u64)
}
struct AbortOnDrop(tokio::task::AbortHandle);
impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}
async fn download_with<F, Fut>(perform: F) -> Result<Vec<u8>, String>
where
    F: FnOnce(Box<dyn FnMut(usize, Option<u64>) + Send>) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<Vec<u8>, String>> + Send + 'static,
{
    use std::sync::atomic::AtomicBool;
    let count = Arc::new(AtomicU64::new(0));
    let exceeded = Arc::new(AtomicBool::new(false));
    let abort = Arc::new(Mutex::new(None::<tokio::task::AbortHandle>));
    let tally = count.clone();
    let overflow = exceeded.clone();
    let cancellation = abort.clone();
    let (ready, start) = tokio::sync::oneshot::channel::<()>();
    let task = tokio::spawn(async move {
        start.await.map_err(|_| "Descarga cancelada.".to_string())?;
        perform(Box::new(move |size, length| {
            let total = tally
                .fetch_add(size as u64, Ordering::SeqCst)
                .saturating_add(size as u64);
            if budget_exceeded(total, length) {
                overflow.store(true, Ordering::SeqCst);
                if let Ok(guard) = cancellation.lock() {
                    if let Some(handle) = guard.as_ref() {
                        handle.abort();
                    }
                }
            }
        }))
        .await
    });
    let handle = task.abort_handle();
    let _abort_on_drop = AbortOnDrop(handle.clone());
    *abort.lock().map_err(|_| "Descarga no disponible.")? = Some(handle.clone());
    let _ = ready.send(());
    // Upstream has already received this chunk when the callback runs. Abort is
    // observed at the next task yield; final validation also rejects same-poll completion.
    let result = tokio::time::timeout(Duration::from_secs(120), task).await;
    handle.abort();
    if exceeded.load(Ordering::SeqCst) || budget_exceeded(count.load(Ordering::SeqCst), None) {
        return Err("El archivo supera el límite de 256 MiB.".into());
    }
    let bytes = result
        .map_err(|_| "Tiempo de descarga agotado.")?
        .map_err(|_| "Descarga cancelada.")??;
    if bytes.len() > update_trust::MAX_ARCHIVE_BYTES {
        return Err("El archivo supera el límite de 256 MiB.".into());
    }
    Ok(bytes)
}
async fn download(update: Update) -> Result<Vec<u8>, String> {
    download_with(move |on_chunk| async move {
        update
            .download(on_chunk, || {})
            .await
            .map_err(|_| "La descarga o su firma/version no son válidas.".into())
    })
    .await
}
struct InstallBarrier {
    app: AppHandle,
    release: bool,
}
impl InstallBarrier {
    fn begin(app: &AppHandle) -> Result<Self, String> {
        let runtime = app.state::<Runtime>();
        let batch = {
            let mut model = runtime
                .model
                .lock()
                .map_err(|_| "Estado local no disponible.")?;
            model.begin_update(runtime.now())?;
            let op = model.configure(false);
            model.effect_batch(vec![op])
        };
        execute(app, batch);
        Ok(Self {
            app: app.clone(),
            release: true,
        })
    }
    fn safe(&self) -> bool {
        self.app
            .state::<Runtime>()
            .model
            .lock()
            .map(|model| model.update_safe())
            .unwrap_or(false)
    }
}
impl Drop for InstallBarrier {
    fn drop(&mut self) {
        if self.release {
            let runtime = self.app.state::<Runtime>();
            if let Ok(mut model) = runtime.model.lock() {
                model.end_update();
                let op = model.configure(false);
                let batch = model.effect_batch(vec![op]);
                drop(model);
                execute(&self.app, batch);
            };
        }
    }
}
fn record_install_result(
    state: &UpdaterState,
    result: Result<(), tauri_plugin_updater::Error>,
) -> bool {
    match result {
        Ok(()) => true,
        Err(tauri_plugin_updater::Error::PreviousAppNotRestored(path)) => {
            state.fail("No se pudo restaurar la aplicación anterior. Conserva el respaldo y reinstálalo manualmente antes de reiniciar.",Some(path.to_string_lossy().into_owned()));
            false
        }
        Err(_) => {
            state.fail("No se pudo instalar. La aplicación no se reiniciará; puedes volver a buscar actualizaciones.",None);
            false
        }
    }
}
pub async fn install(
    app: AppHandle,
    version: String,
    confirm_restart: bool,
) -> Result<Status, String> {
    if !confirm_restart {
        return Err("Confirma el reinicio y la pérdida del historial en memoria.".into());
    }
    let state = app.state::<UpdaterState>();
    let (flight, update) = state.claim_install(&version)?;
    let mut barrier = match InstallBarrier::begin(&app) {
        Ok(guard) => guard,
        Err(error) => {
            state.fail(&error, None);
            drop(flight);
            return Err(error);
        }
    };
    let mut download_update = update.clone();
    download_update.timeout = Some(REQUEST_TIMEOUT);
    let bytes = match download(download_update).await {
        Ok(bytes) => bytes,
        Err(error) => {
            state.fail(&error, None);
            drop(barrier);
            drop(flight);
            return snapshot(&app);
        }
    };
    if !barrier.safe() {
        state.fail(
            "Hay trabajo pendiente; la instalación se ha cancelado.",
            None,
        );
        drop(barrier);
        drop(flight);
        return snapshot(&app);
    }
    // The plugin verifies signature plus signed VERSION during download. Verify again
    // at this boundary so helpers and installer share the exact trust requirement.
    if update_trust::verify(&bytes, &update.signature, PUBLIC_KEY, &version).is_err() {
        state.fail("Firma o versión firmada rechazada.", None);
        drop(barrier);
        drop(flight);
        return snapshot(&app);
    }
    {
        let mut inner = state.0.lock().map_err(|_| "Actualizador no disponible.")?;
        inner.status.state = "installing";
        inner.status.message = "Instalando; la aplicación se reiniciará…".into();
    }
    if !barrier.safe() {
        state.fail("La instalación se ha cancelado por seguridad.", None);
        drop(barrier);
        drop(flight);
        return snapshot(&app);
    }
    let handle = app.clone();
    // The blocking installer owns both guards: cancellation of this IPC future
    // cannot reopen admissions or the updater while replacement is still running.
    let result = tauri::async_runtime::spawn_blocking(move || {
        let _flight = flight;
        let state = handle.state::<UpdaterState>();
        let restart = record_install_result(&state, update.install(bytes));
        if restart {
            handle.restart();
        }
        if state
            .0
            .lock()
            .map(|inner| inner.status.recovery_path.is_some())
            .unwrap_or(true)
        {
            barrier.release = false;
        }
        drop(barrier);
    })
    .await;
    if result.is_err() {
        state.fail(
            "La instalación se interrumpió. Comprueba la aplicación antes de reiniciar.",
            None,
        );
    }
    snapshot(&app)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn semver_channels_and_precedence_use_full_distribution_version() {
        for (current, offered, expected) in [
            ("0.1.0-beta.9", "0.1.0-beta.10", true),
            ("0.1.0-beta.10", "0.1.0", true),
            ("0.1.0", "0.2.0-beta.1", false),
            ("0.1.0-beta.10", "0.1.0-beta.9", false),
            ("0.1.0-beta.10", "0.1.0-beta.10+build.2", false),
            ("0.1.0", "0.1.0+build.2", false),
            ("0.1.0", "0.1.1", true),
        ] {
            assert_eq!(
                newer(
                    &Version::parse(current).unwrap(),
                    &Version::parse(offered).unwrap()
                ),
                expected,
                "{current}->{offered}"
            );
        }
    }
    #[test]
    fn checks_downloads_and_installs_share_one_flight_and_release_after_failure() {
        let state = UpdaterState::new();
        state.0.lock().unwrap().status.state = "idle";
        let flight = state.claim("checking").unwrap();
        assert!(state.claim("checking").is_err());
        assert!(state.claim("downloading").is_err());
        drop(flight);
        assert!(state.claim("downloading").is_ok());
    }
    #[test]
    fn fixed_release_urls_and_redirects_reject_http_foreign_origins_and_other_versions() {
        for bad in ["http://github.com/SergioGMR/uploadimg-desktop/releases/download/v0.1.0-beta.3/a.tar.gz", "https://evil.example/a", "https://github.com/SergioGMR/uploadimg-desktop/releases/download/v0.1.0-beta.2/a.tar.gz", "https://github.com/other/repo/releases/download/v0.1.0-beta.3/a.tar.gz"] {
            assert!(!approved_artifact(&bad.parse().unwrap(),"0.1.0-beta.3"));
        }
        assert!(approved_artifact(&"https://github.com/SergioGMR/uploadimg-desktop/releases/download/v0.1.0-beta.3/UploadIMG.app.tar.gz".parse().unwrap(),"0.1.0-beta.3"));
        assert!(!redirect_allowed(
            &"http://release-assets.githubusercontent.com/a"
                .parse()
                .unwrap()
        ));
        assert!(!redirect_allowed(
            &"https://evil.example/a".parse().unwrap()
        ));
        assert!(redirect_allowed(
            &"https://release-assets.githubusercontent.com/a"
                .parse()
                .unwrap()
        ));
    }
    #[test]
    fn oversize_download_is_cancelled_and_cannot_reach_install() {
        let installed = std::sync::atomic::AtomicBool::new(false);
        let result = tauri::async_runtime::block_on(download_with(|mut on_chunk| async move {
            on_chunk(1, Some(update_trust::MAX_ARCHIVE_BYTES as u64 + 1));
            std::future::pending::<Result<Vec<u8>, String>>().await
        }));
        if result.is_ok() {
            installed.store(true, Ordering::SeqCst);
        }
        assert!(result.is_err());
        assert!(!installed.load(Ordering::SeqCst));
        let result = tauri::async_runtime::block_on(download_with(|mut on_chunk| async move {
            on_chunk(update_trust::MAX_ARCHIVE_BYTES + 1, None);
            Ok(vec![1])
        }));
        assert!(result.is_err());
    }

    #[test]
    fn backend_status_matches_ui_contract_and_tracks_pending_work() {
        let updater = UpdaterState::new();
        let mut base = updater.0.lock().unwrap().status.clone();
        base.state = "available";
        base.offered_version = Some("0.1.0-beta.4".into());
        let model = Model::new(true);
        let projected = project_status(&base, true, false, &model);
        assert!(projected.can_install);
        assert!(projected.blocked_reason.is_none());
        let json = serde_json::to_value(projected).unwrap();
        let mut keys = json
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>();
        keys.sort();
        assert_eq!(
            keys,
            vec![
                "blockedReason",
                "canInstall",
                "channel",
                "currentVersion",
                "message",
                "offeredVersion",
                "recoveryPath",
                "state"
            ]
        );
        assert_eq!(json["currentVersion"], CURRENT_VERSION);
        assert!(!project_status(&base, true, true, &model).can_install);
        let mut model = Model::new(true);
        model
            .core
            .admit(
                "a".repeat(32),
                uploadimg_core::Image::new(vec![1], [1; 32], 1, 1).unwrap(),
                uploadimg_core::Source::Synthetic,
                0,
                true,
            )
            .unwrap();
        assert!(!project_status(&base, true, false, &model).can_install);
        assert!(project_status(&base, true, false, &model)
            .blocked_reason
            .unwrap()
            .contains("pendientes"));
        base.state = "failed";
        assert!(!project_status(&base, true, false, &Model::new(true)).can_install);
        base.state = "available";
        base.recovery_path = Some("/tmp/previous.app".into());
        assert!(!project_status(&base, true, false, &Model::new(true)).can_install);
    }

    #[test]
    fn installation_failure_never_authorizes_restart_and_retains_manual_recovery_path() {
        let state = UpdaterState::new();
        let error = tauri_plugin_updater::Error::PreviousAppNotRestored(std::path::PathBuf::from(
            "/tmp/previous.app",
        ));
        assert!(!record_install_result(&state, Err(error)));
        let status = state.0.lock().unwrap().status.clone();
        assert_eq!(status.state, "failed");
        assert_eq!(status.recovery_path.as_deref(), Some("/tmp/previous.app"));
        assert!(state.claim("checking").is_err());
        let state = UpdaterState::new();
        assert!(!record_install_result(
            &state,
            Err(std::io::Error::other("unwritable install").into())
        ));
        assert_eq!(state.0.lock().unwrap().status.state, "failed");
        assert!(state.0.lock().unwrap().status.recovery_path.is_none());
        assert!(record_install_result(&state, Ok(())));
    }
    #[test]
    fn archive_request_total_timeout_is_fifteen_seconds_separate_from_watchdog() {
        assert_eq!(REQUEST_TIMEOUT, Duration::from_secs(15));
    }
}
