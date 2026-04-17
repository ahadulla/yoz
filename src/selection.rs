#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pos {
    pub row: usize,
    pub col: usize,
}

impl Pos {
    pub fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }
}

impl PartialOrd for Pos {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Pos {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.row.cmp(&other.row).then(self.col.cmp(&other.col))
    }
}

#[derive(Debug, Clone)]
pub struct Selection {
    pub anchor: Option<Pos>,
}

impl Selection {
    pub fn new() -> Self {
        Self { anchor: None }
    }

    pub fn is_active(&self) -> bool {
        self.anchor.is_some()
    }

    pub fn start_at(&mut self, pos: Pos) {
        self.anchor = Some(pos);
    }

    pub fn clear(&mut self) {
        self.anchor = None;
    }

    pub fn range(&self, cursor: Pos) -> Option<(Pos, Pos)> {
        let anchor = self.anchor?;
        if anchor == cursor {
            return None;
        }
        Some(if anchor < cursor {
            (anchor, cursor)
        } else {
            (cursor, anchor)
        })
    }

    pub fn contains(&self, cursor: Pos, row: usize, col: usize) -> bool {
        let Some((start, end)) = self.range(cursor) else {
            return false;
        };
        let p = Pos::new(row, col);
        p >= start && p < end
    }
}
