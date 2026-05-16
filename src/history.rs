use chrono::Local;

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub timestamp: String,
    pub text: String,
}

impl HistoryEntry {
    pub fn new(text: String) -> Self {
        Self {
            timestamp: Local::now().format("%H:%M:%S").to_string(),
            text,
        }
    }

    pub fn preview(&self, max_len: usize) -> String {
        if self.text.len() <= max_len {
            self.text.clone()
        } else {
            format!("{}...", &self.text[..max_len])
        }
    }
}

#[derive(Debug, Clone)]
pub struct History {
    entries: Vec<HistoryEntry>,
    max_entries: usize,
}

impl History {
    pub fn new() -> Self {
        Self::with_capacity(100)
    }

    pub fn with_capacity(max: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_entries: max,
        }
    }

    pub fn add(&mut self, text: String) {
        let entry = HistoryEntry::new(text);
        self.entries.insert(0, entry);
        if self.entries.len() > self.max_entries {
            self.entries.pop();
        }
    }

    pub fn entries(&self) -> &[HistoryEntry] {
        &self.entries
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    #[allow(dead_code)]
    pub fn remove(&mut self, index: usize) {
        if index < self.entries.len() {
            self.entries.remove(index);
        }
    }

    #[allow(dead_code)]
    pub fn get(&self, index: usize) -> Option<&HistoryEntry> {
        self.entries.get(index)
    }
}

impl Default for History {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_retrieve() {
        let mut history = History::new();
        history.add("test message".to_string());
        assert_eq!(history.len(), 1);
        assert_eq!(history.get(0).unwrap().text, "test message");
    }

    #[test]
    fn test_max_entries() {
        let mut history = History::with_capacity(3);
        for i in 0..5 {
            history.add(format!("msg {}", i));
        }
        assert_eq!(history.len(), 3);
        assert_eq!(history.get(0).unwrap().text, "msg 4");
        assert_eq!(history.get(2).unwrap().text, "msg 2");
    }

    #[test]
    fn test_clear() {
        let mut history = History::new();
        history.add("hello".to_string());
        history.clear();
        assert!(history.is_empty());
    }

    #[test]
    fn test_preview() {
        let entry = HistoryEntry::new("a".repeat(100));
        assert_eq!(entry.preview(10).len(), 13);
        assert_eq!(entry.preview(200).len(), 100);
    }

    #[test]
    fn test_remove() {
        let mut history = History::with_capacity(5);
        history.add("first".to_string());
        history.add("second".to_string());
        history.add("third".to_string());
        assert_eq!(history.len(), 3);
        history.remove(1);
        assert_eq!(history.len(), 2);
        assert_eq!(history.get(1).unwrap().text, "first");
    }

    #[test]
    fn test_get_out_of_bounds() {
        let history = History::new();
        assert!(history.get(0).is_none());
        assert!(history.get(100).is_none());
    }
}
