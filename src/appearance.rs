//! Shared visual treatment for the control center and standalone tool windows.

fn update(provider: &gtk::CssProvider, style: &adw::StyleManager) {
    // Keep the desktop's accessible palette when high contrast is enabled.
    let palette = if style.is_high_contrast() {
        ""
    } else if style.is_dark() {
        "@define-color window_bg_color #1b2026;
         @define-color window_fg_color #f0f3f7;
         @define-color view_bg_color #171c22;
         @define-color view_fg_color #f0f3f7;
         @define-color card_bg_color #222930;
         @define-color card_fg_color #f0f3f7;
         @define-color headerbar_bg_color #1b2026;
         @define-color headerbar_fg_color #f0f3f7;
         @define-color accent_bg_color #2879ee;
         @define-color accent_fg_color #ffffff;
         @define-color accent_color #78acff;
         @define-color warning_color #ffca66;"
    } else {
        "@define-color window_bg_color #f3f5f8;
         @define-color window_fg_color #202631;
         @define-color view_bg_color #edf0f5;
         @define-color view_fg_color #202631;
         @define-color card_bg_color #ffffff;
         @define-color card_fg_color #202631;
         @define-color headerbar_bg_color #f3f5f8;
         @define-color headerbar_fg_color #202631;
         @define-color accent_bg_color #2167d5;
         @define-color accent_fg_color #ffffff;
         @define-color accent_color #1959bb;
         @define-color warning_color #965300;"
    };
    provider.load_from_string(&format!("{palette}\n{}", include_str!("appearance.css")));
}

pub fn install() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let Some(display) = gtk::gdk::Display::default() else { return };
        let provider = gtk::CssProvider::new();
        let style = adw::StyleManager::default();
        update(&provider, &style);
        gtk::style_context_add_provider_for_display(&display, &provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
        let dark_provider = provider.clone();
        style.connect_dark_notify(move |style| update(&dark_provider, style));
        style.connect_high_contrast_notify(move |style| update(&provider, style));
    });
}
