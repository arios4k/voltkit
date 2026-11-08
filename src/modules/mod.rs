pub mod color;
pub mod compress;
pub mod envvars;
pub mod gpu;
pub mod gpu_advisor;
pub mod gpu_check;
pub mod gpu_curve;
pub mod gpu_idle;
pub mod gpu_profiles;
pub mod gpu_session;
pub mod gpu_tune;
pub mod hosts;
pub mod nvml;
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

impl ModuleInfo {
    pub fn title(&self) -> &'static str {
        crate::i18n::tr(self.title)
    }

    pub fn summary(&self) -> &'static str {
        crate::i18n::tr(self.summary)
    }
}

/// The tools in sidebar order: by title in the user's language, ignoring
/// accents, case and leading punctuation such as Spanish's ¿.
pub fn sorted() -> Vec<&'static ModuleInfo> {
    let key = |m: &ModuleInfo| -> String {
        m.title()
            .chars()
            .skip_while(|c| !c.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .map(|c| match c {
                'á' | 'à' | 'â' | 'ä' => 'a',
                'é' | 'è' | 'ê' | 'ë' => 'e',
                'í' | 'ì' | 'î' | 'ï' => 'i',
                'ó' | 'ò' | 'ô' | 'ö' => 'o',
                'ú' | 'ù' | 'û' | 'ü' => 'u',
                'ñ' => 'n',
                'ß' => 's',
                c => c,
            })
            .collect()
    };
    let mut all: Vec<&ModuleInfo> = ALL.iter().collect();
    all.sort_by_cached_key(|m| key(m));
    all
}

/// Sorted by English title; [`sorted`] gives the order for the user's language.
pub const ALL: &[ModuleInfo] = &[
    ModuleInfo {
        id: "permissions",
        title: N_!("App Permissions"),
        icon: "security-high-symbolic",
        summary: N_!("Change or forget the answers you gave when apps asked for screenshots, the camera and more."),
    },
    ModuleInfo {
        id: "compress",
        title: N_!("Asset Compressor"),
        icon: "package-x-generic-symbolic",
        summary: N_!("Drop images or videos to strip metadata and shrink them."),
    },
    ModuleInfo {
        id: "color",
        title: N_!("Color Picker"),
        icon: "color-select-symbolic",
        summary: N_!("HEX / RGB / HSL conversion, WCAG contrast checking and history."),
    },
    ModuleInfo {
        id: "gpu",
        title: N_!("GPU Tuner"),
        icon: "voltkit-gpu-symbolic",
        summary: N_!("Monitor and tune NVIDIA, AMD and Intel graphics cards, and see exactly which controls each one supports."),
    },
    ModuleInfo {
        id: "system",
        title: N_!("Hosts & Environment"),
        icon: "network-server-symbolic",
        summary: N_!("Edit /etc/hosts and your session's environment variables without a text editor."),
    },
    ModuleInfo {
        id: "awake",
        title: N_!("Keep Awake"),
        icon: "display-brightness-symbolic",
        summary: N_!("Stop your computer from sleeping or locking for a while."),
    },
    ModuleInfo {
        id: "qr",
        title: N_!("QR Code"),
        icon: "view-grid-symbolic",
        summary: N_!("Read a QR code anywhere on screen, or turn text, links and Wi-Fi details into one."),
    },
    ModuleInfo {
        id: "ruler",
        title: N_!("Screen Ruler"),
        icon: "zoom-fit-best-symbolic",
        summary: N_!("Measure distances and sizes on screen in pixels."),
    },
    ModuleInfo {
        id: "shortcuts",
        title: N_!("Shortcut Detective"),
        icon: "input-keyboard-symbolic",
        summary: N_!("Find which app owns a keyboard shortcut, fix clashes, and pick free combinations."),
    },
    ModuleInfo {
        id: "smartpaste",
        title: N_!("Smart Paste"),
        icon: "edit-paste-symbolic",
        summary: N_!("Paste what you copied converted: tidy JSON, tables as Markdown, clean links, a new letter case."),
    },
    ModuleInfo {
        id: "sysreport",
        title: N_!("System Report"),
        icon: "computer-symbolic",
        summary: N_!("What's in this computer, explained: drivers, drive health, temperatures, firmware updates and what changed."),
    },
    ModuleInfo {
        id: "snippets",
        title: N_!("Text Expansion"),
        icon: "insert-text-symbolic",
        summary: N_!("Snippets with dynamic variables: dates, clipboard, parameters."),
    },
    ModuleInfo {
        id: "ocr",
        title: N_!("Text Extractor"),
        icon: "edit-select-all-symbolic",
        summary: N_!("Drag a box around any text on screen and copy it to the clipboard."),
    },
    ModuleInfo {
        id: "using",
        title: N_!("What's Using This?"),
        icon: "system-search-symbolic",
        summary: N_!("See which apps keep a file, folder, drive or network port busy, and close them."),
    },
    ModuleInfo {
        id: "sleep",
        title: N_!("Why Didn't It Sleep?"),
        icon: "weather-clear-night-symbolic",
        summary: N_!("What keeps the computer awake, what woke it up, and how much battery it used asleep."),
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
        let ids: Vec<&str> = super::sorted().iter().map(|m| m.id).collect();
        assert_eq!(ids, super::ALL.iter().map(|m| m.id).collect::<Vec<_>>());
    }
}
