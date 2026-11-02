# The release profile strips the binary, so there's no debug info to package.
%global debug_package %{nil}

Name:           voltkit
Version:        0.4.1
Release:        1%{?dist}
Summary:        Power tools for your Linux desktop in one GTK4 app
License:        MIT
URL:            https://github.com/arios4k/voltkit
Source0:        %{url}/archive/refs/tags/v%{version}/%{name}-%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  gcc
BuildRequires:  gtk4-devel
BuildRequires:  libadwaita-devel
BuildRequires:  libappstream-glib

Requires:       hicolor-icon-theme
Requires:       polkit
Requires:       tesseract
Requires:       tesseract-langpack-eng
Requires:       wl-clipboard
Requires:       xdg-desktop-portal
Recommends:     ffmpeg-free
Recommends:     jpegoptim
Recommends:     libwebp-tools
Recommends:     oxipng

%description
Voltkit brings everyday desktop tools to Linux in one GTK4/libadwaita app and a
single command: screen OCR, a QR code scanner and maker, a color picker with
contrast checking, a screen ruler, an image and video compressor, a text
expander, Smart Paste, Keep Awake, a hosts and environment variable editor, a
finder for the apps keeping a drive, file or port busy, an app permissions
manager and a troubleshooter for sleep and wake-ups. Built for Wayland, with
global hotkeys.

%prep
%autosetup -n %{name}-%{version}

%build
cargo build --release --locked

%install
install -Dm755 target/release/voltkit -t %{buildroot}%{_bindir}/
install -Dm644 data/io.github.arios4k.Voltkit.desktop -t %{buildroot}%{_datadir}/applications/
install -Dm644 data/io.github.arios4k.Voltkit.Daemon.desktop -t %{buildroot}%{_sysconfdir}/xdg/autostart/
install -Dm644 data/io.github.arios4k.Voltkit.metainfo.xml -t %{buildroot}%{_metainfodir}/
install -Dm644 data/icons/io.github.arios4k.Voltkit.svg -t %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/
install -Dm644 data/icons/io.github.arios4k.Voltkit-symbolic.svg -t %{buildroot}%{_datadir}/icons/hicolor/symbolic/apps/
install -Dm644 man/*.1 -t %{buildroot}%{_mandir}/man1/

%check
appstream-util validate-relax --nonet %{buildroot}%{_metainfodir}/io.github.arios4k.Voltkit.metainfo.xml

%files
%license LICENSE
%doc README.md data/config.example.toml
%{_bindir}/voltkit
%{_datadir}/applications/io.github.arios4k.Voltkit.desktop
%{_metainfodir}/io.github.arios4k.Voltkit.metainfo.xml
%config(noreplace) %{_sysconfdir}/xdg/autostart/io.github.arios4k.Voltkit.Daemon.desktop
%{_datadir}/icons/hicolor/scalable/apps/io.github.arios4k.Voltkit.svg
%{_datadir}/icons/hicolor/symbolic/apps/io.github.arios4k.Voltkit-symbolic.svg
%{_mandir}/man1/voltkit*.1*

%changelog
* Sun Nov 01 2026 Alexis Rios-Negron <arios4k@users.noreply.github.com> - 0.4.1-1
- First Fedora package: sets itself up on first launch (background service and hotkeys)
