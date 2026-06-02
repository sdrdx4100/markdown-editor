#[derive(Default, Clone)]
pub struct FindReplaceState {
    pub visible: bool,
    pub show_replace: bool,
    pub query: String,
    pub replace_with: String,
    pub case_sensitive: bool,
    pub current_match: usize,
    pub focus_query: bool,
}

impl FindReplaceState {
    pub fn open_find(&mut self) {
        self.visible = true;
        self.show_replace = false;
        self.focus_query = true;
    }

    pub fn open_replace(&mut self) {
        self.visible = true;
        self.show_replace = true;
        self.focus_query = true;
    }

    pub fn close(&mut self) {
        self.visible = false;
    }
}

pub fn find_all(text: &str, query: &str, case_sensitive: bool) -> Vec<(usize, usize)> {
    if query.is_empty() {
        return Vec::new();
    }
    if case_sensitive {
        find_all_inner(text, query)
    } else {
        let hay = text.to_lowercase();
        let needle = query.to_lowercase();
        find_all_inner(&hay, &needle)
            .into_iter()
            .filter_map(|(s_byte, e_byte)| {
                // Map back to original char positions using lowercase indices.
                // For ASCII queries this is identity; for Unicode the byte offsets in lowercase
                // generally align with the original since most Japanese chars don't case-fold.
                let chars_before = text.char_indices().take_while(|(b, _)| *b < s_byte).count();
                let chars_len = query.chars().count();
                Some((chars_before, chars_before + chars_len))
                    .filter(|(_, e)| *e <= text.chars().count())
                    .map(|_| (s_byte, e_byte))
            })
            .collect()
    }
}

fn find_all_inner(hay: &str, needle: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = 0usize;
    while let Some(pos) = hay[start..].find(needle) {
        let abs = start + pos;
        out.push((abs, abs + needle.len()));
        start = abs + needle.len().max(1);
    }
    out
}

/// Byte ranges → character ranges
pub fn byte_to_char_ranges(text: &str, byte_ranges: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let mut result = Vec::with_capacity(byte_ranges.len());
    let mut iter = text.char_indices().peekable();
    let mut char_idx = 0;
    let mut last_byte = 0;

    let mut byte_to_char_cache = std::collections::HashMap::new();
    byte_to_char_cache.insert(0, 0);
    while let Some((b, _)) = iter.next() {
        char_idx += 1;
        byte_to_char_cache.insert(b + text[b..].chars().next().map(|c| c.len_utf8()).unwrap_or(0), char_idx);
        last_byte = b;
    }
    let _ = last_byte;
    let total_chars = text.chars().count();
    byte_to_char_cache.insert(text.len(), total_chars);

    let mut byte_to_char = |b: usize| -> usize {
        if let Some(&c) = byte_to_char_cache.get(&b) {
            return c;
        }
        text.char_indices()
            .take_while(|(bb, _)| *bb < b)
            .count()
    };

    for (s, e) in byte_ranges {
        result.push((byte_to_char(*s), byte_to_char(*e)));
    }
    result
}

pub fn replace_all(text: &str, query: &str, replacement: &str, case_sensitive: bool) -> (String, usize) {
    if query.is_empty() {
        return (text.to_string(), 0);
    }
    if case_sensitive {
        let count = text.matches(query).count();
        (text.replace(query, replacement), count)
    } else {
        // Manual case-insensitive replacement preserving original bytes outside matches.
        let hay = text.to_lowercase();
        let needle = query.to_lowercase();
        let matches = find_all_inner(&hay, &needle);
        if matches.is_empty() {
            return (text.to_string(), 0);
        }
        let mut out = String::with_capacity(text.len());
        let mut cursor = 0usize;
        let count = matches.len();
        for (s, e) in matches {
            out.push_str(&text[cursor..s]);
            out.push_str(replacement);
            cursor = e;
        }
        out.push_str(&text[cursor..]);
        (out, count)
    }
}
