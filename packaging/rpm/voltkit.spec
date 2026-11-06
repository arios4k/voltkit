# The release profile strips the binary, so there's no debug info to package.
%global debug_package %{nil}

Name:           voltkit
Version:        0.10.0
Release:        1%{?dist}
Summary:        Power tools for your Linux desktop in one GTK4 app
License:        MIT
URL:            https://github.com/arios4k/voltkit
Source0:        %{url}/archive/refs/tags/v%{version}/%{name}-%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  gcc
BuildRequires:  systemd-rpm-macros
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

%description
Voltkit brings everyday desktop tools to Linux in one GTK4/libadwaita app and a
single command: screen OCR, a QR code scanner and maker, a color picker with
contrast checking, a screen ruler, an image and video compressor, a text
expander, Smart Paste, Keep Awake, a hosts and environment variable editor, a
finder for the apps keeping a drive, file or port busy, an app permissions
manager, a troubleshooter for sleep and wake-ups, and a system report with
driver and drive-health checks, firmware updates and what changed after
updates, and a shortcut detective that finds what owns a keyboard shortcut,
lists clashes and changes them, plus a GPU tuner for NVIDIA, AMD and Intel cards. Built for Wayland, with global hotkeys.

%prep
%autosetup -n %{name}-%{version}

%build
cargo build --release --locked

%install
install -Dm755 target/release/voltkit -t %{buildroot}%{_bindir}/
install -Dm644 data/io.github.arios4k.Voltkit.desktop -t %{buildroot}%{_datadir}/applications/
install -Dm644 data/io.github.arios4k.Voltkit.Daemon.desktop -t %{buildroot}%{_sysconfdir}/xdg/autostart/
install -Dm644 data/io.github.arios4k.Voltkit.metainfo.xml -t %{buildroot}%{_metainfodir}/
install -Dm644 data/voltkit-gpu.service -t %{buildroot}%{_unitdir}/
install -Dm644 data/icons/io.github.arios4k.Voltkit.svg -t %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/
install -Dm644 data/icons/io.github.arios4k.Voltkit-symbolic.svg -t %{buildroot}%{_datadir}/icons/hicolor/symbolic/apps/
install -Dm644 man/*.1 -t %{buildroot}%{_mandir}/man1/
install -Dm644 man/es/*.1 -t %{buildroot}%{_mandir}/es/man1/
install -Dm644 man/de/*.1 -t %{buildroot}%{_mandir}/de/man1/
install -Dm644 man/fr/*.1 -t %{buildroot}%{_mandir}/fr/man1/
install -Dm644 man/zh_CN/*.1 -t %{buildroot}%{_mandir}/zh_CN/man1/

%check
appstream-util validate-relax --nonet %{buildroot}%{_metainfodir}/io.github.arios4k.Voltkit.metainfo.xml

%files
%license LICENSE
%doc README.md README.es.md README.de.md README.fr.md README.zh-CN.md data/config.example.toml
%{_bindir}/voltkit
%{_datadir}/applications/io.github.arios4k.Voltkit.desktop
%{_metainfodir}/io.github.arios4k.Voltkit.metainfo.xml
%config(noreplace) %{_sysconfdir}/xdg/autostart/io.github.arios4k.Voltkit.Daemon.desktop
%{_unitdir}/voltkit-gpu.service
%{_datadir}/icons/hicolor/scalable/apps/io.github.arios4k.Voltkit.svg
%{_datadir}/icons/hicolor/symbolic/apps/io.github.arios4k.Voltkit-symbolic.svg
%{_mandir}/man1/voltkit*.1*
%lang(es) %{_mandir}/es/man1/voltkit*.1*
%lang(de) %{_mandir}/de/man1/voltkit*.1*
%lang(fr) %{_mandir}/fr/man1/voltkit*.1*
%lang(zh_CN) %{_mandir}/zh_CN/man1/voltkit*.1*

%changelog
* Sat Nov 07 2026 Alexis Rios-Negron <arios4k@users.noreply.github.com> - 0.10.0-1
- GPU Tuner: edit fan curves by dragging points on a graph, and choose the temperature and fans they follow
- GPU Tuner: profiles that switch automatically by power source, running program, load or temperature
- GPU Tuner: record sessions and compare two side by side with energy and efficiency
- GPU Tuner: edit full voltage and frequency curves on AMD cards that have them
- New voltkit gpu check compares readings with nvidia-smi and measures monitoring cost; fixes video memory read too high on NVIDIA

* Fri Nov 06 2026 Alexis Rios-Negron <arios4k@users.noreply.github.com> - 0.9.2-1
- Add GPU Tuner: monitor NVIDIA, AMD and Intel graphics cards and change power limits, fans and fan curves, temperature targets, clocks and voltage, with every change previewed and undone unless confirmed
- Fix unreadable number fields in the light theme

* Thu Nov 05 2026 Alexis Rios-Negron <arios4k@users.noreply.github.com> - 0.9.1-1
- Add translated guidance for inspecting busy files, drives and network ports
- Refresh documentation and project funding links

* Wed Nov 04 2026 Alexis Rios-Negron <arios4k@users.noreply.github.com> - 0.9.0-1
- Translate the app, command line, help and man pages into French and Simplified Chinese

* Tue Nov 03 2026 Alexis Rios-Negron <arios4k@users.noreply.github.com> - 0.8.0-1
- Translate the app, command line, help and man pages into Spanish and German
- Follow the system language, falling back to English

* Mon Nov 02 2026 Alexis Rios-Negron <arios4k@users.noreply.github.com> - 0.7.0-1
- Redesign all tools with adaptive card layouts and consistent light and dark styling
- Add searchable snippet and app permission lists and a shortcut details workspace
- Add a branded About dialog, larger default window, and compact tool navigation

* Mon Nov 02 2026 Alexis Rios-Negron <arios4k@users.noreply.github.com> - 0.6.0-1
- Typed triggers turn on with one switch: one password, no logout or restart
- PNG compression is built in (oxipng is no longer needed)
- Quit in the main menu (Ctrl+Q), more room beside header buttons

* Mon Nov 02 2026 Alexis Rios-Negron <arios4k@users.noreply.github.com> - 0.5.0-1
- New: System Report (drivers, drive health, temperatures, firmware, memory, what changed)
- New: Shortcut Detective (who owns a shortcut, clashes, changing them)
- The menu lists the tools alphabetically

* Sun Nov 01 2026 Alexis Rios-Negron <arios4k@users.noreply.github.com> - 0.4.3-1
- The download page links the Fedora COPR repository

* Sun Nov 01 2026 Alexis Rios-Negron <arios4k@users.noreply.github.com> - 0.4.2-1
- Hotkey results (text, QR codes, colors) show up in the open app
- The Color Picker page names its hotkey

* Sun Nov 01 2026 Alexis Rios-Negron <arios4k@users.noreply.github.com> - 0.4.1-1
- First Fedora package: sets itself up on first launch (background service and hotkeys)
