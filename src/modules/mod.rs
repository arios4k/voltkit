pub mod color;
pub mod compress;
pub mod ocr;
pub mod qr;
pub mod snippets;

/// Static description of a module, used to build the control center sidebar.
pub struct ModuleInfo {
    pub id: &'static str,
    pub title: &'static str,
    pub icon: &'static str,
    pub summary: &'static str,
}

pub const ALL: &[ModuleInfo] = &[
    ModuleInfo {
        id: "ocr",
        title: "Text Extractor",
        icon: "edit-select-all-symbolic",
        summary: "Drag a box around any text on screen and copy it to the clipboard.",
    },
    ModuleInfo {
        id: "qr",
        title: "QR Code",
        icon: "view-grid-symbolic",
        summary: "Read a QR code anywhere on screen, or turn text, links and Wi-Fi details into one.",
    },
    ModuleInfo {
        id: "color",
        title: "Color Picker",
        icon: "color-select-symbolic",
        summary: "HEX / RGB / HSL conversion, WCAG contrast checking and history.",
    },
    ModuleInfo {
        id: "ruler",
        title: "Screen Ruler",
        icon: "zoom-fit-best-symbolic",
        summary: "Measure distances and sizes on screen in pixels.",
    },
    ModuleInfo {
        id: "compress",
        title: "Asset Compressor",
        icon: "package-x-generic-symbolic",
        summary: "Drop images or videos to strip metadata and shrink them.",
    },
    ModuleInfo {
        id: "snippets",
        title: "Text Expansion",
        icon: "insert-text-symbolic",
        summary: "Snippets with dynamic variables: dates, clipboard, parameters.",
    },
    ModuleInfo {
        id: "awake",
        title: "Keep Awake",
        icon: "display-brightness-symbolic",
        summary: "Stop your computer from sleeping or locking for a while.",
    },
];
