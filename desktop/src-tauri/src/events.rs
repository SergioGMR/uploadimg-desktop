use std::collections::VecDeque;

pub struct ImageEvent {
    pub generation: u64,
    pub lifecycle: u64,
    pub source: i32,
    pub width: u32,
    pub height: u32,
    pub original: Vec<u8>,
    pub thumbnail: Vec<u8>,
    pub digest: [u8; 32],
}
pub enum Event {
    Image(ImageEvent),
    Control {
        kind: i32,
        generation: u64,
        lifecycle: u64,
        code: i32,
        text: String,
    },
}
#[derive(Default)]
pub struct EventQueue {
    events: VecDeque<Event>,
}
impl EventQueue {
    pub fn push(&mut self, event: Event) -> bool {
        match &event {
            Event::Image(image) => {
                if !(0..=2).contains(&image.source)
                    || image.original.is_empty()
                    || image.original.len() > 10_000_000
                    || image.width == 0
                    || image.height == 0
                    || image.width > 8192
                    || image.height > 8192
                    || u64::from(image.width) * u64::from(image.height) > 16_000_000
                    || !valid_thumbnail(&image.thumbnail)
                {
                    return false;
                }
                let images: Vec<_> = self
                    .events
                    .iter()
                    .filter_map(|e| match e {
                        Event::Image(i) => Some(i),
                        _ => None,
                    })
                    .collect();
                if images.len() == 3
                    || images.iter().map(|i| i.original.len()).sum::<usize>() + image.original.len()
                        > 30_000_000
                {
                    return false;
                }
            }
            Event::Control { kind, text, .. } => {
                if !(2..=6).contains(kind) || text.len() > 128 {
                    return false;
                }
                if self
                    .events
                    .iter()
                    .filter(|e| matches!(e, Event::Control { .. }))
                    .count()
                    == 32
                {
                    if let Some(index) = self
                        .events
                        .iter()
                        .position(|e| matches!(e, Event::Control { .. }))
                    {
                        self.events.remove(index);
                    }
                }
            }
        }
        self.events.push_back(event);
        true
    }
    pub fn drain(&mut self) -> Vec<Event> {
        self.events.drain(..).collect()
    }
    pub fn invalidate_images(&mut self) {
        self.events.retain(|e| !matches!(e, Event::Image(_)));
    }
}
pub fn valid_thumbnail(bytes: &[u8]) -> bool {
    if bytes.len() < 24
        || bytes.len() > 1_100_000
        || &bytes[..16] != b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR"
    {
        return false;
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
    width > 0 && height > 0 && width <= 512 && height <= 512
}

#[cfg(test)]
mod tests {
    use super::*;
    fn thumbnail(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        bytes.extend(width.to_be_bytes());
        bytes.extend(height.to_be_bytes());
        bytes
    }
    fn image(generation: u64, bytes: usize) -> Event {
        Event::Image(ImageEvent {
            generation,
            lifecycle: 0,
            source: 1,
            width: 2,
            height: 2,
            original: vec![7; bytes],
            thumbnail: thumbnail(2, 2),
            digest: [7; 32],
        })
    }
    #[test]
    fn bounded_callback_queue_preserves_fifo_and_rejects_fourth_image() {
        let mut queue = EventQueue::default();
        for n in 0..3 {
            assert!(queue.push(image(n, 10_000_000)));
        }
        assert!(!queue.push(image(3, 1)));
        let events = queue.drain();
        assert_eq!(events.len(), 3);
        for (index, event) in events.iter().enumerate() {
            let Event::Image(event) = event else {
                panic!("image FIFO")
            };
            assert_eq!(event.generation, index as u64);
        }
        assert!(queue.push(image(4, 10_000_000)));
    }
    #[test]
    fn stop_invalidation_drops_queued_pixels_but_preserves_explicit_actions() {
        let mut queue = EventQueue::default();
        assert!(queue.push(image(0, 3)));
        assert!(queue.push(Event::Control {
            kind: 6,
            generation: 0,
            lifecycle: 0,
            code: 2,
            text: "known-id".into()
        }));
        queue.invalidate_images();
        let events = queue.drain();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            Event::Control {
                kind: 6,
                code: 2,
                ..
            }
        ));
    }
    #[test]
    fn callback_thumbnails_are_small_png_headers_not_arbitrary_originals() {
        assert!(valid_thumbnail(&thumbnail(512, 256)));
        for bytes in [
            thumbnail(513, 1),
            thumbnail(1, 0),
            vec![7; 20],
            vec![0; 1_100_001],
        ] {
            assert!(!valid_thumbnail(&bytes));
        }
        let mut queue = EventQueue::default();
        assert!(!queue.push(image(0, 10_000_001)));
    }
    #[test]
    fn metadata_queue_remains_bounded_without_webview_polling() {
        let mut queue = EventQueue::default();
        for n in 0..100 {
            assert!(queue.push(Event::Control {
                kind: 2,
                generation: n,
                lifecycle: 0,
                code: 1,
                text: String::new()
            }));
        }
        assert_eq!(queue.drain().len(), 32);
    }
}
