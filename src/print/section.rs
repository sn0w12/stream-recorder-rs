use tiny_table::{Color, StyleAction, apply_style_actions, impl_style_methods};

struct ItemBody {
    name: String,
    note: String,
    styles: Vec<StyleAction>,
}

/// Types of items that can appear in a section.
enum Item {
    Ok(ItemBody),
    Err(ItemBody),
    Warn(ItemBody),
    Dot(ItemBody),
    Plain(ItemBody),
}

impl_style_methods!(ItemBody, |mut item: ItemBody, action| {
    item.styles.push(action);
    item
});

fn with_styles(content: &str, styles: &[StyleAction]) -> String {
    apply_style_actions(content, styles)
}

fn with_default_color(content: &str, color: Color) -> String {
    with_styles(content, &[StyleAction::Color(color)])
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
        self.push_item(Item::Ok(ItemBody {
            name: name.into(),
            note: note.into(),
            styles: Vec::new(),
        }));
    }

    /// Add a "✗" item (red).
    pub fn err(&mut self, name: impl Into<String>, note: impl Into<String>) {
        self.push_item(Item::Err(ItemBody {
            name: name.into(),
            note: note.into(),
            styles: Vec::new(),
        }));
    }

    /// Add a "→" item (yellow).
    pub fn warn(&mut self, name: impl Into<String>, note: impl Into<String>) {
        self.push_item(Item::Warn(ItemBody {
            name: name.into(),
            note: note.into(),
            styles: Vec::new(),
        }));
    }

    /// Add a "•" item (plain).
    pub fn dot(&mut self, name: impl Into<String>, note: impl Into<String>) {
        self.push_item(Item::Dot(ItemBody {
            name: name.into(),
            note: note.into(),
            styles: Vec::new(),
        }));
    }

    /// Add a plain text line (e.g., "No platforms configured").
    pub fn plain(&mut self, text: impl Into<String>, color: Option<Color>) {
        self.push_item(Item::Plain(ItemBody {
            name: text.into(),
            note: String::new(),
            styles: vec![StyleAction::Color(color.unwrap_or(Color::White))],
        }));
    }

    /// Print all sections with globally consistent underlines and name alignment.
    pub fn print(&self) {
        if self.sections.is_empty() {
            return;
        }

        let mut global_max_name_len = 0;
        for section in &self.sections {
            for item in &section.items {
                if let Item::Ok(ItemBody { name, .. })
                | Item::Err(ItemBody { name, .. })
                | Item::Warn(ItemBody { name, .. })
                | Item::Dot(ItemBody { name, .. }) = item
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
                    Item::Ok(ItemBody { note, .. })
                    | Item::Err(ItemBody { note, .. })
                    | Item::Warn(ItemBody { note, .. })
                    | Item::Dot(ItemBody { note, .. }) => {
                        // 4 = symbol(1) + space(1) + two spaces before note(2)
                        4 + global_max_name_len + note.chars().count()
                    }
                    Item::Plain(ItemBody { name, .. }) => {
                        // plain lines are indented by 2 spaces
                        2 + name.chars().count()
                    }
                };
                if width > global_max_line_width {
                    global_max_line_width = width;
                }
            }
        }

        for section in &self.sections {
            // Print section title (bold, no extra spaces)
            println!("{}", with_styles(&section.title, &[StyleAction::Bold]));
            // Underline to global max width
            println!("{}", "─".repeat(global_max_line_width));

            // Print items
            for item in &section.items {
                match item {
                    Item::Ok(ItemBody { name, note, styles }) => {
                        let name_padded = format!("{:width$}", name, width = global_max_name_len);
                        let name_colored = with_default_color(&name_padded, Color::Green);
                        let name_styled = with_styles(&name_colored, styles);
                        let note_styled = with_styles(note, styles);
                        println!("✓ {}  {}", name_styled, note_styled);
                    }
                    Item::Err(ItemBody { name, note, styles }) => {
                        let name_padded = format!("{:width$}", name, width = global_max_name_len);
                        let name_colored = with_default_color(&name_padded, Color::Red);
                        let name_styled = with_styles(&name_colored, styles);
                        let note_styled = with_styles(note, styles);
                        println!("✗ {}  {}", name_styled, note_styled);
                    }
                    Item::Warn(ItemBody { name, note, styles }) => {
                        let name_padded = format!("{:width$}", name, width = global_max_name_len);
                        let name_colored = with_default_color(&name_padded, Color::Yellow);
                        let name_styled = with_styles(&name_colored, styles);
                        let note_styled = with_styles(note, styles);
                        println!("→ {}  {}", name_styled, note_styled);
                    }
                    Item::Dot(ItemBody { name, note, styles }) => {
                        let name_padded = format!("{:width$}", name, width = global_max_name_len);
                        let name_styled = with_styles(&name_padded, styles);
                        let note_styled = with_styles(note, styles);
                        println!("• {}  {}", name_styled, note_styled);
                    }
                    Item::Plain(ItemBody { name, styles, .. }) => {
                        let styled = with_styles(name, styles);
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
