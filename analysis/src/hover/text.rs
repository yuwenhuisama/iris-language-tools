pub(super) struct BoundedText {
    text: String,
    limit: usize,
    truncated: bool,
}

impl BoundedText {
    pub(super) const fn full(&self) -> bool {
        self.truncated
    }

    pub(super) const fn new(limit: usize) -> Self {
        Self {
            text: String::new(),
            limit,
            truncated: false,
        }
    }

    pub(super) fn push(&mut self, value: &str) {
        if self.truncated {
            return;
        }
        let available = self.limit.saturating_sub(self.text.len());
        if value.len() <= available {
            self.text.push_str(value);
            return;
        }
        let end = value.floor_char_boundary(available);
        self.text.push_str(&value[..end]);
        self.truncated = true;
    }

    pub(super) fn finish(mut self) -> String {
        const MARKER: &str = "... [truncated]";
        if self.truncated {
            let end = self
                .text
                .floor_char_boundary(self.limit.saturating_sub(MARKER.len()));
            self.text.truncate(end);
            self.text.push_str(MARKER);
        }
        self.text
    }
}
