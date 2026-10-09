use crate::events::Event;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Serialize;
use std::collections::HashMap;
use uploadimg_core::{CandidateAction, Core, Effect, Image, ResultAction, Source};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiState {
    simulation: bool,
    native_available: bool,
    platform: &'static str,
    paused: bool,
    clipboard_enabled: bool,
    clipboard_available: bool,
    folder_available: bool,
    notification_status: String,
    status: String,
    pending: Vec<PendingSummary>,
    results: Vec<ResultSummary>,
    selected_candidate_id: Option<String>,
    selected_result_id: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PendingSummary {
    id: String,
    source: &'static str,
    width: u32,
    height: u32,
    input_bytes: usize,
    expires_in_seconds: u64,
    preview_data_url: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResultSummary {
    id: String,
    url: String,
    fictitious: bool,
    expires_in_seconds: u64,
}

pub enum Operation {
    Configure {
        enabled: bool,
        paused: bool,
        generation: u64,
        user_change: bool,
    },
    Notify {
        id: String,
        result: bool,
    },
    SyncIds(Vec<String>),
    Result {
        id: String,
        action: i32,
    },
    ShowWindow {
        candidate_id: Option<String>,
    },
    Quit,
}
pub struct EffectBatch {
    pub revision: u64,
    pub live_ids: Vec<String>,
    pub operations: Vec<Operation>,
}

pub struct Model {
    delivery_revision: u64,
    pub core: Core,
    pub generation: u64,
    lifecycle: u64,
    previews: HashMap<String, String>,
    selected_candidate: Option<String>,
    selected_result: Option<String>,
    notifications: String,
    status: String,
    native_available: bool,
}
impl Model {
    pub fn new(native_available: bool) -> Self {
        Self { delivery_revision: 0, core: Core::default(), generation: 0, lifecycle: 0, previews: HashMap::new(), selected_candidate: None,
            selected_result: None, notifications: if native_available { "not_requested" } else { "unavailable" }.into(),
            status: if native_available { "Simulación local. Portapapeles apagado; requiere activación explícita." }
                else { "Adaptadores nativos no disponibles en esta plataforma. No se ha verificado su ejecución." }.into(), native_available }
    }
    // Called while holding the model lock. Each batch carries the complete ID
    // snapshot, so the destination can reject obsolete effects after reordering.
    pub fn effect_batch(&mut self, mut operations: Vec<Operation>) -> EffectBatch {
        // Several OS events can share a drain. Applying intermediate controls
        // could briefly enable reads before the final pause/disable in that drain.
        let last_control = operations
            .iter()
            .rposition(|op| matches!(op, Operation::Configure { .. }));
        let user_change = operations.iter().any(|op| {
            matches!(
                op,
                Operation::Configure {
                    user_change: true,
                    ..
                }
            )
        });
        if let Some(last) = last_control {
            if let Operation::Configure {
                user_change: final_change,
                ..
            } = &mut operations[last]
            {
                *final_change |= user_change;
            }
        }
        let operations = operations
            .into_iter()
            .enumerate()
            .filter_map(|(index, operation)| {
                if matches!(operation, Operation::Configure { .. }) && Some(index) != last_control {
                    None
                } else {
                    Some(operation)
                }
            })
            .collect();
        self.delivery_revision = self
            .delivery_revision
            .checked_add(1)
            .expect("effect revision exhausted");
        let Operation::SyncIds(live_ids) = self.sync_ids() else {
            unreachable!()
        };
        EffectBatch {
            revision: self.delivery_revision,
            live_ids,
            operations,
        }
    }
    pub fn require_native(&self) -> Result<(), String> {
        if self.native_available {
            Ok(())
        } else {
            Err("Adaptadores nativos no disponibles en esta plataforma.".into())
        }
    }
    pub fn can_foreground_candidate(&mut self, id: &str, now: u64) -> bool {
        self.tick(now);
        self.selected_candidate.as_deref() == Some(id) && self.previews.contains_key(id)
    }
    pub fn tick(&mut self, now: u64) {
        self.core.expire(now);
        self.previews
            .retain(|id, _| self.core.pending().iter().any(|p| &p.id == id));
        if !self
            .core
            .pending()
            .iter()
            .any(|p| Some(&p.id) == self.selected_candidate.as_ref())
        {
            self.selected_candidate = None;
        }
        if !self
            .core
            .results()
            .iter()
            .any(|p| Some(&p.id) == self.selected_result.as_ref())
        {
            self.selected_result = None;
        }
    }
    pub fn snapshot(&mut self, now: u64) -> UiState {
        self.tick(now);
        UiState {
            simulation: true,
            native_available: self.native_available,
            platform: std::env::consts::OS,
            paused: self.core.paused(),
            clipboard_enabled: self.core.clipboard_enabled(),
            clipboard_available: self.native_available,
            folder_available: false,
            notification_status: self.notifications.clone(),
            status: self.status.clone(),
            pending: self
                .core
                .pending()
                .iter()
                .map(|p| PendingSummary {
                    id: p.id.clone(),
                    source: match p.source {
                        Source::Synthetic => "synthetic",
                        Source::Clipboard => "clipboard",
                    },
                    width: p.image.width,
                    height: p.image.height,
                    input_bytes: p.image.bytes().len(),
                    expires_in_seconds: remaining(p.created_at, now, 600_000),
                    preview_data_url: self.previews.get(&p.id).cloned().unwrap_or_default(),
                })
                .collect(),
            results: self
                .core
                .results()
                .iter()
                .map(|r| ResultSummary {
                    id: r.id.clone(),
                    url: r.url.clone(),
                    fictitious: true,
                    expires_in_seconds: remaining(r.created_at, now, 86_400_000),
                })
                .collect(),
            selected_candidate_id: self.selected_candidate.clone(),
            selected_result_id: self.selected_result.clone(),
        }
    }
    pub fn configure(&self, user_change: bool) -> Operation {
        Operation::Configure {
            enabled: self.core.clipboard_enabled(),
            paused: self.core.paused(),
            generation: self.generation,
            user_change,
        }
    }
    pub fn set_clipboard(&mut self, enabled: bool, now: u64) -> Result<Vec<Operation>, String> {
        self.require_native()?;
        self.generation = self
            .generation
            .checked_add(1)
            .expect("source control token exhausted");
        self.core.set_clipboard_enabled(enabled);
        self.tick(now);
        self.status = if enabled { "Detector de imágenes activado con baseline nuevo. La lectura puede solicitar permiso del sistema." }
            else { "Detector apagado; candidatos de portapapeles eliminados." }.into();
        Ok(vec![self.configure(true), self.sync_ids()])
    }
    pub fn set_paused(&mut self, paused: bool, now: u64) -> Vec<Operation> {
        self.generation = self
            .generation
            .checked_add(1)
            .expect("source control token exhausted");
        self.core.set_paused(paused);
        self.tick(now);
        self.status = if paused {
            "En pausa. Pendientes eliminados; fuentes sin consultar."
        } else {
            "Reanudado. Solo se detectarán imágenes nuevas a partir de ahora."
        }
        .into();
        vec![self.configure(false), self.sync_ids()]
    }
    pub fn request_notifications(&mut self) -> Result<(), String> {
        self.require_native()?;
        self.notifications = "requesting".into();
        Ok(())
    }
    pub fn candidate_action(
        &mut self,
        id: &str,
        action: &str,
        now: u64,
    ) -> Result<Vec<Operation>, String> {
        let action = match action {
            "preview" => CandidateAction::Preview,
            "share" => CandidateAction::Share,
            "ignore" => CandidateAction::Ignore,
            _ => return Err("Acción desconocida.".into()),
        };
        let result_id = if matches!(action, CandidateAction::Share) {
            random_id(24)?
        } else {
            String::new()
        };
        let effect = self
            .core
            .act(id, action, &result_id, now)
            .map_err(|_| "Esta imagen ya no está disponible.".to_string())?;
        let mut operations = Vec::new();
        match effect {
            Effect::Preview(id) => {
                self.selected_candidate = Some(id);
                operations.push(Operation::ShowWindow { candidate_id: None });
            }
            Effect::Ignored => {
                self.status = "Imagen ignorada. No se ha realizado ninguna subida.".into();
            }
            Effect::Shared(_bytes, result) => {
                // Fake transport: never open a socket or publish the original bytes.
                self.status =
                    "Compartido solo en la simulación; enlace ficticio e inaccesible.".into();
                self.selected_result = Some(result.id.clone());
                operations.push(Operation::Notify {
                    id: result.id,
                    result: true,
                });
            }
            _ => return Err("Estado de candidato inválido.".into()),
        }
        self.tick(now);
        operations.push(self.sync_ids());
        Ok(operations)
    }
    pub fn result_action(
        &mut self,
        id: &str,
        action: &str,
        now: u64,
    ) -> Result<Vec<Operation>, String> {
        let typed = match action {
            "preview" => ResultAction::Preview,
            "copy" => ResultAction::Copy,
            "open" => ResultAction::Open,
            _ => return Err("Acción desconocida.".into()),
        };
        let effect = self
            .core
            .result_action(id, typed, now)
            .map_err(|_| "Este enlace ficticio ya no está disponible.".to_string())?;
        match effect {
            Effect::ShowResult(result) => {
                self.selected_result = Some(result.id);
                Ok(vec![Operation::ShowWindow { candidate_id: None }])
            }
            Effect::Copy(url) | Effect::Open(url) => {
                if !Core::valid_result(id, &url) {
                    return Err("Enlace rechazado.".into());
                }
                self.require_native()?;
                self.status = if action == "copy" { "Copia explícita del enlace ficticio solicitada." }
                    else { "Apertura explícita del enlace ficticio solicitada; no contiene una imagen pública." }.into();
                Ok(vec![Operation::Result {
                    id: id.into(),
                    action: if action == "copy" { 5 } else { 6 },
                }])
            }
            _ => Err("Estado de resultado inválido.".into()),
        }
    }
    pub fn apply_event(&mut self, event: Event, now: u64) -> Vec<Operation> {
        self.tick(now);
        match event {
            Event::Image(event) => {
                if !self.native_available
                    || event.generation != self.generation
                    || event.lifecycle != self.lifecycle
                    || !(0..=2).contains(&event.source)
                    || !crate::events::valid_thumbnail(&event.thumbnail)
                {
                    return vec![];
                }
                let Ok(image) = Image::new(event.original, event.digest, event.width, event.height)
                else {
                    return vec![];
                };
                let Ok(id) = random_id(16) else {
                    self.status = "No se pudo generar un identificador seguro.".into();
                    return vec![];
                };
                let source = if event.source == 0 {
                    Source::Synthetic
                } else {
                    Source::Clipboard
                };
                if self
                    .core
                    .admit(id.clone(), image, source, now, event.source == 2)
                    .is_err()
                {
                    self.status = "Sin candidato nuevo: duplicado, límite de pendientes o fuente desactivada.".into();
                    return vec![];
                }
                self.previews.insert(
                    id.clone(),
                    format!("data:image/png;base64,{}", STANDARD.encode(event.thumbnail)),
                );
                self.selected_candidate = Some(id.clone());
                self.status = "Imagen lista para compartir; requiere Compartir explícito.".into();
                let mut operations = vec![
                    Operation::Notify {
                        id: id.clone(),
                        result: false,
                    },
                    self.sync_ids(),
                ];
                if event.source != 0 {
                    operations.push(Operation::ShowWindow {
                        candidate_id: Some(id),
                    });
                }
                operations
            }
            Event::Control {
                kind: 2,
                code,
                generation,
                ..
            } if generation == self.generation => {
                self.status = match code {
                    1 => "Pausa o procesamiento activo; vuelve a intentarlo.",
                    2 => "La imagen supera 10 MB.",
                    3 => "No hay una imagen PNG, TIFF o JPEG compatible.",
                    4 => "Imagen inválida, animada o fuera de los límites (10 MB, 16 MP, 8192 px).",
                    5 => "No se entregó la notificación; revisa pendientes en la aplicación.",
                    _ => "Operación nativa no disponible.",
                }
                .into();
                vec![]
            }
            Event::Control {
                kind: 3,
                generation,
                ..
            } if generation == self.generation => {
                self.generation = self
                    .generation
                    .checked_add(1)
                    .expect("source control token exhausted");
                self.core.set_clipboard_enabled(false);
                self.tick(now);
                self.status = "Lectura de imágenes no disponible o denegada. Detector detenido. Usa Pegar imagen explícitamente o revisa permisos.".into();
                vec![self.configure(true), self.sync_ids()]
            }
            Event::Control { kind: 4, code, .. } => {
                self.notifications = match code {
                    1 => "granted",
                    2 => "requesting",
                    _ => "denied",
                }
                .into();
                self.status = if code == 1 { "Permiso de notificaciones concedido; presentación y acciones dependen de macOS/Focus." }
                    else if code == 2 { "Solicitud explícita de notificaciones en curso." }
                    else { "Notificaciones desactivadas; revisa pendientes desde la aplicación o bandeja." }.into();
                if code == 1 {
                    self.core
                        .pending()
                        .iter()
                        .map(|p| Operation::Notify {
                            id: p.id.clone(),
                            result: false,
                        })
                        .collect()
                } else {
                    vec![]
                }
            }
            Event::Control {
                kind: 5,
                generation,
                lifecycle,
                code: 1,
                ..
            } if generation == self.generation && lifecycle > self.lifecycle => {
                // A configure ACK fences in-flight images, but the Rust control
                // already applied its source-specific pending-candidate policy.
                self.lifecycle = lifecycle;
                vec![]
            }
            Event::Control {
                kind: 5,
                lifecycle,
                code: 0,
                ..
            } if lifecycle > self.lifecycle => {
                // Native callbacks are FIFO and lifecycle is native-owned. A
                // queued real wake still purges pending work if Rust changed
                // control meanwhile; it must never adopt that old Rust token.
                self.lifecycle = lifecycle;
                let paused = self.core.paused();
                self.core.set_paused(true);
                self.core.set_paused(paused);
                self.tick(now);
                self.status = "Despertar: pendientes invalidados y baseline nativo renovado; pausa preservada.".into();
                vec![self.sync_ids()]
            }
            Event::Control {
                kind: 6,
                code,
                text,
                ..
            } => {
                let outcome = match code {
                    1 => self.candidate_action(&text, "preview", now),
                    2 => self.candidate_action(&text, "share", now),
                    3 => self.candidate_action(&text, "ignore", now),
                    4 => self.result_action(&text, "preview", now),
                    5 => self.result_action(&text, "copy", now),
                    6 => self.result_action(&text, "open", now),
                    7 => Ok(vec![Operation::ShowWindow { candidate_id: None }]),
                    8 => Ok(self.set_paused(!self.core.paused(), now)),
                    9 => Ok(vec![Operation::Quit]),
                    _ => Err("Acción nativa desconocida.".into()),
                };
                match outcome {
                    Ok(ops) => ops,
                    Err(error) => {
                        self.status = error;
                        vec![]
                    }
                }
            }
            _ => vec![],
        }
    }
    pub fn sync_ids(&self) -> Operation {
        Operation::SyncIds(
            self.core
                .pending()
                .iter()
                .map(|p| format!("candidate.{}", p.id))
                .chain(
                    self.core
                        .results()
                        .iter()
                        .map(|r| format!("result.{}", r.id)),
                )
                .collect(),
        )
    }
}
fn random_id(bytes: usize) -> Result<String, String> {
    let mut buffer = vec![0u8; bytes];
    getrandom::fill(&mut buffer)
        .map_err(|_| "No se pudo generar un identificador seguro.".to_string())?;
    Ok(buffer.iter().map(|b| format!("{b:02x}")).collect())
}
fn remaining(created: u64, now: u64, lifetime: u64) -> u64 {
    lifetime
        .saturating_sub(now.saturating_sub(created))
        .div_ceil(1000)
}
#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
