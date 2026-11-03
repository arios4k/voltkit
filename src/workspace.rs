//! Shared card layout for tool workspaces, with an adaptive secondary column.
use adw::prelude::*;

pub struct Workspace {
    root: adw::BreakpointBin,
    primary: gtk::Box,
    secondary: gtk::Box,
    body: gtk::Box,
    narrow: adw::Breakpoint,
}

impl std::ops::Deref for Workspace {
    type Target = adw::BreakpointBin;
    fn deref(&self) -> &Self::Target { &self.root }
}

impl Workspace {
    pub fn new() -> Self {
        let body = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(18)
            .css_classes(["workspace-body"]).build();
        let columns = gtk::Box::builder().spacing(18).homogeneous(true).valign(gtk::Align::Start).build();
        let column = || gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(18)
            .hexpand(true).valign(gtk::Align::Start).build();
        let primary = column();
        let secondary = column();
        secondary.set_visible(false);
        columns.append(&primary);
        columns.append(&secondary);
        body.append(&columns);
        let scroll = gtk::ScrolledWindow::builder().hscrollbar_policy(gtk::PolicyType::Never).child(&body).build();
        let root = adw::BreakpointBin::builder().child(&scroll).width_request(280).height_request(240).build();
        let narrow = adw::Breakpoint::new(adw::BreakpointCondition::parse("max-width: 820sp").unwrap());
        narrow.add_setter(&columns, "orientation", Some(&gtk::Orientation::Vertical.to_value()));
        narrow.add_setter(&columns, "homogeneous", Some(&false.to_value()));
        root.add_breakpoint(narrow.clone());
        Self { root, primary, secondary, body, narrow }
    }

    pub fn add(&self, widget: &impl IsA<gtk::Widget>) {
        self.primary.append(&self.card(widget));
    }

    pub fn aside(&self, widget: &impl IsA<gtk::Widget>) {
        self.secondary.set_visible(true);
        self.secondary.append(&self.card(widget));
    }

    pub fn banner(&self, widget: &impl IsA<gtk::Widget>) {
        self.body.prepend(&self.card(widget));
    }

    fn card(&self, widget: &impl IsA<gtk::Widget>) -> gtk::Box {
        let card = gtk::Box::builder().orientation(gtk::Orientation::Vertical).css_classes(["workspace-card"]).build();
        card.append(widget);
        if let Some(group) = widget.as_ref().downcast_ref::<adw::PreferencesGroup>()
            && let Some(action) = group.header_suffix()
        {
            let toolbar = gtk::Box::builder().spacing(8).halign(gtk::Align::Start).visible(false).build();
            card.append(&toolbar);
            let (group_, toolbar_, action_) = (group.downgrade(), toolbar.downgrade(), action.downgrade());
            self.narrow.connect_apply(move |_| {
                if let (Some(group), Some(toolbar), Some(action)) = (group_.upgrade(), toolbar_.upgrade(), action_.upgrade()) {
                    group.set_header_suffix(None::<&gtk::Widget>);
                    toolbar.append(&action);
                    toolbar.set_margin_top(12);
                    toolbar.set_visible(true);
                }
            });
            let (group_, toolbar_, action_) = (group.downgrade(), toolbar.downgrade(), action.downgrade());
            self.narrow.connect_unapply(move |_| {
                if let (Some(group), Some(toolbar), Some(action)) = (group_.upgrade(), toolbar_.upgrade(), action_.upgrade()) {
                    toolbar.remove(&action);
                    group.set_header_suffix(Some(&action));
                    toolbar.set_visible(false);
                }
            });
        }
        // Async findings and optional sections retain their original visibility.
        widget.bind_property("visible", &card, "visible").sync_create().build();
        card
    }

    pub fn root(&self) -> &adw::BreakpointBin { &self.root }
    pub fn upcast(self) -> gtk::Widget { self.root.upcast() }
}

pub fn note(title: &str, text: &str, icon: &str) -> gtk::Box {
    let content = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(12).build();
    content.append(&gtk::Image::builder().icon_name(icon).pixel_size(32).halign(gtk::Align::Start).css_classes(["accent"]).build());
    content.append(&gtk::Label::builder().label(title).xalign(0.0).wrap(true).css_classes(["title-2"]).build());
    content.append(&gtk::Label::builder().label(text).xalign(0.0).wrap(true).css_classes(["dim-label"]).build());
    content
}
