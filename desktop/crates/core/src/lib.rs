//! Platform-independent candidate lifecycle. All time inputs are monotonic milliseconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Synthetic,
    Clipboard,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    InvalidImage,
    InvalidId,
    Disabled,
    Paused,
    Duplicate,
    Capacity,
    NotFound,
}
#[derive(Debug)]
pub struct Image {
    bytes: Vec<u8>,
    pub digest: [u8; 32],
    pub width: u32,
    pub height: u32,
}
impl Image {
    pub fn new(bytes: Vec<u8>, digest: [u8; 32], width: u32, height: u32) -> Result<Self, Error> {
        if bytes.is_empty()
            || bytes.len() > 10_000_000
            || width == 0
            || height == 0
            || width > 8192
            || height > 8192
            || u64::from(width) * u64::from(height) > 16_000_000
        {
            return Err(Error::InvalidImage);
        }
        Ok(Self {
            bytes,
            digest,
            width,
            height,
        })
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}
#[derive(Debug)]
pub struct Candidate {
    pub id: String,
    pub image: Image,
    pub source: Source,
    pub created_at: u64,
}
#[derive(Clone, Debug)]
pub struct ResultItem {
    pub id: String,
    pub url: String,
    pub created_at: u64,
    pub expires_at: u64,
}
#[derive(Debug)]
pub enum Effect {
    Preview(String),
    Shared(Image),
    ShowResult(ResultItem),
    Copy(String),
    Open(String),
    Ignored,
}
#[derive(Clone, Copy, Debug)]
pub enum CandidateAction {
    Preview,
    Share,
    Ignore,
}
#[derive(Clone, Copy, Debug)]
pub enum ResultAction {
    Preview,
    Copy,
    Open,
}
#[derive(Default)]
pub struct Core {
    paused: bool,
    clipboard_enabled: bool,
    pending: Vec<Candidate>,
    results: Vec<ResultItem>,
    fingerprints: std::collections::VecDeque<([u8; 32], u64)>,
    failed_retry: Option<([u8; 32], u64)>,
    last_tick: u64,
}
impl Core {
    pub fn pending(&self) -> &[Candidate] {
        &self.pending
    }
    pub fn results(&self) -> &[ResultItem] {
        &self.results
    }
    pub fn paused(&self) -> bool {
        self.paused
    }
    pub fn clipboard_enabled(&self) -> bool {
        self.clipboard_enabled
    }
    pub fn set_clipboard_enabled(&mut self, value: bool) {
        self.clipboard_enabled = value;
        if !value {
            self.pending.retain(|p| p.source != Source::Clipboard);
        }
    }
    pub fn set_paused(&mut self, value: bool) {
        self.paused = value;
        if value {
            self.pending.clear();
        }
    }
    fn observe(&mut self, now: u64) -> u64 {
        self.last_tick = self.last_tick.max(now);
        self.last_tick
    }
    pub fn expire(&mut self, now: u64) {
        let now = self.observe(now);
        self.pending
            .retain(|p| now.saturating_sub(p.created_at) < 600_000);
        self.results.retain(|p| now < p.expires_at);
        self.fingerprints
            .retain(|p| now.saturating_sub(p.1) < 30_000);
        if self
            .failed_retry
            .is_some_and(|p| now.saturating_sub(p.1) >= 30_000)
        {
            self.failed_retry = None;
        }
    }
    // A concrete terminal failure allows one explicit paste of that admission,
    // without releasing suppression for automatic events or newer admissions.
    pub fn allow_failed_retry(&mut self, digest: [u8; 32], admitted_at: u64) {
        let fingerprint = (digest, admitted_at);
        if self.fingerprints.contains(&fingerprint) {
            self.failed_retry = Some(fingerprint);
        }
    }
    pub fn admit(
        &mut self,
        id: String,
        image: Image,
        source: Source,
        now: u64,
        explicit_paste: bool,
    ) -> Result<(), Error> {
        self.expire(now);
        if image.bytes.is_empty()
            || image.bytes.len() > 10_000_000
            || image.width == 0
            || image.height == 0
            || image.width > 8192
            || image.height > 8192
            || u64::from(image.width) * u64::from(image.height) > 16_000_000
        {
            return Err(Error::InvalidImage);
        }
        if !hex_id(&id, 32) || self.pending.iter().any(|p| p.id == id) {
            return Err(Error::InvalidId);
        }
        if self.paused {
            return Err(Error::Paused);
        }
        if source == Source::Clipboard && !self.clipboard_enabled && !explicit_paste {
            return Err(Error::Disabled);
        }
        let duplicate = self
            .fingerprints
            .iter()
            .find(|f| f.0 == image.digest)
            .copied();
        let retry = explicit_paste
            && duplicate.is_some()
            && duplicate == self.failed_retry
            && !self.pending.iter().any(|p| p.image.digest == image.digest);
        if duplicate.is_some() && !retry {
            return Err(Error::Duplicate);
        }
        if self.pending.len() == 3
            || self
                .pending
                .iter()
                .map(|p| p.image.bytes.len())
                .sum::<usize>()
                + image.bytes.len()
                > 30_000_000
        {
            return Err(Error::Capacity);
        }
        if retry {
            self.fingerprints.retain(|f| Some(*f) != duplicate);
            self.failed_retry = None;
        }
        self.fingerprints.push_back((image.digest, self.last_tick));
        if self.fingerprints.len() > 128 {
            self.fingerprints.pop_front();
        }
        self.pending.push(Candidate {
            id,
            image,
            source,
            created_at: self.last_tick,
        });
        Ok(())
    }
    pub fn act(
        &mut self,
        id: &str,
        action: CandidateAction,
        result_id: &str,
        now: u64,
    ) -> Result<Effect, Error> {
        self.expire(now);
        let index = self
            .pending
            .iter()
            .position(|p| p.id == id)
            .ok_or(Error::NotFound)?;
        match action {
            CandidateAction::Preview => Ok(Effect::Preview(id.into())),
            CandidateAction::Ignore => {
                self.pending.remove(index);
                Ok(Effect::Ignored)
            }
            CandidateAction::Share => {
                if !hex_id(result_id, 48) || self.results.iter().any(|r| r.id == result_id) {
                    return Err(Error::InvalidId);
                }
                // Removing the candidate is the serialized, atomic consent boundary.
                let candidate = self.pending.remove(index);
                Ok(Effect::Shared(candidate.image))
            }
        }
    }
    // Called only after validating the production finalize projection. Consent
    // removes a candidate but never creates a result by itself.
    pub fn record_result(
        &mut self,
        id: String,
        url: String,
        now: u64,
        remaining_ms: u64,
    ) -> Result<(), Error> {
        self.expire(now);
        if !Self::valid_result(&id, &url)
            || remaining_ms == 0
            || remaining_ms > 86_400_000
            || self.results.iter().any(|r| r.id == id)
        {
            return Err(Error::InvalidId);
        }
        self.results.push(ResultItem {
            id,
            url,
            created_at: self.last_tick,
            expires_at: self.last_tick.saturating_add(remaining_ms),
        });
        if self.results.len() > 20 {
            self.results.remove(0);
        }
        Ok(())
    }
    pub fn result_action(
        &mut self,
        id: &str,
        action: ResultAction,
        now: u64,
    ) -> Result<Effect, Error> {
        self.expire(now);
        let result = self
            .results
            .iter()
            .find(|r| r.id == id)
            .ok_or(Error::NotFound)?;
        if !Self::valid_result(&result.id, &result.url) {
            return Err(Error::InvalidId);
        }
        match action {
            ResultAction::Preview => Ok(Effect::ShowResult(result.clone())),
            ResultAction::Copy => Ok(Effect::Copy(result.url.clone())),
            ResultAction::Open => Ok(Effect::Open(result.url.clone())),
        }
    }
    pub fn valid_result(id: &str, url: &str) -> bool {
        hex_id(id, 48) && url == format!("https://uploadimg.vercel.app/i/{id}")
    }
}
fn hex_id(id: &str, length: usize) -> bool {
    id.len() == length
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
#[cfg(test)]
mod tests;
