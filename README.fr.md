<div align="center">

<img src="docs/icon.png" width="128" alt="Icône de Voltkit : un éclair qui croise une clé à molette">

# Voltkit

[English](README.md) · [Español](README.es.md) · [Deutsch](README.de.md) · **Français** · [简体中文](README.zh-CN.md)

**Des outils puissants pour votre bureau Linux.**
OCR d’écran, un lecteur de codes QR, un sélecteur de couleur, une règle d’écran, un compresseur d’images et de
vidéos, un expanseur de texte, le collage intelligent, Rester éveillé, un éditeur de hosts et d’environnement, un
détecteur de « qu’est-ce qui l’utilise ? », un gestionnaire d’autorisations des applications, un diagnostic de la
veille, un rapport système, un détective de raccourcis et un réglage du GPU, dans une seule application.

[![Dernière version](https://img.shields.io/github/v/release/arios4k/voltkit?label=version&color=7c3aed)](https://github.com/arios4k/voltkit/releases/latest)
[![CI](https://github.com/arios4k/voltkit/actions/workflows/ci.yml/badge.svg)](https://github.com/arios4k/voltkit/actions/workflows/ci.yml)
[![Manuel](https://img.shields.io/badge/Manuel-lire-2ea44f)](https://arios4k.github.io/voltkit/manual/)
[![Licence : MIT](https://img.shields.io/badge/licence-MIT-blue)](LICENSE)
[![Installer avec apt](https://img.shields.io/badge/installer-apt-orange)](#install-apt)
[![Installer sous Fedora](https://img.shields.io/badge/installer-Fedora-51a2da)](#install-fedora)
[![Fait avec Rust](https://img.shields.io/badge/fait%20avec-Rust-dea584?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Soutenir sur PayPal](https://img.shields.io/badge/PayPal-Support-00457C?logo=paypal&logoColor=white)](https://paypal.me/AlexisRios)

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/color-dark.png?v=0.11.0">
  <img src="docs/screenshots/color-light.png?v=0.11.0" alt="Le sélecteur de couleur de Voltkit affichant #7C3AED avec ses valeurs HEX, RGB et HSL et les notes de contraste WCAG">
</picture>

</div>

> Voltkit est disponible en français, anglais, espagnol, allemand et chinois simplifié, et utilise automatiquement la
> langue de votre système : l’application, la ligne de commande et les pages de manuel (`man voltkit`). Les captures
> d’écran de cette page montrent la version anglaise.

## Une application au lieu de dix-huit

Obtenir ces outils de base sous Linux oblige souvent à chercher une petite application pour chaque besoin, chacune avec
son apparence, ses réglages et ses raccourcis. Voltkit les réunit dans une seule application GTK4 native, avec un seul
jeu de raccourcis :

| Au lieu de… | Voltkit vous offre | Raccourci |
|---|---|---|
| rien : il n’existait aucun écran pour cela | **Autorisations des applications** : modifiez ou oubliez les réponses données aux applications qui demandaient des captures d’écran, la caméra, votre position, etc. | — |
| un compresseur d’images à part, plus des formules ffmpeg obscures | **Compresseur de fichiers** : déposez des fichiers PNG, JPEG, WebP ou MP4 pour supprimer leurs métadonnées et les alléger | glisser-déposer |
| un sélecteur de couleur à part | **Sélecteur de couleur** : une pipette pour n’importe quel pixel, HEX/RGB/HSL, vérification du contraste WCAG et historique | <kbd>Super</kbd>+<kbd>Maj</kbd>+<kbd>C</kbd> |
| des boucles de shell avec `mv` ou un renommeur à part | **Renommage en lot** : renomme beaucoup de fichiers d’un coup avec chercher-remplacer, expressions régulières et numérotation, avec aperçu et annulation | — |
| un redimensionneur d’images à part | **Redimensionneur d’images** : redimensionne beaucoup d’images d’un coup, en gardant leur forme et sans toucher aux originaux | — |
| un outil de réglage du GPU à part, plus les utilitaires des fabricants | **Réglage du GPU** : surveillez les cartes graphiques NVIDIA, AMD et Intel et modifiez limites de puissance, ventilateurs et courbes de ventilateur, températures cibles, fréquences et tension, chaque modification étant prévisualisée et annulée sans votre confirmation | — |
| modifier `/etc/hosts` et `~/.profile` dans un terminal | **Hôtes et environnement** : activez et désactivez des entrées hosts, et gérez les variables d’environnement de votre session | — |
| une extension du shell pour garder l’écran allumé | **Rester éveillé** : empêchez votre ordinateur de se mettre en veille ou de se verrouiller de 30 minutes à 4 heures, ou jusqu’à ce que vous le désactiviez | — |
| un stylo de présentation à part | **Zoom et annotations** : fige l’écran, zoome et dessine dessus avec un stylo, des flèches, des cadres et du texte | <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>Z</kbd> |
| un lecteur de QR sur votre téléphone ou un site web de QR | **Code QR** : lisez un code QR n’importe où à l’écran, ou transformez du texte, des liens et des informations Wi-Fi en code QR | <kbd>Super</kbd>+<kbd>Maj</kbd>+<kbd>Q</kbd> |
| une règle d’écran à part, ou des captures d’écran et un éditeur d’images | **Règle d’écran** : figez l’écran et mesurez n’importe quoi en pixels | <kbd>Super</kbd>+<kbd>Maj</kbd>+<kbd>R</kbd> |
| deviner pourquoi un raccourci clavier ne fait rien | **Détective de raccourcis** : appuyez sur un raccourci pour voir à qui il appartient, trouvez les conflits, choisissez une combinaison libre et changez-la | — |
| des convertisseurs en ligne pour le JSON, les tableaux et la casse | **Collage intelligent** : collez le presse-papiers converti : du JSON mis en forme, un tableau de tableur en Markdown, un lien sans suivi, du texte d’un PDF sans retours à la ligne cassés, ou le texte d’une image copiée | <kbd>Super</kbd>+<kbd>Maj</kbd>+<kbd>V</kbd> |
| une pile de commandes de terminal pour répondre à « quel matériel ai-je ? » | **Rapport système** : votre matériel expliqué, avec vérifications et correctifs pour les pilotes et la santé des disques, températures, mises à jour du micrologiciel, extensions de mémoire, ce qui a changé depuis une mise à jour, et un rapport qu’on peut partager sans risque | — |
| un expanseur de texte à part | **Expansion de texte** : un sélecteur d’extraits avec recherche, plus des déclencheurs saisis facultatifs comme `;date` | <kbd>Super</kbd>+<kbd>Maj</kbd>+<kbd>E</kbd> |
| une application d’OCR à part | **Extracteur de texte** : tracez un rectangle autour d’un texte à l’écran, même dans des vidéos et des images, et il arrive dans votre presse-papiers | <kbd>Super</kbd>+<kbd>Maj</kbd>+<kbd>T</kbd> |
| `lsof`, `fuser` et `ss` dans un terminal | **Qu’est-ce qui l’utilise ?** : voyez quelles applications gardent un fichier, un dossier, un disque ou un port occupé, et fermez-les | — |
| fouiller dans les journaux système dans un terminal | **Pourquoi ne s’est-il pas mis en veille ?** : voyez ce qui maintient l’ordinateur éveillé, ce qui l’a réveillé, et combien de batterie il a utilisé pendant la veille | — |

## Manuel

**Pour apprendre à utiliser Voltkit, lisez le [manuel de Voltkit](https://arios4k.github.io/voltkit/manual/)** (en anglais). Il explique chaque outil pas à pas, avec les raccourcis clavier, l’utilisation en ligne de commande, la configuration et le dépannage. Il s’ouvre aussi depuis l’application avec **F1** ou **Aide et manuel** dans le menu.

## Pourquoi Voltkit

- **Conçu pour Wayland.** Il utilise les portails du bureau pour les captures d’écran, la pipette et le collage : pas
  de bidouilles X11, pas d’extensions du shell.
- **Respectueux de la vie privée.** Tout s’exécute en local, sans accès au réseau ni télémétrie. Les déclencheurs
  saisis ne gardent en mémoire que les derniers caractères et ne journalisent ni ne stockent jamais ce que vous tapez.
- **Petit et natif.** Un binaire de 4 Mo (un paquet de 1,4 Mo), GTK4 + libadwaita, mode clair et sombre.
- **Scriptable.** Chaque fonctionnalité est aussi une commande (`voltkit ocr`, `voltkit color '#3584e4'`,
  `voltkit compress *.jpg`), avec des pages de manuel pour toutes.
- **Auto-vérifiant.** `voltkit doctor` vous dit exactement ce qui manque et la commande pour l’installer.
- **Paquets signés.** Installez et mettez à jour via apt depuis un dépôt signé GPG.
- **Dans votre langue.** Français, anglais, espagnol, allemand et chinois simplifié, selon la langue de votre système.

Voltkit est développé et testé sous GNOME (Zorin OS 18). Il utilise les portails XDG standard, donc KDE Plasma et d’autres bureaux devraient aussi fonctionner ; les retours sont bienvenus. D’autres réponses se trouvent dans la [FAQ](https://arios4k.github.io/voltkit/manual/faq.html) du manuel.

<sub>Captures d’écran réalisées sous Zorin OS 18 avec les palettes claire et sombre de Voltkit. Régénérez-les avec
`docs/screenshots/capture.sh` (ou `docs/screenshots/capture.sh fr` pour l’application en français ; aussi `es`, `de` et `zh_CN`).</sub>


<a id="install-apt"></a>

## Installation (Ubuntu, Debian et dérivées)

Pour Ubuntu 24.04+, Debian 13+, Linux Mint 22, Zorin OS 18, Pop!_OS 24.04 et autres dérivées
sur Intel/AMD 64 bits :

```bash
sudo curl -fsSLo /usr/share/keyrings/voltkit-archive-keyring.gpg \
  https://arios4k.github.io/voltkit/voltkit-archive-keyring.gpg
echo "deb [signed-by=/usr/share/keyrings/voltkit-archive-keyring.gpg] https://arios4k.github.io/voltkit stable main" \
  | sudo tee /etc/apt/sources.list.d/voltkit.list
sudo apt update && sudo apt install voltkit
```

apt installe les outils nécessaires (Tesseract, wl-clipboard, le portail du bureau) avec Voltkit, et les mises à jour
arrivent avec `sudo apt upgrade`.

Ouvrez ensuite Voltkit depuis vos applications : il configure lui-même son service en arrière-plan et ses raccourcis clavier. Consultez [Install](https://arios4k.github.io/voltkit/manual/install.html) et [A tour of the app](https://arios4k.github.io/voltkit/manual/tour.html) dans le manuel pour la suite.

<a id="install-fedora"></a>

## Installation (Fedora)

Pour Fedora 43 et ultérieures sur Intel/AMD 64 bits, depuis le [dépôt COPR](https://copr.fedorainfracloud.org/coprs/arios/voltkit/) :

```bash
sudo dnf copr enable arios/voltkit
sudo dnf install voltkit
```

Ouvrez ensuite Voltkit depuis vos applications ; il se configure de la même façon. Les mises à jour arrivent avec
`sudo dnf upgrade`.

## Installation (depuis les sources)

Prérequis : Rust 1.85+, GTK ≥ 4.14, libadwaita ≥ 1.5.

```bash
# Debian/Ubuntu
sudo apt install libgtk-4-dev libadwaita-1-dev
# Fedora
sudo dnf install gtk4-devel libadwaita-devel
# Arch
sudo pacman -S gtk4 libadwaita
```

```bash
cargo install --path .     # installe dans ~/.cargo/bin/voltkit
voltkit
```

Installez le binaire avant de configurer les raccourcis ou le démarrage automatique : les deux enregistrent le chemin du
binaire.

Pour ajouter Voltkit à la grille des applications avec son icône :

```bash
install -Dm644 data/icons/io.github.arios4k.Voltkit.svg ~/.local/share/icons/hicolor/scalable/apps/io.github.arios4k.Voltkit.svg
install -Dm644 data/icons/io.github.arios4k.Voltkit-symbolic.svg ~/.local/share/icons/hicolor/symbolic/apps/io.github.arios4k.Voltkit-symbolic.svg
sed "s|^Exec=voltkit|Exec=$HOME/.cargo/bin/voltkit|" data/io.github.arios4k.Voltkit.desktop > ~/.local/share/applications/io.github.arios4k.Voltkit.desktop
```

## Traductions

Voltkit est traduit en espagnol, allemand, français et chinois simplifié. Les textes se trouvent dans des fichiers
gettext ordinaires dans [`po/`](po/), intégrés au binaire : il n’y a donc rien de plus à installer. Pour ajouter une
langue ou corriger une traduction, voir [`po/README.md`](po/README.md).

## Organisation du projet

```
src/
  main.rs            point d’entrée de la CLI (clap) → interface graphique sans sous-commande
  i18n.rs            traductions : catalogues .po intégrés, langue du système
  app.rs             centre de contrôle GTK4 / libadwaita
  daemon.rs          service en arrière-plan : API D-Bus, raccourcis du portail, démarrage automatique
  actions.rs         actions de raccourci (OCR, pipette) avec notifications
  shortcuts.rs       conversion des accélérateurs + solution de repli pour les raccourcis GNOME
  picker.rs          fenêtre du sélecteur d’extraits
  snippets_ui.rs     page Expansion de texte : liste et éditeur d’extraits
  qr_ui.rs           page Code QR
  ruler.rs           règle d’écran (superposition sur une capture figée)
  awake.rs           Rester éveillé (portail Inhibit, tenu par le démon)
  awake_ui.rs        page Rester éveillé
  system_ui.rs       page Hôtes et environnement
  usage_ui.rs        page Qu’est-ce qui l’utilise ?
  permissions_ui.rs  page Autorisations des applications
  sleep_ui.rs        page Pourquoi ne s’est-il pas mis en veille ?
  gpu_ui.rs          page Réglage du GPU : mesures, rapport de compatibilité, contrôles
  gpu_cli.rs         `voltkit gpu`, l’assistant root et le service en arrière-plan
  sysreport_ui.rs    page Rapport système
  detective_ui.rs    page Détective de raccourcis
  smartpaste_ui.rs   fenêtre et page du collage intelligent
  expander.rs        déclencheurs saisis (lecture du clavier via evdev, clavier virtuel uinput)
  paste.rs           coller dans l’application active en restaurant le presse-papiers
  portal.rs          appels aux portails XDG (capture d’écran, PickColor) et notifications
  config.rs          configuration TOML avec valeurs par défaut
  system.rs          utilitaires Wayland/X11 : presse-papiers, détection d’outils, lanceur de processus
  modules/
    ocr.rs           capture de zone + Tesseract
    color.rs         analyse, HSL, luminance et contraste WCAG
    compress.rs      détection du format + chaînes de compression par format
    snippets.rs      moteur de modèles d’extraits
    qr.rs            lecture et création de codes QR
    hosts.rs         analyse de /etc/hosts qui préserve tout ce qu’elle ne modifie pas
    envvars.rs       variables d’environnement de la session (environment.d)
    usage.rs         trouve et ferme les processus qui utilisent un chemin ou un port (/proc)
    permissions.rs   le magasin d’autorisations du portail du bureau
    sleep.rs         blocages de la veille, historique de veille (journal, UPower) et appareils de réveil
    gpu.rs           détection des GPU, capteurs et ce que chaque carte prend en charge (NVIDIA, AMD, Intel)
    gpu_tune.rs      modifications GPU validées et transactionnelles, courbes de ventilateur, protection au démarrage
    nvml.rs          liaison avec la bibliothèque de gestion de NVIDIA
    sysreport.rs     matériel, vérifications, santé des disques, capteurs, micrologiciel, instantanés, mémoire SMBIOS
    keybindings.rs   tous les raccourcis GSettings : propriétaires, conflits, combinaisons libres, modifications
    smartpaste.rs    conversions de texte du collage intelligent
  manpages.rs        générateur de pages de manuel (clap_mangen + sections écrites à la main)
  screenshots.rs     produit les captures d’écran du README (commande cachée `screenshots`)
po/                  traductions (espagnol, allemand, français, chinois simplifié) et update.py pour les actualiser
man/                 pages de manuel générées (man/es, man/de, man/fr, man/zh_CN : traduites)
packaging/apt/        générateur du dépôt APT et sa page d’accueil
docs/screenshots/    captures d’écran du README et capture.sh pour les régénérer
docs/ARCHITECTURE.md notes de conception et feuille de route
```

## Contribuer

```bash
cargo test
cargo clippy
python3 po/update.py --check   # traductions à jour et complètes
```

Après avoir modifié une commande ou son texte d’aide, régénérez les pages de manuel de [`man/`](man/) avec `cargo run -- manpages man` ; `cargo test` échoue tant que ce n’est pas fait.

Voir [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) pour l’API D-Bus et la feuille de route. Un écran de réglages est prévu. Les rapports de bogues et les tests sur d’autres bureaux sont
particulièrement bienvenus.

## Publier une version

1. Augmentez `version` dans `Cargo.toml`, exécutez `cargo run -- manpages man` et `python3 po/update.py`, et faites un commit.
2. Créez et poussez l’étiquette : `git tag v0.2.0 && git push origin v0.2.0`.

Le [workflow de publication](.github/workflows/release.yml) construit le `.deb`, publie une version GitHub avec lui, et
reconstruit le dépôt APT signé sur GitHub Pages. Il signe avec la clé du secret `APT_SIGNING_KEY` (empreinte
`65E8 4B8D 114C 6147 9CCA 4DDC EC0F 6B66 E480 2360`). Pour construire le dépôt en local :
`APT_KEY_ID=<empreinte> packaging/apt/build-repo.sh target/debian site`.

## Auteur

Créé par **Alexis Rios-Negron**.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/about-dark.png?v=0.11.0">
  <img src="docs/screenshots/about-light.png?v=0.11.0" width="640" alt="La fenêtre À propos de Voltkit montrant l’icône de l’application, la version et l’auteur Alexis Rios-Negron">
</picture>

## Soutenir Voltkit

Si Voltkit vous a été utile, pensez à [faire un don du montant de votre choix](https://paypal.me/AlexisRios) pour aider à faire vivre le projet et à poursuivre son développement. Chaque contribution est sincèrement appréciée.

L’objectif est que Voltkit reste gratuit et accessible à tout le monde. Merci de l’utiliser et pour tout le soutien que vous pouvez apporter !

## Licence

MIT © Alexis Rios-Negron
