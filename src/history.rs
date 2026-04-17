#[derive(Debug, Clone)]
pub enum Action {
    InsertChar {
        row: usize,
        col: usize,
        ch: char,
    },
    DeleteChar {
        row: usize,
        col: usize,
        ch: char,
    },
    InsertNewline {
        row: usize,
        col: usize,
    },
    DeleteNewline {
        row: usize,
        col: usize,
    },
}

#[derive(Debug)]
pub struct History {
    undo_stack: Vec<Vec<Action>>,
    redo_stack: Vec<Vec<Action>>,
    current_group: Vec<Action>,
    grouping: bool,
}

impl History {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            current_group: Vec::new(),
            grouping: false,
        }
    }

    pub fn begin_group(&mut self) {
        self.grouping = true;
        self.current_group.clear();
    }

    pub fn end_group(&mut self) {
        if !self.current_group.is_empty() {
            self.undo_stack.push(std::mem::take(&mut self.current_group));
        }
        self.grouping = false;
    }

    pub fn push(&mut self, action: Action) {
        self.redo_stack.clear();
        if self.grouping {
            self.current_group.push(action);
        } else {
            self.undo_stack.push(vec![action]);
        }
    }

    pub fn undo(&mut self) -> Option<Vec<Action>> {
        if self.grouping {
            self.end_group();
        }
        let group = self.undo_stack.pop()?;
        self.redo_stack.push(group.clone());
        Some(group)
    }

    pub fn redo(&mut self) -> Option<Vec<Action>> {
        let group = self.redo_stack.pop()?;
        self.undo_stack.push(group.clone());
        Some(group)
    }
}
