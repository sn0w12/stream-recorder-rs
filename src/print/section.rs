use colored::*;

/// Types of items that can appear in a section.
enum Item {
    Ok { name: String, note: String },
    Err { name: String, note: String },
    Warn { name: String, note: String },
    Dot { name: String, note: String },
    Plain { text: String, color: Option<Color> },
}

/// A section with a title and a list of items.
struct Section {
    title: String,
    items: Vec<Item>,
}

/// Main printer that collects sections and prints them with globally consistent sizing.
pub struct StartupInfo {
    sections: Vec<Section>,
}

impl StartupInfo {
    pub fn new() -> Self {
        StartupInfo {
            sections: Vec::new(),
        }
    }

    fn push_item(&mut self, item: Item) {
        if let Some(section) = self.sections.last_mut() {
            section.items.push(item);
        }
    }

    /// Start a new section with the given title.
    pub fn begin_section(&mut self, title: impl Into<String>) {
        self.sections.push(Section {
            title: title.into(),
            items: Vec::new(),
        });
    }

    /// Add a "✓" item (green).
    pub fn ok(&mut self, name: impl Into<String>, note: impl Into<String>) {
        self.push_item(Item::Ok {
            name: name.into(),
            note: note.into(),
        });
    }

    /// Add a "✗" item (red).
    pub fn err(&mut self, name: impl Into<String>, note: impl Into<String>) {
        self.push_item(Item::Err {
            name: name.into(),
            note: note.into(),
        });
    }

    /// Add a "→" item (yellow).
    pub fn warn(&mut self, name: impl Into<String>, note: impl Into<String>) {
        self.push_item(Item::Warn {
            name: name.into(),
            note: note.into(),
        });
    }

    /// Add a "•" item (plain).
    pub fn dot(&mut self, name: impl Into<String>, note: impl Into<String>) {
        self.push_item(Item::Dot {
            name: name.into(),
            note: note.into(),
        });
    }

    /// Add a plain text line (e.g., "No platforms configured").
    pub fn plain(&mut self, text: impl Into<String>, color: Option<Color>) {
        self.push_item(Item::Plain {
            text: text.into(),
            color,
        });
    }

    /// Print all sections with globally consistent underlines and name alignment.
    pub fn print(&self) {
        if self.sections.is_empty() {
            return;
        }

        let mut global_max_name_len = 0;
        for section in &self.sections {
            for item in &section.items {
                if let Item::Ok { name, .. }
                | Item::Err { name, .. }
                | Item::Warn { name, .. }
                | Item::Dot { name, .. } = item
                {
                    let len = name.chars().count();
                    if len > global_max_name_len {
                        global_max_name_len = len;
                    }
                }
            }
        }

        let mut global_max_line_width = 0;
        for section in &self.sections {
            // Title line (printed before underline) – we don't include it in max width
            for item in &section.items {
                let width = match item {
                    Item::Ok { note, .. }
                    | Item::Err { note, .. }
                    | Item::Warn { note, .. }
                    | Item::Dot { note, .. } => {
                        // 4 = symbol(1) + space(1) + two spaces before note(2)
                        4 + global_max_name_len + note.chars().count()
                    }
                    Item::Plain { text, .. } => {
                        // plain lines are indented by 2 spaces
                        2 + text.chars().count()
                    }
                };
                if width > global_max_line_width {
                    global_max_line_width = width;
                }
            }
        }

        for section in &self.sections {
            // Print section title (bold, no extra spaces)
            println!("{}", section.title.bold());
            // Underline to global max width
            println!("{}", "─".repeat(global_max_line_width));

            // Print items
            for item in &section.items {
                match item {
                    Item::Ok { name, note } => {
                        let name_padded = format!("{:width$}", name, width = global_max_name_len);
                        println!("✓ {}  {}", name_padded.green(), note);
                    }
                    Item::Err { name, note } => {
                        let name_padded = format!("{:width$}", name, width = global_max_name_len);
                        println!("✗ {}  {}", name_padded.red(), note);
                    }
                    Item::Warn { name, note } => {
                        let name_padded = format!("{:width$}", name, width = global_max_name_len);
                        println!("→ {}  {}", name_padded.yellow(), note);
                    }
                    Item::Dot { name, note } => {
                        let name_padded = format!("{:width$}", name, width = global_max_name_len);
                        println!("• {}  {}", name_padded, note);
                    }
                    Item::Plain { text, color } => {
                        let styled = if let Some(c) = color {
                            text.color(*c).to_string()
                        } else {
                            text.clone()
                        };
                        println!("  {}", styled); // two spaces to align with items
                    }
                }
            }
            println!(); // blank line after section
        }
    }
}

impl Default for StartupInfo {
    fn default() -> Self {
        Self::new()
    }
}
