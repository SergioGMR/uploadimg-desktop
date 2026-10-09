//! Production wire protocol. Capabilities and original bytes never enter UI summaries.
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const ORIGIN: &str = "https://uploadimg.vercel.app";
const MAX_BODY: usize = 65_536;
pub struct Request {
    pub method: &'static str,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub timeout_ms: u64,
}
pub struct Response {
    pub status: u16,
    pub body: Vec<u8>,
}
pub trait Transport {
    fn send(&mut self, request: Request) -> Result<Response, ()>;
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ticket {
    id: String,
    secret: String,
    pathname: String,
    expires_at: u64,
    token_expires_at: u64,
    #[serde(skip)]
    deadline: Option<Instant>,
}
pub enum Job {
    Image(uploadimg_core::Image),
    Finalize(Ticket),
}
pub enum Outcome {
    Ready(PublicResult),
    Recover(Ticket),
    Failed(&'static str),
}
pub struct PublicResult {
    pub id: String,
    pub url: String,
    pub remaining_ms: u64,
}
pub fn wall_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
fn hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some("image/jpeg")
    } else {
        None
    }
}
fn post(path: &str, body: Value, timeout_ms: u64) -> Request {
    Request {
        method: "POST",
        url: format!("{ORIGIN}/api/uploads/{path}"),
        headers: vec![
            ("Content-Type".into(), "application/json".into()),
            ("Origin".into(), ORIGIN.into()),
        ],
        body: serde_json::to_vec(&body).expect("bounded JSON"),
        timeout_ms,
    }
}
fn response_json(response: &Response) -> Option<Value> {
    (response.body.len() <= MAX_BODY)
        .then(|| serde_json::from_slice(&response.body).ok())
        .flatten()
}
fn timeout(start: Instant) -> u64 {
    90_000u64.saturating_sub(start.elapsed().as_millis() as u64)
}
fn request(transport: &mut impl Transport, request: Request) -> Result<Response, ()> {
    if request.timeout_ms == 0 {
        return Err(());
    }
    transport.send(request)
}
fn validate_ticket(ticket: &Ticket, now: u64) -> bool {
    hex(&ticket.id, 48)
        && hex(&ticket.secret, 64)
        && ticket.pathname == format!("uploadimg/v1/staging/{}", ticket.id)
        && ticket.token_expires_at > now
        && ticket.expires_at > ticket.token_expires_at
        && ticket.expires_at.saturating_sub(now) <= 600_000
        && ticket.token_expires_at.saturating_sub(now) <= 300_000
        && ticket.expires_at - ticket.token_expires_at == 300_000
}
// This is parsing an opaque, server-signed capability, never client signing or
// asserting trust in its contents. Vercel verifies its signature on the PUT.
fn blob_request(
    value: &Value,
    ticket: &Ticket,
    mime: &str,
    bytes: Vec<u8>,
    now: u64,
    timeout_ms: u64,
) -> Option<Request> {
    let payload = value.get("presignedUrlPayload")?;
    let delegation = payload.get("delegationToken")?.as_str()?;
    let signature = payload.get("signature")?.as_str()?;
    if delegation.len() > 16_384
        || signature.is_empty()
        || signature.len() > 256
        || !signature
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return None;
    }
    let (segment, signed) = delegation.split_once('.')?;
    if signed.is_empty()
        || !delegation
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
    {
        return None;
    }
    let scope: Value = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(segment).ok()?).ok()?;
    let store = scope
        .get("storeId")?
        .as_str()?
        .strip_prefix("store_")
        .unwrap_or(scope.get("storeId")?.as_str()?);
    if store.is_empty()
        || store.len() > 128
        || !store.bytes().all(|b| b.is_ascii_alphanumeric())
        || scope.get("pathname")?.as_str()? != ticket.pathname
        || scope.get("validUntil")?.as_u64()? != ticket.token_expires_at
        || scope.get("validUntil")?.as_u64()? <= now
        || scope.get("maximumSizeInBytes")?.as_u64()? != bytes.len() as u64
        || scope.get("allowedContentTypes")? != &json!([mime])
        || scope.get("operations")? != &json!(["put"])
    {
        return None;
    }
    let params = payload.get("params")?.as_object()?;
    if params.len() != 2
        || params.get("vercel-blob-add-random-suffix")?.as_str()? != "false"
        || params.get("vercel-blob-allow-overwrite")?.as_str()? != "false"
    {
        return None;
    }
    let mut url = tauri::Url::parse("https://vercel.com/api/blob/").ok()?;
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("pathname", &ticket.pathname);
        for (key, value) in params {
            query.append_pair(key, value.as_str()?);
        }
        query.append_pair("vercel-blob-delegation", delegation);
        query.append_pair("vercel-blob-signature", signature);
    }
    Some(Request {
        method: "PUT",
        url: url.into(),
        headers: vec![
            ("x-api-version".into(), "12".into()),
            ("x-vercel-blob-store-id".into(), store.into()),
            ("x-vercel-blob-access".into(), "private".into()),
            ("x-content-type".into(), mime.into()),
            ("Content-Type".into(), mime.into()),
        ],
        body: bytes,
        timeout_ms,
    })
}
fn public_result(value: &Value, ticket: &Ticket, now: u64) -> Option<PublicResult> {
    let id = value.get("id")?.as_str()?;
    let url = value.get("url")?.as_str()?;
    let ready = value.get("readyAt")?.as_u64()?;
    let expires = value.get("expiresAt")?.as_u64()?;
    let bounded = |field, max| {
        value
            .get(field)
            .and_then(Value::as_u64)
            .is_some_and(|n| n > 0 && n <= max)
    };
    if id != ticket.id
        || url != format!("/i/{id}")
        || value.get("format")?.as_str()? != "webp"
        || ready == 0
        || ready > now.saturating_add(30_000)
        || expires != ready.checked_add(86_400_000)?
        || expires <= now
        || expires.saturating_sub(now) > 86_430_000
        || !bounded("width", 1920)
        || !bounded("height", 1920)
        || !bounded("bytes", 5_000_000)
        || !bounded("inputBytes", 10_000_000)
    {
        return None;
    }
    Some(PublicResult {
        id: id.into(),
        url: format!("{ORIGIN}{url}"),
        remaining_ms: (expires - now).min(86_400_000),
    })
}
pub fn run(transport: &mut impl Transport, job: Job, clock: impl Fn() -> u64) -> Outcome {
    let start = Instant::now();
    let ticket = match job {
        Job::Finalize(ticket) => ticket,
        Job::Image(image) => {
            let Some(mime) = mime(image.bytes()) else {
                return Outcome::Failed("Usa una imagen PNG o JPEG válida.");
            };
            let Ok(response) = request(
                transport,
                post(
                    "prepare",
                    json!({"expectedBytes":image.bytes().len(),"mime":mime}),
                    timeout(start),
                ),
            ) else {
                return Outcome::Failed(
                    "No se pudo iniciar la subida. Pega la imagen de nuevo para reintentar.",
                );
            };
            if response.status != 201 {
                return Outcome::Failed(http_message(response.status));
            }
            let Some(value) = response_json(&response) else {
                return Outcome::Failed("Respuesta de subida no válida.");
            };
            let Ok(mut ticket) = serde_json::from_value::<Ticket>(value) else {
                return Outcome::Failed("Respuesta de subida no válida.");
            };
            let now = clock();
            if !validate_ticket(&ticket, now) {
                return Outcome::Failed("Respuesta de subida no válida.");
            }
            ticket.deadline = Some(Instant::now() + Duration::from_millis(ticket.expires_at - now));
            let capability = json!({"id":ticket.id,"secret":ticket.secret});
            let token = post(
                "token",
                json!({"type":"blob.generate-presigned-url", "payload":{"pathname":ticket.pathname,"clientPayload":capability.to_string(),"multipart":false}}),
                timeout(start),
            );
            let Ok(response) = request(transport, token) else {
                return Outcome::Failed("No se pudo autorizar la subida. Pega la imagen de nuevo.");
            };
            if response.status != 200 {
                return Outcome::Failed(http_message(response.status));
            }
            let Some(value) = response_json(&response) else {
                return Outcome::Failed("Respuesta de autorización no válida.");
            };
            let Some(blob) = blob_request(
                &value,
                &ticket,
                mime,
                image.bytes().to_vec(),
                clock(),
                timeout(start),
            ) else {
                return Outcome::Failed("Respuesta de autorización no válida.");
            };
            drop(image);
            // Once PUT was attempted its outcome can be uncertain. Always try
            // finalize with this exact capability; never blindly resend bytes.
            let _ = request(transport, blob);
            ticket
        }
    };
    if clock() >= ticket.expires_at
        || ticket
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
    {
        return Outcome::Failed("Esta subida ha caducado. Pega la imagen de nuevo.");
    }
    let final_request = post(
        "finalize",
        json!({"id":ticket.id,"secret":ticket.secret}),
        timeout(start),
    );
    let Ok(response) = request(transport, final_request) else {
        return Outcome::Recover(ticket);
    };
    let value = response_json(&response);
    if response.status == 201 {
        if let Some(result) = value
            .as_ref()
            .and_then(|value| public_result(value, &ticket, clock()))
        {
            return Outcome::Ready(result);
        }
    } else if matches!(
        (
            response.status,
            value
                .as_ref()
                .and_then(|v| v.get("error"))
                .and_then(Value::as_str)
        ),
        (403, Some("forbidden"))
            | (404, Some("not_found"))
            | (413, Some("too_large"))
            | (422, Some("invalid_image"))
    ) {
        return Outcome::Failed("Esta subida ya no está disponible. Pega la imagen de nuevo.");
    }
    Outcome::Recover(ticket)
}
fn http_message(status: u16) -> &'static str {
    match status {
        413 => "La imagen supera el máximo de 10 MB.",
        422 => "La imagen no es compatible.",
        429 => "El servicio está ocupado. Espera y pega la imagen de nuevo.",
        507 => "El almacenamiento está lleno. Inténtalo más tarde.",
        _ => "No se pudo subir la imagen. Pega la imagen de nuevo para reintentar.",
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    struct Fake {
        replies: VecDeque<Result<Response, ()>>,
        requests: Vec<Request>,
    }
    impl Transport for Fake {
        fn send(&mut self, request: Request) -> Result<Response, ()> {
            self.requests.push(request);
            self.replies
                .pop_front()
                .expect("unexpected network operation")
        }
    }
    fn reply(status: u16, value: serde_json::Value) -> Result<Response, ()> {
        Ok(Response {
            status,
            body: serde_json::to_vec(&value).unwrap(),
        })
    }
    fn ticket() -> Ticket {
        Ticket {
            id: "a".repeat(48),
            secret: "b".repeat(64),
            pathname: format!("uploadimg/v1/staging/{}", "a".repeat(48)),
            expires_at: 1_900_000,
            token_expires_at: 1_300_000,
            deadline: None,
        }
    }
    fn result() -> serde_json::Value {
        serde_json::json!({"id":"a".repeat(48),"url":format!("/i/{}","a".repeat(48)),"readyAt":1_000_000,"expiresAt":87_400_000,"width":2,"height":2,"bytes":50,"inputBytes":8,"format":"webp"})
    }
    fn presigned() -> Value {
        let ticket = ticket();
        let scope = json!({"storeId":"store_ABC123", "pathname":ticket.pathname,"operations":["put"],"validUntil":ticket.token_expires_at,"maximumSizeInBytes":8,"allowedContentTypes":["image/png"]});
        json!({"presignedUrlPayload":{"delegationToken":format!("{}.signature",URL_SAFE_NO_PAD.encode(scope.to_string())),"signature":"signed-value", "params":{"vercel-blob-add-random-suffix":"false","vercel-blob-allow-overwrite":"false"}}})
    }
    fn initial_replies() -> VecDeque<Result<Response, ()>> {
        let ticket = ticket();
        [reply(201,json!({"id":ticket.id,"secret":ticket.secret,"pathname":ticket.pathname,"expiresAt":1_600_000,"tokenExpiresAt":1_300_000})), reply(200,presigned())].into()
    }
    fn image() -> uploadimg_core::Image {
        uploadimg_core::Image::new(b"\x89PNG\r\n\x1a\n".to_vec(), [1; 32], 2, 2).unwrap()
    }
    #[test]
    fn exact_presigned_wire_contract_and_original_bytes_precede_validated_public_result() {
        let mut fake = Fake {
            replies: initial_replies(),
            requests: vec![],
        };
        fake.replies.extend([
            reply(200, json!({"url":"private-url-never-returned"})),
            reply(201, result()),
        ]);
        let Outcome::Ready(result) = run(&mut fake, Job::Image(image()), || 1_000_000) else {
            panic!("valid upload")
        };
        assert_eq!(result.url, format!("{ORIGIN}/i/{}", "a".repeat(48)));
        assert_eq!(fake.requests.len(), 4);
        for r in [&fake.requests[0], &fake.requests[1], &fake.requests[3]] {
            assert!(r.headers.contains(&("Origin".into(), ORIGIN.into())));
            assert!(r.timeout_ms > 0 && r.timeout_ms <= 90_000);
        }
        let put = &fake.requests[2];
        assert_eq!(put.method, "PUT");
        let url = tauri::Url::parse(&put.url).unwrap();
        assert_eq!(
            (url.scheme(), url.host_str(), url.path()),
            ("https", Some("vercel.com"), "/api/blob/")
        );
        assert!(url.query_pairs().any(
            |(k, v)| k == "pathname" && v == format!("uploadimg/v1/staging/{}", "a".repeat(48))
        ));
        assert_eq!(put.body, b"\x89PNG\r\n\x1a\n");
        assert!(put.headers.contains(&("x-api-version".into(), "12".into())));
        assert!(put
            .headers
            .contains(&("x-vercel-blob-access".into(), "private".into())));
        assert!(put
            .headers
            .contains(&("x-vercel-blob-store-id".into(), "ABC123".into())));
        assert!(!put
            .headers
            .iter()
            .any(|(k, _)| k.to_lowercase() == "authorization"));
    }
    #[test]
    fn uncertain_put_finalizes_before_any_possible_new_upload() {
        let mut fake = Fake {
            replies: initial_replies(),
            requests: vec![],
        };
        fake.replies.extend([Err(()), Err(())]);
        let Outcome::Recover(ticket) = run(&mut fake, Job::Image(image()), || 1_000_000) else {
            panic!("uncertain upload needs exact recovery")
        };
        fake.replies.push_back(reply(201, result()));
        assert!(matches!(
            run(&mut fake, Job::Finalize(ticket), || 1_000_001),
            Outcome::Ready(_)
        ));
        assert_eq!(
            fake.requests.iter().filter(|r| r.method == "PUT").count(),
            1
        );
        assert_eq!(
            fake.requests
                .iter()
                .filter(|r| r.url.ends_with("/prepare"))
                .count(),
            1
        );
        assert_eq!(
            fake.requests
                .iter()
                .filter(|r| r.url.ends_with("/finalize"))
                .count(),
            2
        );
    }
    #[test]
    fn invalid_presigned_scopes_never_send_original_bytes() {
        for field in [
            "pathname",
            "validUntil",
            "maximumSizeInBytes",
            "allowedContentTypes",
            "operations",
            "storeId",
        ] {
            let mut signed = presigned();
            let delegation = signed["presignedUrlPayload"]["delegationToken"]
                .as_str()
                .unwrap();
            let mut scope: Value = serde_json::from_slice(
                &URL_SAFE_NO_PAD
                    .decode(delegation.split('.').next().unwrap())
                    .unwrap(),
            )
            .unwrap();
            scope[field] = if field == "storeId" {
                json!("evil/store")
            } else {
                json!("wrong")
            };
            signed["presignedUrlPayload"]["delegationToken"] = json!(format!(
                "{}.signature",
                URL_SAFE_NO_PAD.encode(scope.to_string())
            ));
            let mut fake = Fake {
                replies: initial_replies(),
                requests: vec![],
            };
            fake.replies[1] = reply(200, signed);
            assert!(
                matches!(
                    run(&mut fake, Job::Image(image()), || 1_000_000),
                    Outcome::Failed(_)
                ),
                "{field}"
            );
            assert_eq!(fake.requests.len(), 2);
        }
    }
    #[test]
    fn uncertain_finalize_retries_same_capability_without_preparing_or_uploading_again() {
        let ticket = ticket();
        let mut fake = Fake {
            replies: [Err(()), reply(201, result())].into(),
            requests: vec![],
        };
        let Outcome::Recover(saved) = run(&mut fake, Job::Finalize(ticket), || 1_000_000) else {
            panic!("retain recovery")
        };
        assert!(matches!(
            run(&mut fake, Job::Finalize(saved), || 1_000_001),
            Outcome::Ready(_)
        ));
        assert_eq!(fake.requests.len(), 2);
        assert!(fake
            .requests
            .iter()
            .all(|r| r.url == format!("{ORIGIN}/api/uploads/finalize") && r.method == "POST"));
        assert_eq!(fake.requests[0].body, fake.requests[1].body);
    }
    #[test]
    fn rejects_expired_ticket_and_fabricated_result() {
        let mut fake = Fake {
            replies: [reply(201, result())].into(),
            requests: vec![],
        };
        assert!(matches!(
            run(&mut fake, Job::Finalize(ticket()), || 1_900_000),
            Outcome::Failed(_)
        ));
        assert!(fake.requests.is_empty());
        let mut wrong = result();
        wrong["url"] = serde_json::json!("https://evil.invalid/i/a");
        fake.replies = [reply(201, wrong)].into();
        assert!(matches!(
            run(&mut fake, Job::Finalize(ticket()), || 1_000_000),
            Outcome::Recover(_)
        ));
    }
    #[test]
    fn malformed_public_projection_never_becomes_a_success() {
        for (field, wrong) in [
            ("id", json!("b".repeat(48))),
            ("readyAt", json!(1_100_000)),
            ("expiresAt", json!(87_400_001)),
            ("width", json!(1921)),
            ("height", json!(0)),
            ("bytes", json!(5_000_001)),
            ("inputBytes", json!(10_000_001)),
            ("format", json!("png")),
        ] {
            let mut value = result();
            value[field] = wrong;
            let mut fake = Fake {
                replies: [reply(201, value)].into(),
                requests: vec![],
            };
            assert!(
                matches!(
                    run(&mut fake, Job::Finalize(ticket()), || 1_000_000),
                    Outcome::Recover(_)
                ),
                "{field}"
            );
        }
    }
    #[test]
    fn recovery_monotonic_deadline_prevents_wall_clock_rollback_extension() {
        let mut ticket = ticket();
        ticket.deadline = Some(Instant::now() - Duration::from_millis(1));
        let mut fake = Fake {
            replies: VecDeque::new(),
            requests: vec![],
        };
        assert!(matches!(
            run(&mut fake, Job::Finalize(ticket), || 1),
            Outcome::Failed(_)
        ));
        assert!(fake.requests.is_empty());
    }
    #[test]
    fn expired_authorization_and_oversized_prepare_body_send_no_original() {
        let mut fake = Fake {
            replies: initial_replies(),
            requests: vec![],
        };
        let calls = std::cell::Cell::new(0);
        assert!(matches!(
            run(&mut fake, Job::Image(image()), || {
                let n = calls.get();
                calls.set(n + 1);
                if n == 0 {
                    1_000_000
                } else {
                    1_300_000
                }
            }),
            Outcome::Failed(_)
        ));
        assert_eq!(fake.requests.len(), 2);
        let mut fake = Fake {
            replies: [Ok(Response {
                status: 201,
                body: vec![b' '; 65_537],
            })]
            .into(),
            requests: vec![],
        };
        assert!(matches!(
            run(&mut fake, Job::Image(image()), || 1_000_000),
            Outcome::Failed(_)
        ));
        assert_eq!(fake.requests.len(), 1);
    }
    #[test]
    fn terminal_finalize_errors_clear_recovery_but_unstructured_errors_do_not() {
        for (status, code, terminal) in [
            (403, "forbidden", true),
            (404, "not_found", true),
            (413, "too_large", true),
            (422, "invalid_image", true),
            (404, "unknown", false),
            (500, "not_found", false),
        ] {
            let mut fake = Fake {
                replies: [reply(status, serde_json::json!({"error":code}))].into(),
                requests: vec![],
            };
            assert_eq!(
                matches!(
                    run(&mut fake, Job::Finalize(ticket()), || 1_000_000),
                    Outcome::Failed(_)
                ),
                terminal
            );
        }
    }
    #[test]
    #[ignore = "Explicitly sends a generated 2x2 PNG to the public production service"]
    fn real_service_synthetic_upload() {
        struct ObservedTransport;
        impl Transport for ObservedTransport {
            fn send(&mut self, request: Request) -> Result<Response, ()> {
                let stage = if request.method == "PUT" {
                    "blob"
                } else if request.url.ends_with("/prepare") {
                    "prepare"
                } else if request.url.ends_with("/token") {
                    "token"
                } else {
                    "finalize"
                };
                let response = crate::native::HttpTransport.send(request);
                match &response {
                    Ok(response) => {
                        let code = response_json(response)
                            .and_then(|body| {
                                body.get("error").and_then(Value::as_str).map(str::to_owned)
                            })
                            .filter(|code| {
                                matches!(
                                    code.as_str(),
                                    "forbidden"
                                        | "not_found"
                                        | "invalid_image"
                                        | "too_large"
                                        | "rate_limited"
                                        | "storage_unavailable"
                                        | "internal_error"
                                )
                            })
                            .unwrap_or_else(|| "unreported".into());
                        println!(
                            "SYNTHETIC_STAGE={stage} STATUS={} CODE={code}",
                            response.status
                        );
                    }
                    Err(()) => println!("SYNTHETIC_STAGE={stage} TRANSPORT_FAILED"),
                }
                response
            }
        }

        let bytes = base64::engine::general_purpose::STANDARD.decode("iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAIAAAD91JpzAAAAEElEQVR4nGMw7rgLRAwQCgAqBgZhF+QOcgAAAABJRU5ErkJggg==").unwrap();
        let image = uploadimg_core::Image::new(bytes, [1; 32], 2, 2).unwrap();
        let mut outcome = run(&mut ObservedTransport, Job::Image(image), wall_millis);
        if let Outcome::Recover(ticket) = outcome {
            outcome = run(&mut ObservedTransport, Job::Finalize(ticket), wall_millis);
        }
        match outcome {
            Outcome::Ready(result) => println!("SYNTHETIC_PUBLIC_URL={}", result.url),
            Outcome::Failed(message) => panic!("Synthetic service validation failed: {message}"),
            Outcome::Recover(_) => {
                panic!("Synthetic service validation is unconfirmed; no new image was sent")
            }
        }
    }
}
