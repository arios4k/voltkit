pub mod color;
pub mod compress;
pub mod envvars;
pub mod hosts;
pub mod keybindings;
pub mod ocr;
pub mod qr;
pub mod sleep;
pub mod smartpaste;
pub mod permissions;
pub mod snippets;
pub mod sysreport;
pub mod usage;

/// Static description of a module, used to build the control center sidebar.
pub struct ModuleInfo {
    pub id: &'static str,
    pub title: &'static str,
    pub icon: &'static str,
    pub summary: &'static str,
}

/// Sorted by title: the sidebar lists them in this order.
pub const ALL: &[ModuleInfo] = &[
    ModuleInfo {
        id: "permissions",
        title: "App Permissions",
        icon: "security-high-symbolic",
        summary: "Change or forget the answers you gave when apps asked for screenshots, the camera and more.",
    },
    ModuleInfo {
        id: "compress",
        title: "Asset Compressor",
        icon: "package-x-generic-symbolic",
        summary: "Drop images or videos to strip metadata and shrink them.",
    },
    ModuleInfo {
        id: "color",
        title: "Color Picker",
        icon: "color-select-symbolic",
        summary: "HEX / RGB / HSL conversion, WCAG contrast checking and history.",
    },
    ModuleInfo {
        id: "system",
        title: "Hosts & Environment",
        icon: "network-server-symbolic",
        summary: "Edit /etc/hosts and your session's environment variables without a text editor.",
    },
    ModuleInfo {
        id: "awake",
        title: "Keep Awake",
        icon: "display-brightness-symbolic",
        summary: "Stop your computer from sleeping or locking for a while.",
    },
    ModuleInfo {
        id: "qr",
        title: "QR Code",
        icon: "view-grid-symbolic",
        summary: "Read a QR code anywhere on screen, or turn text, links and Wi-Fi details into one.",
    },
    ModuleInfo {
        id: "ruler",
        title: "Screen Ruler",
        icon: "zoom-fit-best-symbolic",
        summary: "Measure distances and sizes on screen in pixels.",
    },
    ModuleInfo {
        id: "shortcuts",
        title: "Shortcut Detective",
        icon: "input-keyboard-symbolic",
        summary: "Find which app owns a keyboard shortcut, fix clashes, and pick free combinations.",
    },
    ModuleInfo {
        id: "smartpaste",
        title: "Smart Paste",
        icon: "edit-paste-symbolic",
        summary: "Paste what you copied converted: tidy JSON, tables as Markdown, clean links, a new letter case.",
    },
    ModuleInfo {
        id: "sysreport",
        title: "System Report",
        icon: "computer-symbolic",
        summary: "What's in this computer, explained: drivers, drive health, temperatures, firmware updates and what changed.",
    },
    ModuleInfo {
        id: "snippets",
        title: "Text Expansion",
        icon: "insert-text-symbolic",
        summary: "Snippets with dynamic variables: dates, clipboard, parameters.",
    },
    ModuleInfo {
        id: "ocr",
        title: "Text Extractor",
        icon: "edit-select-all-symbolic",
        summary: "Drag a box around any text on screen and copy it to the clipboard.",
    },
    ModuleInfo {
        id: "using",
        title: "What's Using This?",
        icon: "system-search-symbolic",
        summary: "See which apps keep a file, folder, drive or network port busy, and close them.",
    },
    ModuleInfo {
        id: "sleep",
        title: "Why Didn't It Sleep?",
        icon: "weather-clear-night-symbolic",
        summary: "What keeps the computer awake, what woke it up, and how much battery it used asleep.",
    },
];

#[cfg(test)]
mod tests {
    #[test]
    fn tools_are_sorted_by_title() {
        let titles: Vec<String> = super::ALL.iter().map(|m| m.title.to_lowercase()).collect();
        let mut sorted = titles.clone();
        sorted.sort();
        assert_eq!(titles, sorted);
    }
}
