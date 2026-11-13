<div align="center">

<img src="docs/icon.png" width="128" alt="Voltkit-Symbol: ein Blitz, der einen Schraubenschlüssel kreuzt">

# Voltkit

[English](README.md) · [Español](README.es.md) · **Deutsch** · [Français](README.fr.md) · [简体中文](README.zh-CN.md)

**Praktische Werkzeuge für Ihren Linux-Desktop.**
Texterkennung, ein QR-Code-Scanner, ein Farbwähler, ein Bildschirmlineal, ein Bild- und Videokompressor,
Textbausteine, „Intelligentes Einfügen“, „Wach halten“, ein Editor für hosts und Umgebungsvariablen, eine Suche
nach „Was belegt das?“, eine Verwaltung für App-Berechtigungen, eine Fehlersuche für die Bereitschaft, ein Systembericht, ein Tastenkürzel-Detektiv und ein GPU-Tuner – in einer App.

[![Neueste Version](https://img.shields.io/github/v/release/arios4k/voltkit?label=version&color=7c3aed)](https://github.com/arios4k/voltkit/releases/latest)
[![CI](https://github.com/arios4k/voltkit/actions/workflows/ci.yml/badge.svg)](https://github.com/arios4k/voltkit/actions/workflows/ci.yml)
[![Handbuch](https://img.shields.io/badge/Handbuch-lesen-2ea44f)](https://arios4k.github.io/voltkit/manual/)
[![Lizenz: MIT](https://img.shields.io/badge/lizenz-MIT-blue)](LICENSE)
[![Mit apt installieren](https://img.shields.io/badge/installieren-apt-orange)](#installation-ubuntu-debian-und-abkömmlinge)
[![Unter Fedora installieren](https://img.shields.io/badge/installieren-Fedora-51a2da)](#installation-fedora)
[![Mit Rust gebaut](https://img.shields.io/badge/gebaut%20mit-Rust-dea584?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Auf PayPal unterstützen](https://img.shields.io/badge/PayPal-Support-00457C?logo=paypal&logoColor=white)](https://paypal.me/AlexisRios)

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/color-dark.png?v=0.11.0">
  <img src="docs/screenshots/color-light.png?v=0.11.0" alt="Der Farbwähler von Voltkit zeigt #7C3AED mit HEX-, RGB- und HSL-Werten und WCAG-Kontrastbewertungen">
</picture>

</div>

> Voltkit gibt es auf Deutsch, Englisch, Spanisch, Französisch und vereinfachtem Chinesisch; es verwendet automatisch die Sprache Ihres Systems – in der
> App, auf der Befehlszeile und in den Handbuchseiten (`man voltkit`). Die Bildschirmfotos auf dieser Seite zeigen
> die englische Version.

## Eine App statt achtzehn

Diese Grundfunktionen unter Linux zu bekommen, heißt meist, für jede Aufgabe ein eigenes Applet zu suchen, jedes
mit eigenem Aussehen, eigenen Einstellungen und Tastenkürzeln. Voltkit vereint sie in einer nativen GTK4-App mit
einem gemeinsamen Satz Tastenkürzel:

| Statt … | Voltkit bietet | Tastenkürzel |
|---|---|---|
| nichts: dafür gab es keine Oberfläche | **App-Berechtigungen**: Antworten ändern oder vergessen, die Sie Apps auf Anfragen nach Bildschirmfotos, Kamera, Standort und mehr gegeben haben | — |
| eines eigenen Bildkompressors plus ffmpeg-Beschwörungen | **Dateikompressor**: PNG-, JPEG-, WebP- oder MP4-Dateien ablegen, um Metadaten zu entfernen und sie zu verkleinern | Ziehen &amp; Ablegen |
| eines eigenen Farbwählers | **Farbwähler**: eine Pipette für jedes Pixel, HEX/RGB/HSL, WCAG-Kontrastprüfung und Verlauf | <kbd>Super</kbd>+<kbd>Umschalt</kbd>+<kbd>C</kbd> |
| Shell-Schleifen mit `mv` oder ein eigenes Umbenennungsprogramm | **Stapel-Umbenennen**: viele Dateien auf einmal umbenennen, mit Suchen und Ersetzen, regulären Ausdrücken und Nummerierung, mit Vorschau und Rückgängig | — |
| ein eigenes Bildverkleinerungsprogramm | **Bildgrößen ändern**: viele Bilder auf einmal verkleinern, mit unveränderter Form und ohne die Originale anzutasten | — |
| ein eigenes GPU-Tuning-Programm plus Herstellertools | **GPU-Tuner**: NVIDIA-, AMD- und Intel-Grafikkarten überwachen und Leistungsgrenzen, Lüfter und Lüfterkurven, Zieltemperaturen, Takte und Spannung ändern, jede Änderung mit Vorschau und rückgängig gemacht, wenn Sie sie nicht bestätigen | — |
| `/etc/hosts` und `~/.profile` im Terminal zu bearbeiten | **Hosts &amp; Umgebung**: hosts-Einträge ein- und ausschalten und die Umgebungsvariablen Ihrer Sitzung verwalten | — |
| einer Shell-Erweiterung, die den Bildschirm anlässt | **Wach halten**: Bereitschaft und Sperre für 30 Minuten bis 4 Stunden verhindern oder bis Sie es ausschalten | — |
| einen eigenen Präsentationsstift | **Zoom und Anmerkungen**: den Bildschirm einfrieren, hineinzoomen und mit Stift, Pfeilen, Rahmen und Text darauf zeichnen | <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>Z</kbd> |
| eines QR-Scanners auf dem Telefon oder einer QR-Webseite | **QR-Code**: einen QR-Code irgendwo auf dem Bildschirm lesen oder Text, Links und WLAN-Daten in einen umwandeln | <kbd>Super</kbd>+<kbd>Umschalt</kbd>+<kbd>Q</kbd> |
| eines eigenen Bildschirmlineals oder Bildschirmfoto plus Bildbearbeitung | **Bildschirmlineal**: den Bildschirm einfrieren und alles in Pixeln messen | <kbd>Super</kbd>+<kbd>Umschalt</kbd>+<kbd>R</kbd> |
| zu raten, warum ein Tastenkürzel nichts tut | **Tastenkürzel-Detektiv**: ein beliebiges Tastenkürzel drücken, um zu sehen, wem es gehört, Konflikte finden, eine freie Kombination wählen und es ändern | — |
| Online-Konvertern für JSON, Tabellen und Groß-/Kleinschreibung | **Intelligentes Einfügen**: die Zwischenablage umgewandelt einfügen: aufgeräumtes JSON, eine Tabelle als Markdown, ein Link ohne Tracking, Text aus einem PDF ohne Zeilenumbrüche oder der Text in einem kopierten Bild | <kbd>Super</kbd>+<kbd>Umschalt</kbd>+<kbd>V</kbd> |
| eines Stapels Terminalbefehle für „Welche Hardware habe ich?“ | **Systembericht**: Ihre Hardware erklärt, mit Prüfungen und Lösungen für Treiber und Laufwerkszustand, Temperaturen, Firmware-Aktualisierungen, Speicheraufrüstung, Änderungen seit einer Aktualisierung und einem Bericht, den Sie gefahrlos teilen können | — |
| eines eigenen Textbaustein-Programms | **Textbausteine**: eine durchsuchbare Bausteinauswahl, dazu optionale Kürzel beim Tippen wie `;date` | <kbd>Super</kbd>+<kbd>Umschalt</kbd>+<kbd>E</kbd> |
| einer eigenen OCR-App | **Texterkennung**: einen Rahmen um beliebigen Text auf dem Bildschirm ziehen, auch in Videos und Bildern, und er landet in Ihrer Zwischenablage | <kbd>Super</kbd>+<kbd>Umschalt</kbd>+<kbd>T</kbd> |
| `lsof`, `fuser` und `ss` im Terminal | **Was belegt das?**: sehen, welche Apps eine Datei, einen Ordner, ein Laufwerk oder einen Port belegen, und sie schließen | — |
| in Systemprotokollen im Terminal zu wühlen | **Warum keine Bereitschaft?**: sehen, was den Computer wach hält, was ihn geweckt hat und wie viel Akku er in Bereitschaft verbraucht hat | — |

## Handbuch

**Wie Sie Voltkit benutzen, steht im [Voltkit-Handbuch](https://arios4k.github.io/voltkit/manual/)** (auf Englisch). Es erklärt jedes Werkzeug Schritt für Schritt, mit Tastenkürzeln, Befehlszeile, Konfiguration und Fehlerbehebung. Es öffnet sich auch in der App mit **F1** oder über **Hilfe und Handbuch** im Menü.

## Warum Voltkit

- **Für Wayland gebaut.** Es nutzt die Portale des Desktops für Bildschirmfotos, Pipette und Einfügen – keine
  X11-Tricks und keine Shell-Erweiterungen.
- **Privat von Grund auf.** Alles läuft lokal, ohne Netzwerkzugriff und ohne Telemetrie. Kürzel beim Tippen
  behalten nur die letzten paar Zeichen im Speicher und protokollieren oder speichern nie, was Sie tippen.
- **Klein und nativ.** Ein 4-MB-Programm (ein 1,4-MB-Paket), GTK4 + libadwaita, heller und dunkler Modus.
- **Skriptfähig.** Jede Funktion ist auch ein Befehl (`voltkit ocr`, `voltkit color '#3584e4'`,
  `voltkit compress *.jpg`), mit Handbuchseiten für alle.
- **Prüft sich selbst.** `voltkit doctor` sagt genau, was fehlt, und nennt den Befehl zur Installation.
- **Signierte Pakete.** Installation und Aktualisierung über apt aus einem GPG-signierten Repository.
- **In Ihrer Sprache.** Deutsch, Englisch, Spanisch, Französisch und vereinfachtes Chinesisch, passend zur Sprache Ihres Systems.

Voltkit wird unter GNOME (Zorin OS 18) entwickelt und getestet. Es nutzt die Standard-XDG-Portale, daher sollten auch KDE Plasma und andere Desktops funktionieren; Rückmeldungen sind willkommen. Weitere Antworten finden Sie in den [häufigen Fragen](https://arios4k.github.io/voltkit/manual/faq.html) im Handbuch.

<sub>Bildschirmfotos unter Zorin OS 18 mit den hellen und dunklen Farbpaletten von Voltkit. Neu erstellen mit
`docs/screenshots/capture.sh` (oder `docs/screenshots/capture.sh de` für die deutsche App; auch `es`, `fr` und `zh_CN`).</sub>

## Installation (Ubuntu, Debian und Abkömmlinge)

Für Ubuntu 24.04+, Debian 13+, Linux Mint 22, Zorin OS 18, Pop!_OS 24.04 und andere Abkömmlinge auf
64-Bit-Intel/AMD:

```bash
sudo curl -fsSLo /usr/share/keyrings/voltkit-archive-keyring.gpg \
  https://arios4k.github.io/voltkit/voltkit-archive-keyring.gpg
echo "deb [signed-by=/usr/share/keyrings/voltkit-archive-keyring.gpg] https://arios4k.github.io/voltkit stable main" \
  | sudo tee /etc/apt/sources.list.d/voltkit.list
sudo apt update && sudo apt install voltkit
```

apt installiert die benötigten Werkzeuge (Tesseract, wl-clipboard, das Desktop-Portal) zusammen mit Voltkit, und
Aktualisierungen kommen mit `sudo apt upgrade`.

Öffnen Sie Voltkit dann aus Ihren Apps: Es richtet seinen Hintergrunddienst und seine Tastenkürzel selbst ein. Wie es weitergeht, steht unter [Install](https://arios4k.github.io/voltkit/manual/install.html) und [A tour of the app](https://arios4k.github.io/voltkit/manual/tour.html) im Handbuch.

## Installation (Fedora)

Für Fedora 43 und neuer auf 64-Bit-Intel/AMD, aus dem [COPR-Repository](https://copr.fedorainfracloud.org/coprs/arios/voltkit/):

```bash
sudo dnf copr enable arios/voltkit
sudo dnf install voltkit
```

Öffnen Sie Voltkit dann aus Ihren Apps; es richtet sich genauso ein. Aktualisierungen kommen mit
`sudo dnf upgrade`.

## Installation (aus dem Quellcode)

Voraussetzungen: Rust 1.85+, GTK ≥ 4.14, libadwaita ≥ 1.5.

```bash
# Debian/Ubuntu
sudo apt install libgtk-4-dev libadwaita-1-dev
# Fedora
sudo dnf install gtk4-devel libadwaita-devel
# Arch
sudo pacman -S gtk4 libadwaita
```

```bash
cargo install --path .     # installiert nach ~/.cargo/bin/voltkit
voltkit
```

Installieren Sie das Programm, bevor Sie Tastenkürzel oder den Autostart einrichten: Beide speichern seinen Pfad.

So kommt Voltkit mit Symbol in Ihre App-Übersicht:

```bash
install -Dm644 data/icons/io.github.arios4k.Voltkit.svg ~/.local/share/icons/hicolor/scalable/apps/io.github.arios4k.Voltkit.svg
install -Dm644 data/icons/io.github.arios4k.Voltkit-symbolic.svg ~/.local/share/icons/hicolor/symbolic/apps/io.github.arios4k.Voltkit-symbolic.svg
sed "s|^Exec=voltkit|Exec=$HOME/.cargo/bin/voltkit|" data/io.github.arios4k.Voltkit.desktop > ~/.local/share/applications/io.github.arios4k.Voltkit.desktop
```

## Übersetzungen

Voltkit ist ins Deutsche, Spanische, Französische und vereinfachte Chinesische übersetzt. Die Texte liegen in normalen gettext-Dateien in [`po/`](po/),
die ins Programm eingebaut werden – es muss nichts zusätzlich installiert werden. Wie Sie eine Übersetzung
ergänzen oder korrigieren, steht in [`po/README.md`](po/README.md).

## Projektaufbau

```
src/
  main.rs            Einstieg der Befehlszeile (clap) → Oberfläche ohne Unterbefehl
  i18n.rs            Übersetzungen: eingebaute .po-Kataloge, Systemsprache
  app.rs             GTK4-/libadwaita-Zentrale
  daemon.rs          Hintergrunddienst: D-Bus-API, Portal-Tastenkürzel, Autostart
  actions.rs         Tastenkürzel-Aktionen (Texterkennung, Pipette) mit Benachrichtigungen
  shortcuts.rs       Umrechnung von Tastenkombinationen + Ausweichlösung mit GNOME-Tastenkürzeln
  picker.rs          Fenster der Bausteinauswahl
  snippets_ui.rs     Seite Textbausteine: Bausteinliste und Editor
  qr_ui.rs           Seite QR-Code
  ruler.rs           Bildschirmlineal (Ebene über einem eingefrorenen Bildschirmfoto)
  awake.rs           Wach halten (Inhibit-Portal, vom Dienst gehalten)
  awake_ui.rs        Seite Wach halten
  system_ui.rs       Seite Hosts & Umgebung
  usage_ui.rs        Seite Was belegt das?
  permissions_ui.rs  Seite App-Berechtigungen
  sleep_ui.rs        Seite Warum keine Bereitschaft?
  gpu_ui.rs          Seite GPU-Tuner: Messwerte, Unterstützungsbericht, Steuerungen
  gpu_cli.rs         `voltkit gpu`, der Root-Helfer und der Hintergrunddienst
  sysreport_ui.rs    Seite Systembericht
  detective_ui.rs    Seite Tastenkürzel-Detektiv
  smartpaste_ui.rs   Fenster und Seite Intelligentes Einfügen
  expander.rs        Kürzel beim Tippen (Tastatur lesen über evdev, virtuelle uinput-Tastatur)
  paste.rs           in die aktive App einfügen, Zwischenablage wiederherstellen
  portal.rs          XDG-Portal-Aufrufe (Bildschirmfoto, PickColor) und Benachrichtigungen
  config.rs          TOML-Konfiguration mit Standardwerten
  system.rs          Wayland-/X11-Helfer: Zwischenablage, Werkzeugsuche, Prozessausführung
  modules/
    ocr.rs           Bereichsaufnahme + Tesseract
    color.rs         Einlesen, HSL, WCAG-Leuchtdichte und -Kontrast
    compress.rs      Formaterkennung + Komprimierung je Format
    snippets.rs      Vorlagen-Engine für Bausteine
    qr.rs            QR-Codes lesen und erstellen
    hosts.rs         Einlesen von /etc/hosts, das alles Unbearbeitete erhält
    envvars.rs       Umgebungsvariablen der Sitzung (environment.d)
    usage.rs         findet und beendet Prozesse, die einen Pfad oder Port belegen (/proc)
    permissions.rs   der Berechtigungsspeicher des Desktop-Portals
    sleep.rs         Bereitschaftsblocker, Verlauf (Journal, UPower) und Weckgeräte
    gpu.rs           GPU-Erkennung, Sensoren und was jede Karte unterstützt (NVIDIA, AMD, Intel)
    gpu_tune.rs      geprüfte, transaktionale GPU-Änderungen, Lüfterkurven, Startschutz
    nvml.rs          Anbindung an NVIDIAs Verwaltungsbibliothek
    sysreport.rs     Hardware, Prüfungen, Laufwerkszustand, Sensoren, Firmware, Aufnahmen, SMBIOS-Speicher
    keybindings.rs   alle GSettings-Tastenkürzel: Besitzer, Konflikte, freie Kombinationen, Änderungen
    smartpaste.rs    die Textumwandlungen von Intelligentes Einfügen
  manpages.rs        Generator der Handbuchseiten (clap_mangen + handgeschriebene Abschnitte)
  screenshots.rs     erzeugt die README-Bildschirmfotos (versteckter Befehl `screenshots`)
po/                  Übersetzungen (Deutsch, Spanisch, Französisch, vereinfachtes Chinesisch) und update.py zum Aktualisieren
man/                 erzeugte Handbuchseiten (man/de, man/es, man/fr, man/zh_CN: übersetzt)
packaging/apt/       Bau des APT-Repositorys und seine Startseite
docs/screenshots/    README-Bildschirmfotos und capture.sh zum Neuerstellen
docs/ARCHITECTURE.md Entwurfsnotizen und Pläne
```

## Mitmachen

```bash
cargo test
cargo clippy
python3 po/update.py --check   # Übersetzungen aktuell und vollständig?
```

Nach Änderungen an einem Befehl oder seinem Hilfetext erzeugen Sie die Handbuchseiten in [`man/`](man/) mit `cargo run -- manpages man` neu; bis dahin schlägt `cargo test` fehl.

Die D-Bus-API und die Pläne stehen in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). Ein Einstellungsfenster ist geplant. Fehlerberichte und Tests auf anderen Desktops sind besonders
willkommen.

## Veröffentlichen

1. `version` in `Cargo.toml` erhöhen, `cargo run -- manpages man` ausführen und committen.
2. Tag setzen und pushen: `git tag v0.2.0 && git push origin v0.2.0`.

Der [Release-Workflow](.github/workflows/release.yml) baut das `.deb`, veröffentlicht damit ein GitHub-Release und
baut das signierte APT-Repository auf GitHub Pages neu. Er signiert mit dem Schlüssel im Secret `APT_SIGNING_KEY`
(Fingerabdruck `65E8 4B8D 114C 6147 9CCA 4DDC EC0F 6B66 E480 2360`). Das Repository lokal bauen:
`APT_KEY_ID=<fingerabdruck> packaging/apt/build-repo.sh target/debian site`.

## Autor

Erstellt von **Alexis Rios-Negron**.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/about-dark.png?v=0.11.0">
  <img src="docs/screenshots/about-light.png?v=0.11.0" width="640" alt="Das Über-Fenster von Voltkit mit App-Symbol, Version und Autor Alexis Rios-Negron">
</picture>

## Voltkit unterstützen

Wenn Voltkit für dich hilfreich war, freue ich mich über eine [Spende in beliebiger Höhe](https://paypal.me/AlexisRios). Sie hilft dabei, das Projekt am Laufen zu halten und weiterzuentwickeln. Jeder Beitrag wird dankbar angenommen.

Voltkit soll auch in Zukunft kostenlos und für alle zugänglich sein. Vielen Dank fürs Nutzen und für jede Unterstützung!

## Lizenz

MIT © Alexis Rios-Negron
