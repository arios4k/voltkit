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
[![Lizenz: MIT](https://img.shields.io/badge/lizenz-MIT-blue)](LICENSE)
[![Mit apt installieren](https://img.shields.io/badge/installieren-apt-orange)](#installation-ubuntu-debian-und-abkömmlinge)
[![Unter Fedora installieren](https://img.shields.io/badge/installieren-Fedora-51a2da)](#installation-fedora)
[![Mit Rust gebaut](https://img.shields.io/badge/gebaut%20mit-Rust-dea584?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Auf PayPal unterstützen](https://img.shields.io/badge/PayPal-Support-00457C?logo=paypal&logoColor=white)](https://paypal.me/AlexisRios)

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/color-dark.png?v=0.7.0">
  <img src="docs/screenshots/color-light.png?v=0.7.0" alt="Der Farbwähler von Voltkit zeigt #7C3AED mit HEX-, RGB- und HSL-Werten und WCAG-Kontrastbewertungen">
</picture>

</div>

> Voltkit gibt es auf Deutsch, Englisch, Spanisch, Französisch und vereinfachtem Chinesisch; es verwendet automatisch die Sprache Ihres Systems – in der
> App, auf der Befehlszeile und in den Handbuchseiten (`man voltkit`). Die Bildschirmfotos auf dieser Seite zeigen
> die englische Version.

## Eine App statt fünfzehn

Diese Grundfunktionen unter Linux zu bekommen, heißt meist, für jede Aufgabe ein eigenes Applet zu suchen, jedes
mit eigenem Aussehen, eigenen Einstellungen und Tastenkürzeln. Voltkit vereint sie in einer nativen GTK4-App mit
einem gemeinsamen Satz Tastenkürzel:

| Statt … | Voltkit bietet | Tastenkürzel |
|---|---|---|
| nichts: dafür gab es keine Oberfläche | **App-Berechtigungen**: Antworten ändern oder vergessen, die Sie Apps auf Anfragen nach Bildschirmfotos, Kamera, Standort und mehr gegeben haben | — |
| eines eigenen Bildkompressors plus ffmpeg-Beschwörungen | **Dateikompressor**: PNG-, JPEG-, WebP- oder MP4-Dateien ablegen, um Metadaten zu entfernen und sie zu verkleinern | Ziehen &amp; Ablegen |
| eines eigenen Farbwählers | **Farbwähler**: eine Pipette für jedes Pixel, HEX/RGB/HSL, WCAG-Kontrastprüfung und Verlauf | <kbd>Super</kbd>+<kbd>Umschalt</kbd>+<kbd>C</kbd> |
| ein eigenes GPU-Tuning-Programm plus Herstellertools | **GPU-Tuner**: NVIDIA-, AMD- und Intel-Grafikkarten überwachen und Leistungsgrenzen, Lüfter und Lüfterkurven, Zieltemperaturen, Takte und Spannung ändern, jede Änderung mit Vorschau und rückgängig gemacht, wenn Sie sie nicht bestätigen | — |
| `/etc/hosts` und `~/.profile` im Terminal zu bearbeiten | **Hosts &amp; Umgebung**: hosts-Einträge ein- und ausschalten und die Umgebungsvariablen Ihrer Sitzung verwalten | — |
| einer Shell-Erweiterung, die den Bildschirm anlässt | **Wach halten**: Bereitschaft und Sperre für 30 Minuten bis 4 Stunden verhindern oder bis Sie es ausschalten | — |
| eines QR-Scanners auf dem Telefon oder einer QR-Webseite | **QR-Code**: einen QR-Code irgendwo auf dem Bildschirm lesen oder Text, Links und WLAN-Daten in einen umwandeln | <kbd>Super</kbd>+<kbd>Umschalt</kbd>+<kbd>Q</kbd> |
| eines eigenen Bildschirmlineals oder Bildschirmfoto plus Bildbearbeitung | **Bildschirmlineal**: den Bildschirm einfrieren und alles in Pixeln messen | <kbd>Super</kbd>+<kbd>Umschalt</kbd>+<kbd>R</kbd> |
| zu raten, warum ein Tastenkürzel nichts tut | **Tastenkürzel-Detektiv**: ein beliebiges Tastenkürzel drücken, um zu sehen, wem es gehört, Konflikte finden, eine freie Kombination wählen und es ändern | — |
| Online-Konvertern für JSON, Tabellen und Groß-/Kleinschreibung | **Intelligentes Einfügen**: die Zwischenablage umgewandelt einfügen: aufgeräumtes JSON, eine Tabelle als Markdown, ein Link ohne Tracking, Text aus einem PDF ohne Zeilenumbrüche oder der Text in einem kopierten Bild | <kbd>Super</kbd>+<kbd>Umschalt</kbd>+<kbd>V</kbd> |
| eines Stapels Terminalbefehle für „Welche Hardware habe ich?“ | **Systembericht**: Ihre Hardware erklärt, mit Prüfungen und Lösungen für Treiber und Laufwerkszustand, Temperaturen, Firmware-Aktualisierungen, Speicheraufrüstung, Änderungen seit einer Aktualisierung und einem Bericht, den Sie gefahrlos teilen können | — |
| eines eigenen Textbaustein-Programms | **Textbausteine**: eine durchsuchbare Bausteinauswahl, dazu optionale Kürzel beim Tippen wie `;date` | <kbd>Super</kbd>+<kbd>Umschalt</kbd>+<kbd>E</kbd> |
| einer eigenen OCR-App | **Texterkennung**: einen Rahmen um beliebigen Text auf dem Bildschirm ziehen, auch in Videos und Bildern, und er landet in Ihrer Zwischenablage | <kbd>Super</kbd>+<kbd>Umschalt</kbd>+<kbd>T</kbd> |
| `lsof`, `fuser` und `ss` im Terminal | **Was belegt das?**: sehen, welche Apps eine Datei, einen Ordner, ein Laufwerk oder einen Port belegen, und sie schließen | — |
| in Systemprotokollen im Terminal zu wühlen | **Warum keine Bereitschaft?**: sehen, was den Computer wach hält, was ihn geweckt hat und wie viel Akku er in Bereitschaft verbraucht hat | — |

## Funktionen

### 🛡️ App-Berechtigungen

Wenn eine App Bildschirmfotos aufnehmen, Kamera oder Mikrofon verwenden oder Ihren Standort sehen möchte, merkt
sich der Desktop Ihre Antwort. App-Berechtigungen listet sie nach App
auf: Legen Sie einen Schalter um, um eine zu ändern, oder vergessen Sie sie, damit die App erneut fragt.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/permissions-dark.png?v=0.7.0">
  <img src="docs/screenshots/permissions-light.png?v=0.7.0" alt="Die Seite App-Berechtigungen mit Apps und ihren Kamera- und Bildschirmfoto-Berechtigungen">
</picture>

### 📦 Dateikompressor

Legen Sie Dateien im Fenster ab oder führen Sie `voltkit compress` aus. Voltkit entfernt Standortdaten und andere
Metadaten, komprimiert mit dem besten Werkzeug für jedes Format und schreibt `name.min.ext` neben das Original,
das es nie verändert. In unseren Tests: JPEG −43 %, PNG −23 % (verlustfrei), WebP −60 %, MP4 −77 %.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/compress-dark.png?v=0.7.0">
  <img src="docs/screenshots/compress-light.png?v=0.7.0" alt="Der Dateikompressor mit vier komprimierten Dateien und ihrer Ersparnis">
</picture>

### 🎨 Farbwähler

Nehmen Sie mit <kbd>Super</kbd>+<kbd>Umschalt</kbd>+<kbd>C</kbd> ein beliebiges Pixel auf, und der HEX-Code liegt
schon in der Zwischenablage. Die Farbseite rechnet zwischen HEX, RGB und HSL um, bewertet den Kontrast gegen Weiß
und Schwarz nach WCAG (AA / AAA) und merkt sich Ihre letzten Farben.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/color-dark.png?v=0.7.0">
  <img src="docs/screenshots/color-light.png?v=0.7.0" alt="Der Farbwähler mit Vorschau, HEX/RGB/HSL-Formaten, Kontrastbewertungen und gespeicherten Farben">
</picture>

### 🎛️ GPU-Tuner

Beobachten Sie Ihre NVIDIA-, AMD- und Intel-Grafikkarten live und mit Verlauf und ändern Sie, was die Hardware erlaubt:
die Leistungsgrenze, eine feste Lüfterdrehzahl oder eine Lüfterkurve, die Temperatur, auf die eine Karte abzielt,
Taktversatz oder Taktgrenzen und die Spannung. **Was diese Karte unterstützt** listet jede Steuerung so auf, wie der
Treiber sie meldet, und erklärt, warum eine fehlt und was sie freischalten könnte (eine Laptop-GPU, die ihren Lüfter
verbirgt, oder eine AMD-Karte, die `amdgpu.ppfeaturemask` braucht). Änderungen werden an den Grenzen der Karte selbst
geprüft, vorab angezeigt, über einen Root-Helfer angewendet und **rückgängig gemacht, wenn Sie sie nicht bestätigen**:
Auch das Schließen der App oder ein Absturz macht sie rückgängig. Gespeicherte Einstellungen wendet ein
Hintergrunddienst beim Start an; er überspringt ein Profil, dessen Start nie abgeschlossen wurde, damit ein fehlerhaftes
Sie nicht in einer Schleife festhält. `voltkit gpu` gibt denselben Bericht im Terminal aus.

Bearbeiten Sie Lüfterkurven, indem Sie Punkte in einem Diagramm ziehen, und wählen Sie, welcher Temperatur sie folgen und welche Lüfter sie steuern. Speichern Sie Einstellungen als **Profile**, die von selbst wechseln: im Akkubetrieb, solange eine App läuft, unter anhaltender Last oder wenn die Karte heiß wird. **Zeichnen Sie eine Sitzung auf**, während ein Spiel oder Benchmark läuft, und vergleichen Sie zwei Profile nebeneinander: Temperatur, Leistung, Energie und Effizienz (aktiver Takt pro Watt oder Bilder pro Watt, wenn Bilder aufgezeichnet wurden). `voltkit gpu check` vergleicht die Messwerte von Voltkit mit `nvidia-smi` und misst, was die Überwachung kostet.

Haben Sie es auf Ihrer Karte ausprobiert? Ein [Hardware-Bericht](https://github.com/arios4k/voltkit/issues/new?template=gpu-hardware-report.yml) darüber, was funktioniert hat und was nicht, hilft derzeit am meisten.

Das Anwenden von Änderungen ist neu und wurde bisher nur mit simulierter Hardware getestet; was funktioniert, wie es
sicher bleibt und was noch fehlt, steht in [docs/GPU_TUNER.md](docs/GPU_TUNER.md).

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/gpu-dark.png?v=0.10.0">
  <img src="docs/screenshots/gpu-light.png?v=0.10.0" alt="GPU-Tuner mit Live-Messwerten samt Verlauf und den Steuerungen für Leistungsgrenze und Lüfterkurve einer AMD-Radeon-Karte (simulierte Daten)">
</picture>

<sub>Der Screenshot zeigt simulierte Daten erfundener Karten.</sub>

### 📝 Hosts &amp; Umgebung

Schalten Sie Einträge in `/etc/hosts` ein und aus, fügen Sie `myapp.test → 127.0.0.1` für die lokale Entwicklung
hinzu und setzen Sie Umgebungsvariablen wie `EDITOR` oder einen zusätzlichen `PATH`-Ordner für Ihre ganze
Desktop-Sitzung. Systemeinträge sind geschützt, und jedes Speichern von hosts legt eine Sicherung an.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/system-dark.png?v=0.7.0">
  <img src="docs/screenshots/system-light.png?v=0.7.0" alt="Die Seite Hosts & Umgebung mit hosts-Einträgen und Schaltern">
</picture>

### ☕ Wach halten

Sie sehen etwas an, präsentieren oder laden herunter? Schalten Sie „Wach halten“ für 30 Minuten, ein paar Stunden
oder bis zum Ausschalten ein, und Ihr Computer geht weder in Bereitschaft noch sperrt er sich. Es läuft weiter,
nachdem Sie Voltkit geschlossen haben, und schaltet sich ab, wenn die Zeit um ist.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/awake-dark.png?v=0.7.0">
  <img src="docs/screenshots/awake-light.png?v=0.7.0" alt="Wach halten mit aktuellem Status und Einstellung der Dauer">
</picture>

### 📷 QR-Code

Drücken Sie <kbd>Super</kbd>+<kbd>Umschalt</kbd>+<kbd>Q</kbd> und ziehen Sie einen Rahmen um einen QR-Code in
einem Video, einer Folie oder auf einer Webseite: Sein Text oder Link liegt in der Zwischenablage. Umgekehrt
geben Sie einen Link, einen Text oder Ihre WLAN-Daten ein und scannen den Code mit dem Telefon. Kopieren Sie ihn
als Bild oder speichern Sie ihn zum Drucken als PNG. Auf Wunsch kommt Ihr eigenes kleines Logo in die Mitte:
Voltkit liest den Code danach, um sicherzustellen, dass er sich noch scannen lässt.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/qr-dark.png?v=0.7.0">
  <img src="docs/screenshots/qr-light.png?v=0.7.0" alt="Die Seite QR-Code mit einem erzeugten Code für einen Link">
</picture>

### 📏 Bildschirmlineal

<kbd>Super</kbd>+<kbd>Umschalt</kbd>+<kbd>R</kbd> friert den Bildschirm ein. Ziehen Sie, um Breiten und Höhen in
echten Pixeln zu messen (mit gedrückter Umschalttaste für eine gerade Linie), und drücken Sie Strg+C, um die Größe
zu kopieren.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/ruler-dark.png?v=0.7.0">
  <img src="docs/screenshots/ruler-light.png?v=0.7.0" alt="Das Bildschirmlineal mit seiner Startschaltfläche und der Messsteuerung">
</picture>

### ⌨️ Tastenkürzel-Detektiv

Ein Tastenkürzel gedrückt und nichts passiert – oder das Falsche? Drücken Sie es im **Tastenkürzel-Detektiv**, um
jede Aktion zu sehen, die es verwendet: die Fensterverwaltung, GNOME Shell, Systemtasten, Ihre eigenen
Tastenkürzel, Shell-Erweiterungen und Voltkit selbst. Konflikte werden aufgelistet, jeweils mit freien
Kombinationen zum Wechseln, und jedes Tastenkürzel lässt sich direkt dort ändern, ausschalten oder zurücksetzen.
Funktioniert unter GNOME und den ähnlich aufgebauten Desktops (Cinnamon, Budgie, MATE).

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/shortcuts-dark.png?v=0.7.0">
  <img src="docs/screenshots/shortcuts-light.png?v=0.7.0" alt="Der Tastenkürzel-Detektiv zeigt, dass zwei Aktionen Super+V verwenden">
</picture>

### 📋 Intelligentes Einfügen

Drücken Sie <kbd>Super</kbd>+<kbd>Umschalt</kbd>+<kbd>V</kbd> statt <kbd>Strg</kbd>+<kbd>V</kbd> und wählen Sie,
wie das Kopierte eingefügt wird: JSON aufgeräumt oder kompakt, ein Tabellenbereich als Markdown-Tabelle oder CSV,
ein Link ohne Tracking-Parameter, ein Absatz aus einem PDF ohne Zeilenumbrüche, eine Liste mit Aufzählungszeichen
oder Nummern, oder Text in GROSSBUCHSTABEN, mit großen Wortanfängen, in snake_case und mehr. Ein Bildschirmfoto
kopiert? Fügen Sie den Text darin ein. Angeboten werden nur Umwandlungen, die zum Kopierten passen, jeweils mit
Vorschau, und Ihre Zwischenablage bleibt, wie sie war. Dieselben Umwandlungen funktionieren in Skripten:
`wl-paste | voltkit convert markdown-table`.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/smartpaste-dark.png?v=0.7.0">
  <img src="docs/screenshots/smartpaste-light.png?v=0.7.0" alt="Die Seite Intelligentes Einfügen wandelt einen Tabellenbereich in eine Markdown-Tabelle und CSV um">
</picture>

### 🖥️ Systembericht

Nicht nur eine Liste technischer Daten: Der **Systembericht** prüft jedes Hardwareteil und sagt, was es bedeutet.
Hat die Grafikkarte den richtigen Treiber (und wenn nicht, den genauen Befehl zur Behebung)? Ist das Laufwerk
gesund, und wie verschlissen? Wird der Prozessor zu heiß oder drosselt er sich? Wartet eine BIOS- oder
Firmware-Aktualisierung? Wie viele Speichersteckplätze sind frei, und wie viel verträgt dieser Computer?

Voltkit speichert außerdem täglich eine Aufnahme von Hardware, Treibern, Kernel und Firmware; wenn nach einer
Aktualisierung etwas kaputtgeht, zeigt **Was sich geändert hat** genau, was anders ist. Wenn Sie Hilfe brauchen,
liefert **Bericht kopieren** eine aufgeräumte Zusammenfassung für ein Forum oder einen Fehlerbericht, ohne
Seriennummern, Namen oder Netzwerkadressen.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/sysreport-dark.png?v=0.7.0">
  <img src="docs/screenshots/sysreport-light.png?v=0.7.0" alt="Die Seite Systembericht mit Prüfungen, letzten Änderungen und der Hardware des Computers">
</picture>

### ⌨️ Textbausteine

Drücken Sie <kbd>Super</kbd>+<kbd>Umschalt</kbd>+<kbd>E</kbd>, suchen Sie, drücken Sie die Eingabetaste, und der
Baustein wird in die App eingefügt, die Sie gerade verwendet haben. Bausteine können das Datum in jedem Format,
Ihre Zwischenablage und Ausfüllfelder enthalten. Lieber tippen? Schalten Sie Kürzel beim Tippen ein, und aus
`;sig` wird in jeder App Ihre Signatur. Bausteine legen Sie direkt in der App an, bearbeiten und löschen sie: Die
Einfügen-Schaltflächen ergänzen Datum, Uhrzeit, Zwischenablage oder ein Ausfüllfeld, und eine Live-Vorschau zeigt
genau, was eingefügt wird.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/snippets-dark.png?v=0.7.0">
  <img src="docs/screenshots/snippets-light.png?v=0.7.0" alt="Textbausteine mit durchsuchbaren Bausteinen und Einfügen-Steuerelementen">
</picture>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/picker-dark.png?v=0.7.0">
  <img src="docs/screenshots/picker-light.png?v=0.7.0" alt="Die Bausteinauswahl mit Bausteinen und ihren Vorschauen">
</picture>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/snippet-editor-dark.png?v=0.7.0">
  <img src="docs/screenshots/snippet-editor-light.png?v=0.7.0" alt="Der Baustein-Editor mit Kürzel, Text mit Schaltflächen für Datum, Uhrzeit, Zwischenablage und Ausfüllfelder sowie einer Live-Vorschau">
</picture>

### 🔍 Texterkennung

Ein WLAN-Passwort auf einer Konferenzfolie, ein Befehl in einem Video-Tutorial, ein Fehler in einem
Bildschirmfoto: Drücken Sie <kbd>Super</kbd>+<kbd>Umschalt</kbd>+<kbd>T</kbd>, ziehen Sie einen Rahmen und fügen
Sie ein. Mit Tesseract, das [über 100 Sprachen](https://tesseract-ocr.github.io/tessdoc/Data-Files-in-different-versions.html)
unterstützt (für Deutsch `tesseract-ocr-deu` installieren und `deu` als Sprache verwenden).

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/ocr-dark.png?v=0.7.0">
  <img src="docs/screenshots/ocr-light.png?v=0.7.0" alt="Die Texterkennung zeigt Text aus dem Foto einer Konferenzfolie: ein WLAN-Passwort und einen Link">
</picture>

### 🔎 Was belegt das?

„Target is busy“ beim Auswerfen eines USB-Sticks, „address already in use“ beim Start eines Entwicklungsservers:
Wählen Sie das Laufwerk, die Datei, den Ordner oder den Port und sehen Sie, welche Apps ihn belegen und was sie
geöffnet haben. Schließen Sie eine App mit einem Klick (sie wird zuerst freundlich gebeten, sich zu beenden, und
nur wenn sie nicht reagiert, sofort beendet), oder lassen Sie **Auswerfen** sie schließen und für Sie auswerfen.
Teile des Systems selbst werden nie zum Schließen angeboten.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/using-dark.png?v=0.7.0">
  <img src="docs/screenshots/using-light.png?v=0.7.0" alt="Die Seite Was belegt das? mit den Apps, die ein USB-Laufwerk belegen">
</picture>

### 🌙 Warum keine Bereitschaft?

Der Laptop warm in der Tasche, oder morgens der Akku leer? Sehen Sie, was den Computer gerade wach hält (eine App,
ein Video, „Wach halten“, eine Energieeinstellung, ein externer Monitor), jede Bereitschaftsphase der letzten
Woche mit Beginn, Ende und Akkuverbrauch sowie die USB-Geräte, die ihn wecken dürfen – mit einem Schalter für das
Gerät, das ihn immer wieder weckt.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/sleep-dark.png?v=0.7.0">
  <img src="docs/screenshots/sleep-light.png?v=0.7.0" alt="Die Seite Warum keine Bereitschaft? mit dem, was den Computer wach hält, und den letzten Bereitschaftsphasen">
</picture>

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

## Häufige Fragen

**Läuft es unter KDE, Sway oder Hyprland?**
Voltkit wird unter GNOME (Zorin OS 18) entwickelt und getestet. Es nutzt die Standard-XDG-Portale, daher sollten
KDE Plasma und andere Desktops funktionieren, sie wurden aber noch nicht getestet. Rückmeldungen sind willkommen!
Auf Desktops ohne GlobalShortcuts-Portal belegen Sie `voltkit trigger ocr|color|snippets` selbst mit Tasten.

**Ist es sicher, Kürzel beim Tippen einzuschalten?**
Sie sind standardmäßig aus. Wayland verbirgt Tastendrücke zwischen Apps, daher brauchen Kürzel beim Tippen
Tastaturzugriff, den Sie beim Einschalten einmal mit Ihrem Passwort erlauben. Voltkit behält nur die letzten
paar Zeichen im Speicher und speichert oder sendet nie, was Sie tippen. Die Bausteinauswahl funktioniert ganz
ohne das.

**Wie ändere ich die Sprache?**
Voltkit folgt der Sprache Ihrer Sitzung. Um eine andere nur für Voltkit auszuprobieren, starten Sie es im
Terminal mit `LANGUAGE=en voltkit` (oder `es`, `de`, `fr`, `zh_CN`).

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

Öffnen Sie Voltkit dann aus Ihren Apps. Das ist alles: Es startet seinen Hintergrunddienst (der ab jetzt auch bei
jeder Anmeldung startet) und fügt unter GNOME seine Tastenkürzel hinzu. `voltkit doctor` prüft, ob alles
eingerichtet ist. Kürzel wie `;date` überall zu tippen ist optional: Schalten Sie **Kürzel beim Tippen** auf der
Seite „Textbausteine“ ein (fragt einmal nach Ihrem Passwort und wirkt sofort).

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

## Hintergrunddienst &amp; globale Tastenkürzel

```bash
voltkit daemon                    # im Vordergrund ausführen
voltkit daemon enable-autostart   # bei jeder Anmeldung starten
voltkit daemon status             # läuft er? welches Tastenkürzel-Backend?
voltkit daemon stop
```

Der Dienst belegt `io.github.arios4k.Voltkit.Daemon` auf dem Sitzungsbus, daher läuft immer nur einer. Er belegt
die Tastenkürzel aus Ihrer Konfiguration auf eine dieser Arten:

| Desktop | Wie die Tastenkürzel belegt werden |
|---|---|
| GNOME 48+, KDE Plasma 5.27+, Hyprland (xdg-desktop-portal ≥ 1.19) | **GlobalShortcuts-Portal**: Der Desktop bittet Sie einmal um Zustimmung |
| Älteres GNOME (z. B. 46) und Distributionen auf GNOME-Basis | `voltkit shortcuts install` fügt sie unter *Einstellungen → Tastatur → Eigene Tastenkürzel* hinzu (`--dry-run` zeigt sie vorab; `shortcuts remove` macht es rückgängig) |
| Sway, river und andere | Belegen Sie `voltkit trigger ocr` / `voltkit trigger color` in der Konfiguration Ihres Compositors |

`voltkit trigger <aktion>` (`ocr`, `color` oder `snippets`) schickt die Aktion an den laufenden Dienst oder führt
sie direkt aus, wenn keiner läuft. Ergebnisse und Fehler erscheinen als Desktop-Benachrichtigungen.

## Textbausteine

**Bausteine verwalten:** Verwenden Sie auf der Seite „Textbausteine“ **Baustein hinzufügen**, klicken Sie auf einen
Baustein, um ihn zu bearbeiten, oder löschen Sie ihn (mit Rückgängig). Änderungen werden in
`~/.config/voltkit/config.toml` gespeichert und erreichen Kürzel beim Tippen sofort. Wenn Sie diese Datei von Hand
bearbeiten, führen Sie `voltkit daemon reload` aus.

**Bausteinauswahl** (ohne Einrichtung): Drücken Sie `Super+Umschalt+E`, tippen Sie zum Suchen und drücken Sie die
Eingabetaste. Bausteine mit `{arg:NAME}`-Parametern zeigen ein kleines Formular. Das Ergebnis wird in die App
eingefügt, die Sie verwendet haben, und Ihre Zwischenablage danach wiederhergestellt. Beim ersten Einfügen wird um
die Fernsteuerungsberechtigung gebeten (so lässt Wayland eine App für Sie Strg+V drücken), und die Berechtigung
wird gespeichert.

**Kürzel beim Tippen** (optional): Tippen Sie `;date` in einer beliebigen App, und es wird zu `2026-10-27`.
Schalten Sie **Kürzel beim Tippen** auf der Seite „Textbausteine“ ein. Wayland verbirgt Tastendrücke vor anderen
Apps, daher fragt es beim ersten Mal nach Ihrem Passwort, um Tastaturzugriff zu erlauben, und wirkt dann sofort:
kein Terminal, kein Abmelden. Beim Ausschalten wird angeboten, den Zugriff wieder zu entfernen. Im Terminal geht
dasselbe mit `voltkit expander setup --apply` und `voltkit expander remove-access`.

Der Tastaturzugriff erlaubt Programmen, die die Person am Computer ausführt, die Tastatur zu lesen – nicht nur
Voltkit. Voltkit behält nur die letzten paar Zeichen im Speicher und protokolliert oder speichert nie, was Sie
tippen. Details in `man voltkit-expander`.

Für Terminals, die mit Strg+Umschalt+V einfügen, setzen Sie `paste_shortcut` im Abschnitt `[expander]` der
Konfiguration.

### Hilfsprogramme

Voltkit ruft Standardwerkzeuge auf, wenn sie installiert sind, und bringt daher keine schweren eigenen
Abhängigkeiten mit. Prüfen Sie jederzeit, was fehlt, samt Installationsbefehl für Ihre Distribution:

```bash
voltkit doctor
```

Die Zentrale zeigt ein Banner, wenn eine Funktion Aufmerksamkeit braucht.

| Funktion | Werkzeuge |
|---|---|
| Texterkennung | `tesseract` (+ Sprachpakete wie `tesseract-ocr-deu`) |
| Bereichsaufnahme, Pipette | das Screenshot-Portal des Desktops; die Bereichsaufnahme weicht auf `grim`+`slurp`, `gnome-screenshot`, `spectacle` oder `maim` aus |
| Zwischenablage (Dienst, Befehlszeile) | `wl-clipboard` (Wayland) oder `xclip` (X11). Ohne sie wird das Ergebnis trotzdem in einer Benachrichtigung gezeigt |
| Komprimierung | eingebaut für PNG; `jpegoptim` (JPEG), `cwebp` (WebP), `ffmpeg` (MP4) |

## Befehlszeile

Jedes Modul funktioniert auch ohne Oberfläche, sodass Sie es auf jedem Desktop auf ein Tastenkürzel legen können:

```bash
voltkit                       # die Zentrale öffnen
voltkit ocr --lang deu        # Bereich auswählen → Text in der Zwischenablage
voltkit pick-color            # Pipette → HEX in der Zwischenablage
voltkit color '#3584e4'       # HEX / RGB / HSL + Kontrast auf Weiß/Schwarz
voltkit compress *.png --quality 75
voltkit expand 'Hallo {arg:who}, {date:%A}' --arg who=Sam
voltkit qr make https://example.com  # QR-Code im Terminal (oder --output code.png)
voltkit qr scan               # einen QR-Code auf dem Bildschirm lesen
voltkit ruler                 # auf dem Bildschirm messen
voltkit awake 1h30m           # den Computer wach halten (on / off / ohne Argument für den Status)
voltkit using /media/$USER/USB  # was belegt dieses Laufwerk? (oder ein Port: voltkit using 3000)
voltkit sleep                 # warum keine Bereitschaft, und was hat ihn geweckt
voltkit report                # dieser Computer, seine Prüfungen und letzten Änderungen (--memory: Steckplätze und Aufrüstung)
voltkit shortcuts who super+v # was ein Tastenkürzel tut (auch: shortcuts list, shortcuts clashes)
voltkit smart-paste           # die Zwischenablage umgewandelt einfügen
wl-paste | voltkit convert json  # die Umwandlungen von „Intelligentes Einfügen“ in Skripten (voltkit convert listet sie)
```

Befehle, ihre Argumente (`on`, `off`, `json` …) und die Konfigurationsschlüssel sind in allen Sprachen gleich;
Hilfe und Meldungen erscheinen in Ihrer.

Die vollständige Dokumentation steht in den Handbuchseiten: `man voltkit`, `man voltkit-expand` und so weiter.

### Handbuchseiten

Die Seiten in [`man/`](man/) werden aus den Befehlszeilen-Definitionen erzeugt und stimmen daher immer mit
`--help` überein; die Übersetzungen liegen in `man/de/`, `man/es/`, `man/fr/` und `man/zh_CN/`. Installieren Sie sie neben dem Programm, wo
`man` sie automatisch findet:

```bash
mkdir -p ~/.cargo/share/man/man1 ~/.cargo/share/man/de/man1
cp man/*.1 ~/.cargo/share/man/man1/ && cp man/de/*.1 ~/.cargo/share/man/de/man1/
```

Nach einer Änderung an einem Befehl oder seinem Hilfetext erzeugen Sie sie neu; bis dahin schlägt `cargo test`
fehl:

```bash
cargo run -- manpages man
```

## Konfiguration

`~/.config/voltkit/config.toml`. Jeder Schlüssel ist optional; siehe
[`data/config.example.toml`](data/config.example.toml).

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
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/about-dark.png?v=0.7.0">
  <img src="docs/screenshots/about-light.png?v=0.7.0" width="640" alt="Das Über-Fenster von Voltkit mit App-Symbol, Version und Autor Alexis Rios-Negron">
</picture>

## Voltkit unterstützen

Wenn Voltkit für dich hilfreich war, freue ich mich über eine [Spende in beliebiger Höhe](https://paypal.me/AlexisRios). Sie hilft dabei, das Projekt am Laufen zu halten und weiterzuentwickeln. Jeder Beitrag wird dankbar angenommen.

Voltkit soll auch in Zukunft kostenlos und für alle zugänglich sein. Vielen Dank fürs Nutzen und für jede Unterstützung!

## Lizenz

MIT © Alexis Rios-Negron
