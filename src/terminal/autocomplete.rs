use super::history::CommandHistory;

pub struct AutocompleteState {
    input_buffer: String,
    ghost: Option<String>,
    pub history_search: HistorySearchState,
}

#[derive(Debug)]
pub enum HistorySearchState {
    Closed,
    Open {
        query: String,
        results: Vec<String>,
        selected: usize,
    },
}

impl AutocompleteState {
    pub fn new() -> Self {
        Self {
            input_buffer: String::new(),
            ghost: None,
            history_search: HistorySearchState::Closed,
        }
    }

    pub fn on_char(&mut self, ch: char, history: &CommandHistory) {
        self.input_buffer.push(ch);
        self.update_ghost(history);
    }

    pub fn on_backspace(&mut self, history: &CommandHistory) {
        self.input_buffer.pop();
        self.update_ghost(history);
    }

    pub fn on_enter(&mut self, history: &mut CommandHistory) {
        let cmd = std::mem::take(&mut self.input_buffer);
        self.ghost = None;
        if !cmd.trim().is_empty() {
            history.add(cmd);
        }
    }

    pub fn on_control_reset(&mut self) {
        self.input_buffer.clear();
        self.ghost = None;
    }

    pub fn ghost_text(&self) -> Option<&str> {
        self.ghost.as_deref()
    }

    /// Returns the full ghost suggestion to write to the PTY when accepted.
    pub fn accept_ghost(&mut self) -> Option<String> {
        let text = self.ghost.take()?;
        self.input_buffer.push_str(&text);
        Some(text)
    }

    fn update_ghost(&mut self, history: &CommandHistory) {
        if self.input_buffer.is_empty() {
            self.ghost = None;
            return;
        }

        let matches = history.prefix_search(&self.input_buffer);
        self.ghost = matches
            .first()
            .and_then(|m| {
                if m.len() > self.input_buffer.len() {
                    Some(m[self.input_buffer.len()..].to_string())
                } else {
                    None
                }
            });
    }

    // --- History search (Ctrl+R) ---

    pub fn open_history_search(&mut self, history: &CommandHistory) {
        let results = history.substring_search("").iter().map(|s| s.to_string()).collect();
        self.history_search = HistorySearchState::Open {
            query: String::new(),
            results,
            selected: 0,
        };
    }

    pub fn close_history_search(&mut self) {
        self.history_search = HistorySearchState::Closed;
    }

    pub fn history_search_is_open(&self) -> bool {
        matches!(self.history_search, HistorySearchState::Open { .. })
    }

    pub fn history_search_char(&mut self, ch: char, history: &CommandHistory) {
        if let HistorySearchState::Open {
            query,
            results,
            selected,
        } = &mut self.history_search
        {
            query.push(ch);
            *results = history.substring_search(query).iter().map(|s| s.to_string()).collect();
            *selected = 0;
        }
    }

    pub fn history_search_backspace(&mut self, history: &CommandHistory) {
        if let HistorySearchState::Open {
            query,
            results,
            selected,
        } = &mut self.history_search
        {
            query.pop();
            *results = history.substring_search(query).iter().map(|s| s.to_string()).collect();
            *selected = 0;
        }
    }

    pub fn history_search_next(&mut self) {
        if let HistorySearchState::Open {
            results, selected, ..
        } = &mut self.history_search
        {
            if !results.is_empty() {
                *selected = (*selected + 1) % results.len();
            }
        }
    }

    pub fn history_search_prev(&mut self) {
        if let HistorySearchState::Open {
            results, selected, ..
        } = &mut self.history_search
        {
            if !results.is_empty() {
                *selected = if *selected == 0 {
                    results.len() - 1
                } else {
                    *selected - 1
                };
            }
        }
    }

    /// Accept selected history search result. Returns the command to type into the terminal.
    pub fn history_search_accept(&mut self) -> Option<String> {
        if let HistorySearchState::Open {
            results, selected, ..
        } = &self.history_search
        {
            let cmd = results.get(*selected).cloned();
            self.history_search = HistorySearchState::Closed;
            if let Some(ref c) = cmd {
                self.input_buffer = c.clone();
                self.ghost = None;
            }
            cmd
        } else {
            None
        }
    }

    pub fn history_search_query(&self) -> &str {
        match &self.history_search {
            HistorySearchState::Open { query, .. } => query,
            HistorySearchState::Closed => "",
        }
    }

    pub fn history_search_results(&self) -> &[String] {
        match &self.history_search {
            HistorySearchState::Open { results, .. } => results,
            HistorySearchState::Closed => &[],
        }
    }

    pub fn history_search_selected(&self) -> usize {
        match &self.history_search {
            HistorySearchState::Open { selected, .. } => *selected,
            HistorySearchState::Closed => 0,
        }
    }
}
