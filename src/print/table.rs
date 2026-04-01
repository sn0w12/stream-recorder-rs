use colored::*;
use std::collections::HashMap;

/// Characters used to draw table borders and joints. Placing them in one
/// data structure makes swapping styles (ASCII, Unicode, etc.) easy.
pub struct TableStyle {
    pub top_left: &'static str,
    pub top_right: &'static str,
    pub bottom_left: &'static str,
    pub bottom_right: &'static str,
    pub horiz: &'static str,
    pub vert: &'static str,
    pub top_joint: &'static str,
    pub mid_left: &'static str,
    pub mid_right: &'static str,
    pub mid_joint: &'static str,
    pub bottom_joint: &'static str,
}

impl TableStyle {
    pub fn unicode() -> Self {
        TableStyle {
            top_left: "┌",
            top_right: "┐",
            bottom_left: "└",
            bottom_right: "┘",
            horiz: "─",
            vert: "│",
            top_joint: "┬",
            mid_left: "├",
            mid_right: "┤",
            mid_joint: "┼",
            bottom_joint: "┴",
        }
    }
}

/// Truncation mode used when a column has a maximum width.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Trunc {
    #[default]
    End,
    Start,
    Middle,
    NewLine,
}

/// Alignment used for section labels inside a table.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SectionAlign {
    #[default]
    Center,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug)]
struct ColumnStyle {
    color: Option<Color>,
    max_width: Option<usize>,
    truncation: Trunc,
}

impl Default for ColumnStyle {
    fn default() -> Self {
        Self {
            color: None,
            max_width: None,
            truncation: Trunc::End,
        }
    }
}

/// A table cell with optional color.
pub struct Cell {
    content: String,
    color: Option<Color>,
    truncation: Option<Trunc>,
}

struct PreparedCell {
    lines: Vec<String>,
    color: Option<Color>,
    is_header: bool,
}

enum TableRow {
    Cells(Vec<Cell>),
    Section(SectionRow),
}

enum PreparedRow {
    Cells(Vec<PreparedCell>),
    Section(SectionRow),
}

#[derive(Clone, Debug)]
struct SectionRow {
    title: String,
    align: SectionAlign,
}

pub struct SectionBuilder<'a> {
    table: &'a mut Table,
    row_index: usize,
}

impl Cell {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            color: None,
            truncation: None,
        }
    }

    #[must_use]
    /// Set a color for this cell, overriding any column default.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    #[must_use]
    /// Set a truncation mode for this cell, overriding any column default.
    pub fn truncate(mut self, truncation: Trunc) -> Self {
        self.truncation = Some(truncation);
        self
    }
}

/// A table with optional headers, automatically sized columns, and colors.
pub struct Table {
    headers: Vec<Cell>,
    rows: Vec<TableRow>,
    columns: HashMap<usize, ColumnStyle>,
    style: TableStyle,
}

impl Table {
    pub fn new() -> Self {
        Self {
            headers: Vec::new(),
            rows: Vec::new(),
            columns: HashMap::new(),
            style: TableStyle::unicode(),
        }
    }

    /// Set the header row.
    pub fn set_headers(&mut self, headers: Vec<Cell>) {
        self.headers = headers;
    }

    /// Add a data row.
    pub fn add_row(&mut self, row: Vec<Cell>) {
        self.rows.push(TableRow::Cells(row));
    }

    /// Add a full-width section separator inside the table.
    pub fn add_section(&mut self, title: impl Into<String>) -> SectionBuilder<'_> {
        let row_index = self.rows.len();
        self.rows.push(TableRow::Section(SectionRow {
            title: title.into(),
            align: SectionAlign::Center,
        }));

        SectionBuilder {
            table: self,
            row_index,
        }
    }

    /// Set a default color for a column (0‑based index).
    #[allow(dead_code)]
    pub fn set_column_color(&mut self, col: usize, color: Color) {
        self.column_style_mut(col).color = Some(color);
    }

    /// Limit the visible width of a column in characters.
    #[allow(dead_code)]
    pub fn set_column_max_width(&mut self, col: usize, max_width: usize) {
        self.column_style_mut(col).max_width = Some(max_width);
    }

    /// Set the truncation mode used by a column when a maximum width is set.
    #[allow(dead_code)]
    pub fn set_column_truncation(&mut self, col: usize, truncation: Trunc) {
        self.column_style_mut(col).truncation = truncation;
    }

    /// Print the formatted table.
    pub fn print(&self) {
        for line in self.render_lines() {
            println!("{line}");
        }
    }

    /// Render the formatted table as a single string.
    pub fn render(&self) -> String {
        self.render_lines().join("\n")
    }

    fn column_style_mut(&mut self, col: usize) -> &mut ColumnStyle {
        self.columns.entry(col).or_default()
    }

    fn column_style(&self, col: usize) -> ColumnStyle {
        self.columns.get(&col).copied().unwrap_or_default()
    }

    fn prepare_cell(&self, cell: Option<&Cell>, col: usize, is_header: bool) -> PreparedCell {
        let raw = cell.map(|c| c.content.as_str()).unwrap_or("");
        let column_style = self.column_style(col);
        let truncation = cell
            .and_then(|c| c.truncation)
            .unwrap_or(column_style.truncation);

        let lines = split_lines(raw)
            .into_iter()
            .flat_map(|line| layout_line(line, column_style.max_width, truncation))
            .collect();

        PreparedCell {
            lines,
            color: cell.and_then(|c| c.color).or(column_style.color),
            is_header,
        }
    }

    fn prepare_row(&self, row: &[Cell], col_count: usize, is_header: bool) -> Vec<PreparedCell> {
        (0..col_count)
            .map(|col| self.prepare_cell(row.get(col), col, is_header))
            .collect()
    }

    fn render_row_line(
        &self,
        row: &[PreparedCell],
        line_idx: usize,
        col_widths: &[usize],
    ) -> String {
        let vertical = self.style.vert;
        let rendered_cells: Vec<String> = row
            .iter()
            .enumerate()
            .map(|(col, cell)| {
                let raw = cell.lines.get(line_idx).map(String::as_str).unwrap_or("");
                let padding = col_widths[col].saturating_sub(raw.chars().count());
                format!(
                    "{}{}",
                    style_text(raw, cell.color, cell.is_header),
                    " ".repeat(padding)
                )
            })
            .collect();

        format!(
            "{} {} {}",
            vertical,
            rendered_cells.join(&format!(" {} ", vertical)),
            vertical
        )
    }

    fn render_section_line(&self, section: &SectionRow, col_widths: &[usize]) -> String {
        let total_inner_width = col_widths.iter().sum::<usize>() + (3 * col_widths.len()) - 1;
        let label = truncate_line(
            &format!(" {} ", section.title),
            Some(total_inner_width),
            Trunc::End,
        );
        let remaining = total_inner_width.saturating_sub(label.chars().count());
        let (left_fill, right_fill) = match section.align {
            SectionAlign::Left => (1, remaining - 1),
            SectionAlign::Center => (remaining / 2, remaining - (remaining / 2)),
            SectionAlign::Right => (remaining - 1, 1),
        };

        format!(
            "{}{}{}{}{}",
            self.style.mid_left,
            self.style.horiz.repeat(left_fill),
            style_text(&label, None, true),
            self.style.horiz.repeat(right_fill),
            self.style.mid_right
        )
    }

    fn render_lines(&self) -> Vec<String> {
        // Determine total number of columns = max(header len, any row len)
        let header_len = self.headers.len();
        let max_row_len = self
            .rows
            .iter()
            .filter_map(|row| match row {
                TableRow::Cells(cells) => Some(cells.len()),
                TableRow::Section(_) => None,
            })
            .max()
            .unwrap_or(0);
        let col_count = header_len.max(max_row_len);
        if col_count == 0 {
            return Vec::new();
        }

        let prepared_header =
            (!self.headers.is_empty()).then(|| self.prepare_row(&self.headers, col_count, true));
        let prepared_rows: Vec<PreparedRow> = self
            .rows
            .iter()
            .map(|row| match row {
                TableRow::Cells(cells) => {
                    PreparedRow::Cells(self.prepare_row(cells, col_count, false))
                }
                TableRow::Section(section) => PreparedRow::Section(section.clone()),
            })
            .collect();

        // Compute column widths based on the already-truncated visible content.
        let mut col_widths = vec![0; col_count];
        for row in prepared_header.iter() {
            for (i, cell) in row.iter().enumerate() {
                for line in &cell.lines {
                    let width = line.chars().count();
                    if width > col_widths[i] {
                        col_widths[i] = width;
                    }
                }
            }
        }
        for row in &prepared_rows {
            let PreparedRow::Cells(cells) = row else {
                continue;
            };

            for (i, cell) in cells.iter().enumerate() {
                for line in &cell.lines {
                    let width = line.chars().count();
                    if width > col_widths[i] {
                        col_widths[i] = width;
                    }
                }
            }
        }

        // Short aliases for style characters
        let s = &self.style;
        let h = s.horiz;

        // Build repeated-horiz pieces for each column
        let col_pieces: Vec<String> = col_widths.iter().map(|&w| h.repeat(w)).collect();

        let mut lines = Vec::new();

        // Print top border
        let top_join = format!("{}{}{}", h, s.top_joint, h);
        let top_inner = col_pieces.join(&top_join);
        lines.push(format!(
            "{}{}{}{}{}",
            s.top_left, h, top_inner, h, s.top_right
        ));

        // Print headers (if any)
        if let Some(header) = prepared_header.as_ref() {
            let header_height = header
                .iter()
                .map(|cell| cell.lines.len())
                .max()
                .unwrap_or(1);
            for line_idx in 0..header_height {
                lines.push(self.render_row_line(header, line_idx, &col_widths));
            }

            if prepared_rows.is_empty()
                || !matches!(prepared_rows.first(), Some(PreparedRow::Section(_)))
            {
                let mid_join = format!("{}{}{}", h, s.mid_joint, h);
                let mid_inner = col_pieces.join(&mid_join);
                lines.push(format!(
                    "{}{}{}{}{}",
                    s.mid_left, h, mid_inner, h, s.mid_right
                ));
            }
        }

        // Print data rows
        for row in &prepared_rows {
            match row {
                PreparedRow::Cells(cells) => {
                    let row_height = cells.iter().map(|cell| cell.lines.len()).max().unwrap_or(1);
                    for line_idx in 0..row_height {
                        lines.push(self.render_row_line(cells, line_idx, &col_widths));
                    }
                }
                PreparedRow::Section(section) => {
                    lines.push(self.render_section_line(section, &col_widths))
                }
            }
        }

        // Bottom border
        let bottom_join = format!("{}{}{}", h, s.bottom_joint, h);
        let bottom_inner = col_pieces.join(&bottom_join);
        lines.push(format!(
            "{}{}{}{}{}",
            s.bottom_left, h, bottom_inner, h, s.bottom_right
        ));

        lines
    }
}

impl<'a> SectionBuilder<'a> {
    pub fn align(self, align: SectionAlign) -> Self {
        if let Some(TableRow::Section(section)) = self.table.rows.get_mut(self.row_index) {
            section.align = align;
        }

        self
    }
}

fn split_lines(content: &str) -> Vec<&str> {
    if content.is_empty() {
        return vec![""];
    }

    content
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .collect()
}

fn layout_line(content: &str, max_width: Option<usize>, truncation: Trunc) -> Vec<String> {
    match truncation {
        Trunc::NewLine => wrap_line(content, max_width),
        _ => vec![truncate_line(content, max_width, truncation)],
    }
}

fn truncate_line(content: &str, max_width: Option<usize>, truncation: Trunc) -> String {
    const ELLIPSIS: char = '…';

    let Some(max_width) = max_width else {
        return content.to_string();
    };

    let chars: Vec<char> = content.chars().collect();
    if chars.len() <= max_width {
        return content.to_string();
    }

    match max_width {
        0 => String::new(),
        1 => ELLIPSIS.to_string(),
        _ => {
            let keep = max_width - 1;
            match truncation {
                Trunc::End => chars
                    .iter()
                    .take(keep)
                    .copied()
                    .chain(std::iter::once(ELLIPSIS))
                    .collect(),
                Trunc::Start => std::iter::once(ELLIPSIS)
                    .chain(chars.iter().skip(chars.len() - keep).copied())
                    .collect(),
                Trunc::Middle => {
                    let left = keep.div_ceil(2);
                    let right = keep / 2;

                    chars
                        .iter()
                        .take(left)
                        .copied()
                        .chain(std::iter::once(ELLIPSIS))
                        .chain(chars.iter().skip(chars.len() - right).copied())
                        .collect()
                }
                Trunc::NewLine => content.to_string(),
            }
        }
    }
}

fn wrap_line(content: &str, max_width: Option<usize>) -> Vec<String> {
    let Some(max_width) = max_width else {
        return vec![content.to_string()];
    };

    if max_width == 0 {
        return vec![String::new()];
    }

    let chars: Vec<char> = content.chars().collect();
    if chars.is_empty() {
        return vec![String::new()];
    }

    let mut lines = Vec::new();
    let mut start = 0;

    while start < chars.len() {
        let remaining = chars.len() - start;
        if remaining <= max_width {
            lines.push(chars[start..].iter().collect());
            break;
        }

        let end = start + max_width;
        let wrap_at = chars[start..end]
            .iter()
            .enumerate()
            .rev()
            .find(|(idx, ch)| *idx > 0 && ch.is_whitespace())
            .map(|(idx, _)| start + idx);

        if let Some(split_at) = wrap_at {
            lines.push(chars[start..split_at].iter().collect());
            start = split_at;
            while start < chars.len() && chars[start].is_whitespace() {
                start += 1;
            }
        } else {
            lines.push(chars[start..end].iter().collect());
            start = end;
        }
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

fn style_text(content: &str, color: Option<Color>, is_header: bool) -> String {
    if content.is_empty() {
        return String::new();
    }

    match (color, is_header) {
        (Some(color), true) => content.color(color).bold().to_string(),
        (Some(color), false) => content.color(color).to_string(),
        (None, true) => content.bold().to_string(),
        (None, false) => content.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use colored::Color::BrightBlack;

    #[test]
    fn cell_builders_are_chainable() {
        let cell = Cell::new("value")
            .color(BrightBlack)
            .truncate(Trunc::Middle);

        assert_eq!(cell.color, Some(BrightBlack));
        assert_eq!(cell.truncation, Some(Trunc::Middle));
    }

    #[test]
    fn renders_multiline_headers_and_rows() {
        let mut table = Table::new();
        table.set_headers(vec![Cell::new("Name\nAlias"), Cell::new("Value")]);
        table.add_row(vec![Cell::new("alpha\nbeta"), Cell::new("1")]);

        assert_eq!(
            plain_lines(&table),
            vec![
                "┌───────┬───────┐",
                "│ Name  │ Value │",
                "│ Alias │       │",
                "├───────┼───────┤",
                "│ alpha │ 1     │",
                "│ beta  │       │",
                "└───────┴───────┘",
            ]
        );
    }

    #[test]
    fn renders_center_aligned_sections_inside_a_single_table() {
        assert_eq!(
            section_table_lines(SectionAlign::Center),
            expected_section_lines("├─── Alpha ────┤")
        );
    }

    #[test]
    fn renders_left_aligned_sections_inside_a_single_table() {
        assert_eq!(
            section_table_lines(SectionAlign::Left),
            expected_section_lines("├─ Alpha ──────┤")
        );
    }

    #[test]
    fn renders_right_aligned_sections_inside_a_single_table() {
        assert_eq!(
            section_table_lines(SectionAlign::Right),
            expected_section_lines("├────── Alpha ─┤")
        );
    }

    #[test]
    fn applies_column_and_cell_truncation() {
        let mut table = Table::new();
        table.set_headers(vec![Cell::new("Value"), Cell::new("Other")]);
        table.set_column_max_width(0, 5);
        table.set_column_truncation(0, Trunc::Start);
        table.add_row(vec![Cell::new("abcdefghij"), Cell::new("z")]);
        table.add_row(vec![
            Cell::new("abcdefghij").truncate(Trunc::Middle),
            Cell::new("z"),
        ]);

        assert_eq!(
            plain_lines(&table),
            vec![
                "┌───────┬───────┐",
                "│ Value │ Other │",
                "├───────┼───────┤",
                "│ …ghij │ z     │",
                "│ ab…ij │ z     │",
                "└───────┴───────┘",
            ]
        );
    }

    #[test]
    fn newline_truncation_wraps_at_spaces_and_hard_breaks_when_needed() {
        let mut table = Table::new();
        table.set_headers(vec![Cell::new("Value")]);
        table.set_column_max_width(0, 8);
        table.add_row(vec![Cell::new("one two three").truncate(Trunc::NewLine)]);
        table.add_row(vec![Cell::new("abcdefghij").truncate(Trunc::NewLine)]);

        assert_eq!(
            plain_lines(&table),
            vec![
                "┌──────────┐",
                "│ Value    │",
                "├──────────┤",
                "│ one two  │",
                "│ three    │",
                "│ abcdefgh │",
                "│ ij       │",
                "└──────────┘",
            ]
        );
    }

    fn plain_lines(table: &Table) -> Vec<String> {
        table
            .render_lines()
            .into_iter()
            .map(|line| strip_ansi(&line))
            .collect()
    }

    fn section_table_lines(align: SectionAlign) -> Vec<String> {
        let mut table = Table::new();
        table.set_headers(vec![Cell::new("Name"), Cell::new("Value")]);
        table.add_section("Alpha").align(align);
        table.add_row(vec![Cell::new("a"), Cell::new("1")]);

        plain_lines(&table)
    }

    fn expected_section_lines(section_line: &str) -> Vec<String> {
        vec![
            "┌──────┬───────┐".to_string(),
            "│ Name │ Value │".to_string(),
            section_line.to_string(),
            "│ a    │ 1     │".to_string(),
            "└──────┴───────┘".to_string(),
        ]
    }

    fn strip_ansi(input: &str) -> String {
        let mut plain = String::new();
        let mut chars = input.chars();

        while let Some(ch) = chars.next() {
            if ch == '\u{1b}' {
                for next in chars.by_ref() {
                    if next == 'm' {
                        break;
                    }
                }
                continue;
            }

            plain.push(ch);
        }

        plain
    }
}
