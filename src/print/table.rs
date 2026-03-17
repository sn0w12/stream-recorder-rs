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

/// A table cell with optional color.
pub struct Cell {
    content: String,
    color: Option<Color>,
}

impl Cell {
    pub fn new(content: impl Into<String>, color: Option<Color>) -> Self {
        Cell {
            content: content.into(),
            color,
        }
    }
}

/// A table with optional headers, automatically sized columns, and colors.
pub struct Table {
    headers: Vec<Cell>,
    rows: Vec<Vec<Cell>>,
    col_colors: HashMap<usize, Color>,
    style: TableStyle,
}

impl Table {
    pub fn new() -> Self {
        Table {
            headers: Vec::new(),
            rows: Vec::new(),
            col_colors: HashMap::new(),
            style: TableStyle::unicode(),
        }
    }

    /// Set the header row.
    pub fn set_headers(&mut self, headers: Vec<Cell>) {
        self.headers = headers;
    }

    /// Add a data row.
    pub fn add_row(&mut self, row: Vec<Cell>) {
        self.rows.push(row);
    }

    /// Set a default color for a column (0‑based index).
    #[allow(dead_code)]
    pub fn set_column_color(&mut self, col: usize, color: Color) {
        self.col_colors.insert(col, color);
    }

    /// Print the formatted table.
    pub fn print(&self) {
        // Determine total number of columns = max(header len, any row len)
        let header_len = self.headers.len();
        let max_row_len = self.rows.iter().map(|row| row.len()).max().unwrap_or(0);
        let col_count = header_len.max(max_row_len);
        if col_count == 0 {
            return;
        }

        // Collect all rows plus header into a single iterable for width calculation
        let all_rows = std::iter::once(&self.headers).chain(self.rows.iter());

        // Compute column widths based on raw content length (ignoring color codes)
        let mut col_widths = vec![0; col_count];
        for row in all_rows {
            for (i, cell) in row.iter().enumerate() {
                let width = cell.content.chars().count(); // Unicode‑aware width
                if width > col_widths[i] {
                    col_widths[i] = width;
                }
            }
        }

        // Helper to format a single cell with color and padding
        let format_cell = |cell: Option<&Cell>, col: usize, is_header: bool| -> String {
            let raw = cell.map(|c| c.content.as_str()).unwrap_or("");

            // Determine color: cell color > column color
            let color = cell
                .and_then(|c| c.color)
                .or_else(|| self.col_colors.get(&col).copied());

            let styled = if let Some(c) = color {
                if is_header {
                    raw.color(c).bold().to_string()
                } else {
                    raw.color(c).to_string()
                }
            } else if is_header {
                raw.bold().to_string()
            } else {
                raw.to_string()
            };

            // Pad to column width
            let raw_len = raw.chars().count();
            let padding = col_widths[col] - raw_len;
            format!("{}{}", styled, " ".repeat(padding))
        };

        // Short aliases for style characters
        let s = &self.style;
        let h = s.horiz;
        let v = s.vert;

        // Build repeated-horiz pieces for each column
        let col_pieces: Vec<String> = col_widths.iter().map(|&w| h.repeat(w)).collect();

        // Print top border
        let top_join = format!("{}{}{}", h, s.top_joint, h);
        let top_inner = col_pieces.join(&top_join);
        println!("{}{}{}{}{}", s.top_left, h, top_inner, h, s.top_right);

        // Print headers (if any)
        if !self.headers.is_empty() {
            let header_cells: Vec<String> = (0..col_count)
                .map(|i| format_cell(self.headers.get(i), i, true))
                .collect();
            println!("{} {} {}", v, header_cells.join(&format!(" {} ", v)), v);

            // Separator after header
            let mid_join = format!("{}{}{}", h, s.mid_joint, h);
            let mid_inner = col_pieces.join(&mid_join);
            println!("{}{}{}{}{}", s.mid_left, h, mid_inner, h, s.mid_right);
        }

        // Print data rows
        for row in &self.rows {
            let row_cells: Vec<String> = (0..col_count)
                .map(|i| format_cell(row.get(i), i, false))
                .collect();
            println!("{} {} {}", v, row_cells.join(&format!(" {} ", v)), v);
        }

        // Bottom border
        let bottom_join = format!("{}{}{}", h, s.bottom_joint, h);
        let bottom_inner = col_pieces.join(&bottom_join);
        println!(
            "{}{}{}{}{}",
            s.bottom_left, h, bottom_inner, h, s.bottom_right
        );
    }
}
