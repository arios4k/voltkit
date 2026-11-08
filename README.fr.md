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
[![Licence : MIT](https://img.shields.io/badge/licence-MIT-blue)](LICENSE)
[![Installer avec apt](https://img.shields.io/badge/installer-apt-orange)](#install-apt)
[![Installer sous Fedora](https://img.shields.io/badge/installer-Fedora-51a2da)](#install-fedora)
[![Fait avec Rust](https://img.shields.io/badge/fait%20avec-Rust-dea584?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Soutenir sur PayPal](https://img.shields.io/badge/PayPal-Support-00457C?logo=paypal&logoColor=white)](https://paypal.me/AlexisRios)

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/color-dark.png?v=0.7.0">
  <img src="docs/screenshots/color-light.png?v=0.7.0" alt="Le sélecteur de couleur de Voltkit affichant #7C3AED avec ses valeurs HEX, RGB et HSL et les notes de contraste WCAG">
</picture>

</div>

> Voltkit est disponible en français, anglais, espagnol, allemand et chinois simplifié, et utilise automatiquement la
> langue de votre système : l’application, la ligne de commande et les pages de manuel (`man voltkit`). Les captures
> d’écran de cette page montrent la version anglaise.

## Une application au lieu de quinze

Obtenir ces outils de base sous Linux oblige souvent à chercher une petite application pour chaque besoin, chacune avec
son apparence, ses réglages et ses raccourcis. Voltkit les réunit dans une seule application GTK4 native, avec un seul
jeu de raccourcis :

| Au lieu de… | Voltkit vous offre | Raccourci |
|---|---|---|
| rien : il n’existait aucun écran pour cela | **Autorisations des applications** : modifiez ou oubliez les réponses données aux applications qui demandaient des captures d’écran, la caméra, votre position, etc. | — |
| un compresseur d’images à part, plus des formules ffmpeg obscures | **Compresseur de fichiers** : déposez des fichiers PNG, JPEG, WebP ou MP4 pour supprimer leurs métadonnées et les alléger | glisser-déposer |
| un sélecteur de couleur à part | **Sélecteur de couleur** : une pipette pour n’importe quel pixel, HEX/RGB/HSL, vérification du contraste WCAG et historique | <kbd>Super</kbd>+<kbd>Maj</kbd>+<kbd>C</kbd> |
| un outil de réglage du GPU à part, plus les utilitaires des fabricants | **Réglage du GPU** : surveillez les cartes graphiques NVIDIA, AMD et Intel et modifiez limites de puissance, ventilateurs et courbes de ventilateur, températures cibles, fréquences et tension, chaque modification étant prévisualisée et annulée sans votre confirmation | — |
| modifier `/etc/hosts` et `~/.profile` dans un terminal | **Hôtes et environnement** : activez et désactivez des entrées hosts, et gérez les variables d’environnement de votre session | — |
| une extension du shell pour garder l’écran allumé | **Rester éveillé** : empêchez votre ordinateur de se mettre en veille ou de se verrouiller de 30 minutes à 4 heures, ou jusqu’à ce que vous le désactiviez | — |
| un lecteur de QR sur votre téléphone ou un site web de QR | **Code QR** : lisez un code QR n’importe où à l’écran, ou transformez du texte, des liens et des informations Wi-Fi en code QR | <kbd>Super</kbd>+<kbd>Maj</kbd>+<kbd>Q</kbd> |
| une règle d’écran à part, ou des captures d’écran et un éditeur d’images | **Règle d’écran** : figez l’écran et mesurez n’importe quoi en pixels | <kbd>Super</kbd>+<kbd>Maj</kbd>+<kbd>R</kbd> |
| deviner pourquoi un raccourci clavier ne fait rien | **Détective de raccourcis** : appuyez sur un raccourci pour voir à qui il appartient, trouvez les conflits, choisissez une combinaison libre et changez-la | — |
| des convertisseurs en ligne pour le JSON, les tableaux et la casse | **Collage intelligent** : collez le presse-papiers converti : du JSON mis en forme, un tableau de tableur en Markdown, un lien sans suivi, du texte d’un PDF sans retours à la ligne cassés, ou le texte d’une image copiée | <kbd>Super</kbd>+<kbd>Maj</kbd>+<kbd>V</kbd> |
| une pile de commandes de terminal pour répondre à « quel matériel ai-je ? » | **Rapport système** : votre matériel expliqué, avec vérifications et correctifs pour les pilotes et la santé des disques, températures, mises à jour du micrologiciel, extensions de mémoire, ce qui a changé depuis une mise à jour, et un rapport qu’on peut partager sans risque | — |
| un expanseur de texte à part | **Expansion de texte** : un sélecteur d’extraits avec recherche, plus des déclencheurs saisis facultatifs comme `;date` | <kbd>Super</kbd>+<kbd>Maj</kbd>+<kbd>E</kbd> |
| une application d’OCR à part | **Extracteur de texte** : tracez un rectangle autour d’un texte à l’écran, même dans des vidéos et des images, et il arrive dans votre presse-papiers | <kbd>Super</kbd>+<kbd>Maj</kbd>+<kbd>T</kbd> |
| `lsof`, `fuser` et `ss` dans un terminal | **Qu’est-ce qui l’utilise ?** : voyez quelles applications gardent un fichier, un dossier, un disque ou un port occupé, et fermez-les | — |
| fouiller dans les journaux système dans un terminal | **Pourquoi ne s’est-il pas mis en veille ?** : voyez ce qui maintient l’ordinateur éveillé, ce qui l’a réveillé, et combien de batterie il a utilisé pendant la veille | — |

## Fonctionnalités

### 🛡️ Autorisations des applications

Quand une application demande à faire des captures d’écran, à utiliser la caméra ou le microphone, ou à voir votre
position, le bureau se souvient de votre réponse. Autorisations des applications les liste par application : actionnez un interrupteur pour en modifier une, ou oubliez-la
pour que l’application redemande.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/permissions-dark.png?v=0.7.0">
  <img src="docs/screenshots/permissions-light.png?v=0.7.0" alt="La page Autorisations des applications listant des applications avec leurs autorisations de caméra et de capture d’écran">
</picture>

### 📦 Compresseur de fichiers

Déposez des fichiers sur la fenêtre ou exécutez `voltkit compress`. Voltkit supprime les données de localisation et les
autres métadonnées, compresse avec le meilleur outil pour chaque format, et écrit `nom.min.ext` à côté de l’original,
qu’il ne touche jamais. Dans nos tests : JPEG −43 %, PNG −23 % (sans perte), WebP −60 %, MP4 −77 %.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/compress-dark.png?v=0.7.0">
  <img src="docs/screenshots/compress-light.png?v=0.7.0" alt="Le compresseur de fichiers avec quatre fichiers compressés et leur gain de place">
</picture>

### 🎨 Sélecteur de couleur

Capturez n’importe quel pixel avec <kbd>Super</kbd>+<kbd>Maj</kbd>+<kbd>C</kbd> et le code HEX est déjà dans votre
presse-papiers. La page de couleur convertit entre HEX, RGB et HSL, note le contraste par rapport au blanc et au noir
selon les normes WCAG (AA / AAA), et garde en mémoire vos couleurs récentes.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/color-dark.png?v=0.7.0">
  <img src="docs/screenshots/color-light.png?v=0.7.0" alt="Le sélecteur de couleur avec un aperçu en direct, les formats HEX/RGB/HSL, les notes de contraste et les couleurs enregistrées">
</picture>

### 🎛️ Réglage du GPU

Surveillez en direct vos cartes graphiques NVIDIA, AMD et Intel, avec l’historique, et modifiez ce que le matériel
permet : la limite de puissance, une vitesse de ventilateur fixe ou une courbe, la température visée par la carte, des
décalages ou des limites de fréquence, et la tension. **Ce que cette carte prend en charge** liste chaque contrôle tel
que le pilote le signale, et explique pourquoi l’un manque et ce qui pourrait l’activer (un GPU de portable qui cache son
ventilateur, ou une carte AMD qui a besoin de `amdgpu.ppfeaturemask`). Les modifications sont vérifiées par rapport aux
limites de la carte elle-même, prévisualisées, appliquées par un assistant avec les droits root, et **annulées sans votre
confirmation** : fermer l’application ou un plantage les annule aussi. Les réglages enregistrés sont appliqués au
démarrage par un service en arrière-plan, qui ignore un profil dont le démarrage ne s’est jamais terminé, pour qu’un
mauvais profil ne vous enferme pas dans une boucle. `voltkit gpu` affiche le même rapport dans un terminal.

Modifiez les courbes de ventilateur en faisant glisser des points sur un graphique, et choisissez la température qu’elles suivent et les ventilateurs qu’elles pilotent. Enregistrez des réglages sous forme de **profils** qui changent tout seuls : sur batterie, pendant qu’une application tourne, sous une charge soutenue ou quand la carte chauffe. **Enregistrez une session** pendant qu’un jeu ou un test de performance tourne et comparez deux profils côte à côte : température, puissance, énergie et efficacité (fréquence active par watt, ou images par watt quand les images sont enregistrées). `voltkit gpu check` compare les mesures de Voltkit avec `nvidia-smi` et mesure le coût de la surveillance.

Vous l’avez essayé sur votre carte ? Un [rapport matériel](https://github.com/arios4k/voltkit/issues/new?template=gpu-hardware-report.yml) de ce qui a marché ou non est l’aide la plus utile pour l’instant.

L’application des modifications est nouvelle et n’a été testée jusqu’ici que sur du matériel simulé ; voir
[docs/GPU_TUNER.md](docs/GPU_TUNER.md) pour ce qui fonctionne, comment la sécurité est assurée et ce qui reste à faire.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/gpu-dark.png?v=0.10.4">
  <img src="docs/screenshots/gpu-light.png?v=0.10.4" alt="Réglage du GPU, onglet Surveillance : mesures en direct avec historique, pourquoi la fréquence est freinée et ce que la carte prend en charge (données simulées)">
</picture>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/gpu-2-dark.png?v=0.10.4">
  <img src="docs/screenshots/gpu-2-light.png?v=0.10.4" alt="Réglage du GPU, onglet Régler : limite de puissance, éditeur de courbe de ventilateur et autres contrôles (données simulées)">
</picture>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/gpu-3-dark.png?v=0.10.4">
  <img src="docs/screenshots/gpu-3-light.png?v=0.10.4" alt="Réglage du GPU, onglet Profils : profils enregistrés, règles automatiques, partage et import (données simulées)">
</picture>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/gpu-4-dark.png?v=0.10.4">
  <img src="docs/screenshots/gpu-4-light.png?v=0.10.4" alt="Réglage du GPU, onglet Sessions : sessions enregistrées, prix de l’électricité, suggestion de limite de puissance et comparaison (données simulées)">
</picture>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/gpu-vf-dark.png?v=0.10.4">
  <img src="docs/screenshots/gpu-vf-light.png?v=0.10.4" alt="Éditeur de courbe tension/fréquence du réglage du GPU avec des points déplaçables (données simulées)">
</picture>

<sub>La capture d’écran utilise des données simulées de cartes fictives.</sub>

### 📝 Hôtes et environnement

Activez et désactivez des entrées de `/etc/hosts`, ajoutez `myapp.test → 127.0.0.1` pour le développement local, et
définissez des variables d’environnement comme `EDITOR` ou un dossier `PATH` supplémentaire pour toute votre session de
bureau. Les entrées système sont protégées, et chaque enregistrement du fichier hosts conserve une sauvegarde.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/system-dark.png?v=0.7.0">
  <img src="docs/screenshots/system-light.png?v=0.7.0" alt="La page Hôtes et environnement avec des entrées hosts et des interrupteurs">
</picture>

### ☕ Rester éveillé

Vous regardez une vidéo, faites une présentation ou téléchargez ? Activez Rester éveillé pour 30 minutes, quelques
heures ou jusqu’à ce que vous le désactiviez, et votre ordinateur ne se mettra ni en veille ni en verrouillage. Cela
continue après la fermeture de Voltkit et s’arrête tout seul à la fin du délai.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/awake-dark.png?v=0.7.0">
  <img src="docs/screenshots/awake-light.png?v=0.7.0" alt="Rester éveillé avec son état actuel et les commandes de durée">
</picture>

### 📷 Code QR

Appuyez sur <kbd>Super</kbd>+<kbd>Maj</kbd>+<kbd>Q</kbd> et tracez un rectangle autour d’un code QR dans une vidéo, une
diapositive ou une page web : son texte ou son lien est dans votre presse-papiers. Dans l’autre sens, saisissez un
lien, du texte ou les informations de votre Wi-Fi et scannez le code avec votre téléphone. Copiez-le comme image ou
enregistrez-le en PNG pour l’imprimer. Ajoutez au milieu votre propre petit logo si vous le souhaitez : Voltkit relit
le code pour s’assurer qu’il se scanne toujours.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/qr-dark.png?v=0.7.0">
  <img src="docs/screenshots/qr-light.png?v=0.7.0" alt="La page Code QR avec un code généré pour un lien">
</picture>

### 📏 Règle d’écran

<kbd>Super</kbd>+<kbd>Maj</kbd>+<kbd>R</kbd> fige l’écran. Tracez pour mesurer des largeurs et des hauteurs en pixels
réels (maintenez Maj pour une ligne droite), et appuyez sur Ctrl+C pour copier la taille.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/ruler-dark.png?v=0.7.0">
  <img src="docs/screenshots/ruler-light.png?v=0.7.0" alt="La règle d’écran avec son action de lancement et ses commandes de mesure">
</picture>

### ⌨️ Détective de raccourcis

Vous avez appuyé sur un raccourci et rien ne s’est passé, ou autre chose s’est produit ? Appuyez dessus dans le
**Détective de raccourcis** pour voir toutes les actions qui l’utilisent : le gestionnaire de fenêtres, GNOME Shell,
les touches système, vos raccourcis personnalisés, les extensions du shell et Voltkit lui-même. Les conflits sont
listés pour vous, chacun avec des combinaisons libres vers lesquelles passer, et n’importe quel raccourci peut être
modifié, désactivé ou rétabli sur place. Fonctionne sous GNOME et les bureaux construits comme lui (Cinnamon, Budgie, MATE).

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/shortcuts-dark.png?v=0.7.0">
  <img src="docs/screenshots/shortcuts-light.png?v=0.7.0" alt="Le détective de raccourcis montrant que Super+V est utilisé par deux actions">
</picture>

### 📋 Collage intelligent

Appuyez sur <kbd>Super</kbd>+<kbd>Maj</kbd>+<kbd>V</kbd> au lieu de <kbd>Ctrl</kbd>+<kbd>V</kbd> et choisissez comment
coller ce que vous avez copié : du JSON mis en forme ou compacté, une plage de tableur en tableau Markdown ou CSV, un
lien sans ses paramètres de suivi, un paragraphe d’un PDF sans ses retours à la ligne cassés, une liste à puces ou
numérotée, ou du texte en MAJUSCULES, en casse de titre, en snake_case et plus encore. Vous avez copié une capture
d’écran ? Collez le texte qu’elle contient. Seules les conversions adaptées à ce que vous avez copié sont proposées,
chacune avec un aperçu, et votre presse-papiers reste tel quel. Les mêmes conversions fonctionnent dans les scripts :
`wl-paste | voltkit convert markdown-table`.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/smartpaste-dark.png?v=0.7.0">
  <img src="docs/screenshots/smartpaste-light.png?v=0.7.0" alt="La page Collage intelligent transformant une plage de tableur en tableau Markdown et en CSV">
</picture>

### 🖥️ Rapport système

Pas seulement une liste de caractéristiques : le **Rapport système** vérifie chaque élément matériel et explique ce que
cela signifie. La carte graphique utilise-t-elle le bon pilote (sinon, la commande exacte pour corriger) ? Le disque
est-il en bonne santé, et quel est son niveau d’usure ? Le processeur chauffe-t-il ou se ralentit-il lui-même ? Une mise
à jour du BIOS ou du micrologiciel d’un appareil est-elle en attente ? Combien d’emplacements mémoire sont libres, et
combien cet ordinateur peut-il en accueillir ?

Voltkit conserve aussi un instantané quotidien du matériel, des pilotes, du noyau et du micrologiciel : quand quelque
chose casse après une mise à jour, **Ce qui a changé** montre exactement ce qui est différent. Quand vous avez besoin
d’aide, **Copier le rapport** vous donne un résumé soigné pour un forum ou un rapport de bogue, sans numéros de série,
noms ni adresses réseau.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/sysreport-dark.png?v=0.7.0">
  <img src="docs/screenshots/sysreport-light.png?v=0.7.0" alt="La page Rapport système avec les vérifications, les changements récents et le matériel de l’ordinateur">
</picture>

### ⌨️ Expansion de texte

Appuyez sur <kbd>Super</kbd>+<kbd>Maj</kbd>+<kbd>E</kbd>, cherchez, appuyez sur Entrée, et l’extrait est collé dans
l’application que vous utilisiez. Les extraits peuvent contenir la date dans n’importe quel format, votre
presse-papiers et des champs à remplir. Vous préférez taper ? Activez les déclencheurs saisis et `;sig` devient votre
signature dans n’importe quelle application. Ajoutez, modifiez et supprimez des extraits directement dans
l’application : des boutons Insérer ajoutent la date, l’heure, le presse-papiers ou un champ à remplir, et un aperçu en
direct montre exactement ce qui sera collé.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/snippets-dark.png?v=0.7.0">
  <img src="docs/screenshots/snippets-light.png?v=0.7.0" alt="L’expansion de texte avec des extraits consultables et des commandes d’insertion">
</picture>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/picker-dark.png?v=0.7.0">
  <img src="docs/screenshots/picker-light.png?v=0.7.0" alt="Le sélecteur d’extraits listant des extraits avec des aperçus en direct">
</picture>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/snippet-editor-dark.png?v=0.7.0">
  <img src="docs/screenshots/snippet-editor-light.png?v=0.7.0" alt="L’éditeur d’extraits avec un déclencheur, un texte avec des boutons Insérer pour la date, l’heure, le presse-papiers et les champs à remplir, et un aperçu en direct">
</picture>

### 🔍 Extracteur de texte

Un mot de passe Wi-Fi sur une diapositive de conférence, une commande dans un tutoriel vidéo, une erreur dans une
capture d’écran : appuyez sur <kbd>Super</kbd>+<kbd>Maj</kbd>+<kbd>T</kbd>, tracez un rectangle et collez. Propulsé par
Tesseract, avec la prise en charge de
[plus de 100 langues](https://tesseract-ocr.github.io/tessdoc/Data-Files-in-different-versions.html).

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/ocr-dark.png?v=0.7.0">
  <img src="docs/screenshots/ocr-light.png?v=0.7.0" alt="L’extracteur de texte montrant le texte lu sur la photo d’une diapositive de conférence : un mot de passe Wi-Fi et un lien">
</picture>

### 🔎 Qu’est-ce qui l’utilise ?

« La cible est occupée » quand vous éjectez une clé USB, « adresse déjà utilisée » quand vous démarrez un serveur de
développement : choisissez le disque, le fichier, le dossier ou le port et voyez quelles applications le retiennent,
avec ce qu’elles ont d’ouvert. Fermez une application en un clic (on lui demande d’abord poliment de quitter, avec
Forcer à quitter seulement si elle refuse), ou laissez **Éjecter** les fermer et éjecter pour vous. Les éléments du
système lui-même ne sont jamais proposés à la fermeture.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/using-dark.png?v=0.7.0">
  <img src="docs/screenshots/using-light.png?v=0.7.0" alt="La page Qu’est-ce qui l’utilise ? listant les applications qui gardent un disque USB occupé">
</picture>

### 🌙 Pourquoi ne s’est-il pas mis en veille ?

Portable chaud dans votre sac, ou batterie vide au matin ? Voyez ce qui maintient l’ordinateur éveillé en ce moment (une
application, une vidéo, Rester éveillé, un réglage d’alimentation, un écran externe), chaque mise en veille de la
dernière semaine avec ce qui l’a déclenchée et terminée et la batterie utilisée, et les appareils USB autorisés à le
réveiller, avec un interrupteur pour arrêter celui qui n’arrête pas de le réveiller.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/sleep-dark.png?v=0.7.0">
  <img src="docs/screenshots/sleep-light.png?v=0.7.0" alt="La page Pourquoi ne s’est-il pas mis en veille ? listant ce qui maintient l’ordinateur éveillé et les mises en veille récentes">
</picture>

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

## FAQ

**Ça marche sous KDE, Sway ou Hyprland ?**
Voltkit est développé et testé sous GNOME (Zorin OS 18). Il utilise les portails XDG standard, donc KDE Plasma et
d’autres bureaux devraient fonctionner, mais ils n’ont pas encore été testés. Les retours sont bienvenus ! Sur les
bureaux sans le portail GlobalShortcuts, associez vous-même `voltkit trigger ocr|color|snippets` à des touches.

**Est-ce sûr d’activer les déclencheurs saisis ?**
Ils sont désactivés par défaut. Wayland cache les frappes entre les applications, donc les déclencheurs saisis ont
besoin d’un accès au clavier, que vous autorisez une fois avec votre mot de passe quand vous les activez. Voltkit ne
garde en mémoire que les derniers caractères et n’enregistre ni n’envoie jamais ce que vous tapez. Le sélecteur
d’extraits fonctionne sans rien de tout cela.

**Comment changer la langue ?**
Voltkit suit la langue de votre session (français, anglais, espagnol, allemand ou chinois simplifié). Pour en essayer
une autre pour Voltkit seulement, lancez-le depuis un terminal avec `LANGUAGE=en voltkit` (ou `es`, `de`, `fr`, `zh_CN`).

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

Ouvrez ensuite Voltkit depuis vos applications. C’est tout : il démarre son service en arrière-plan (qui démarre aussi
à chaque ouverture de session dès maintenant) et, sous GNOME, ajoute ses raccourcis clavier. `voltkit doctor` vérifie
que tout est en place. Taper des déclencheurs comme `;date` n’importe où est facultatif : activez les **Déclencheurs
saisis** dans la page Expansion de texte (votre mot de passe est demandé une fois et c’est effectif tout de suite).

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

## Service en arrière-plan et raccourcis globaux

```bash
voltkit daemon                    # exécuter au premier plan
voltkit daemon enable-autostart   # démarrer à chaque ouverture de session
voltkit daemon status             # en cours d’exécution ? quelle méthode de raccourcis ?
voltkit daemon stop
```

Le démon possède `io.github.arios4k.Voltkit.Daemon` sur le bus de session, donc un seul tourne à la fois.
Il associe les raccourcis de votre configuration de l’une de ces façons :

| Bureau | Comment les raccourcis sont associés |
|---|---|
| GNOME 48+, KDE Plasma 5.27+, Hyprland (xdg-desktop-portal ≥ 1.19) | **Portail GlobalShortcuts** : le bureau vous demande de les approuver une fois |
| GNOME plus ancien (p. ex. 46) et distributions basées sur GNOME | `voltkit shortcuts install` les ajoute à *Paramètres → Clavier → Raccourcis personnalisés* (`--dry-run` prévisualise ; `shortcuts remove` annule) |
| Sway, river, autres | Associez `voltkit trigger ocr` / `voltkit trigger color` dans la configuration de votre compositeur |

`voltkit trigger <action>` (`ocr`, `color` ou `snippets`) envoie l’action au démon en cours d’exécution, ou l’exécute
directement si aucun démon ne tourne. Les résultats et les erreurs apparaissent sous forme de notifications du bureau.

## Expansion de texte

**Gérer les extraits :** dans la page Expansion de texte, utilisez **Ajouter un extrait**, cliquez sur un extrait pour
le modifier, ou supprimez-le (avec Annuler). Les modifications sont enregistrées dans `~/.config/voltkit/config.toml`
et atteignent immédiatement les déclencheurs saisis. Si vous modifiez ce fichier à la main, exécutez
`voltkit daemon reload`.

**Sélecteur d’extraits** (sans configuration) : appuyez sur `Super+Maj+E`, tapez pour chercher, appuyez sur Entrée. Les
extraits avec des paramètres `{arg:NOM}` affichent un petit formulaire. Le résultat est collé dans l’application que
vous utilisiez, et votre presse-papiers est ensuite remis en place. Le premier collage demande l’autorisation de bureau
à distance (c’est ainsi que Wayland permet à une application d’appuyer sur Ctrl+V à votre place), et l’autorisation est
mémorisée.

**Déclencheurs saisis** (facultatifs) : tapez `;date` dans n’importe quelle application et cela devient `2026-10-27`.
Activez les **Déclencheurs saisis** dans la page Expansion de texte. Wayland cache les frappes aux autres applications,
donc la première fois votre mot de passe est demandé pour autoriser l’accès au clavier, puis cela fonctionne tout de
suite : pas de terminal, pas de déconnexion. En les désactivant, on vous propose de retirer à nouveau l’accès. Depuis un
terminal, c’est `voltkit expander setup --apply` et `voltkit expander remove-access`.

L’accès au clavier permet aux programmes lancés par la personne devant l’ordinateur de lire le clavier, pas seulement à
Voltkit. Voltkit ne garde en mémoire que les derniers caractères et ne journalise ni ne stocke jamais ce que vous tapez.
Voir `man voltkit-expander` pour les détails.

Pour les terminaux, qui collent avec Ctrl+Maj+V, définissez `paste_shortcut` dans la section `[expander]` de la
configuration.

### Outils d’exécution

Voltkit appelle des outils standard lorsqu’ils sont installés, il n’ajoute donc aucune dépendance lourde de son propre
chef. Vérifiez à tout moment ce qui manque, avec la commande d’installation pour votre distribution :

```bash
voltkit doctor
```

Le centre de contrôle affiche une bannière quand une fonctionnalité demande votre attention.

| Fonctionnalité | Outils |
|---|---|
| OCR | `tesseract` (+ paquets de langues comme `tesseract-ocr-eng`) |
| Capture de zone, pipette | le portail de capture d’écran du bureau ; la capture de zone se rabat sur `grim`+`slurp`, `gnome-screenshot`, `spectacle` ou `maim` |
| Presse-papiers (démon, CLI) | `wl-clipboard` (Wayland) ou `xclip` (X11). Sans lui, le résultat s’affiche quand même dans une notification |
| Compression | intégrée pour le PNG ; `jpegoptim` (JPEG), `cwebp` (WebP), `ffmpeg` (MP4) |

## Ligne de commande

Chaque module fonctionne aussi sans interface, vous pouvez donc l’associer à un raccourci sur n’importe quel bureau :

```bash
voltkit                       # ouvrir le centre de contrôle
voltkit ocr --lang eng        # sélectionner une zone → texte dans le presse-papiers
voltkit pick-color            # pipette → HEX dans le presse-papiers
voltkit color '#3584e4'       # HEX / RGB / HSL + contraste sur blanc/noir
voltkit compress *.png --quality 75
voltkit expand 'Hi {arg:who}, {date:%A}' --arg who=Sam
voltkit qr make https://example.com  # code QR dans le terminal (ou --output code.png)
voltkit qr scan               # lire un code QR à l’écran
voltkit ruler                 # mesurer à l’écran
voltkit awake 1h30m           # garder l’ordinateur éveillé (on / off / sans argument pour l’état)
voltkit using /media/$USER/USB  # qu’est-ce qui garde ce disque occupé ? (ou un port : voltkit using 3000)
voltkit sleep                 # pourquoi il ne s’est pas mis en veille, et ce qui l’a réveillé
voltkit report                # cet ordinateur, ses vérifications et ses changements récents (--memory : emplacements et extensions)
voltkit shortcuts who super+v # ce que fait un raccourci (aussi : shortcuts list, shortcuts clashes)
voltkit smart-paste           # coller le presse-papiers converti
wl-paste | voltkit convert json  # les conversions du collage intelligent dans les scripts (voltkit convert les liste)
```

La documentation complète se trouve dans les pages de manuel : `man voltkit`, `man voltkit-expand`, etc.

### Pages de manuel

Les pages de [`man/`](man/) sont générées à partir des définitions de la CLI, donc elles correspondent toujours à
`--help` ; les traductions sont dans `man/es/`, `man/de/`, `man/fr/` et `man/zh_CN/`. Installez-les à côté du binaire,
là où `man` les trouve automatiquement :

```bash
mkdir -p ~/.cargo/share/man/man1 ~/.cargo/share/man/fr/man1
cp man/*.1 ~/.cargo/share/man/man1/ && cp man/fr/*.1 ~/.cargo/share/man/fr/man1/
```

Après avoir modifié une commande ou son texte d’aide, régénérez-les ; `cargo test` échoue tant que vous ne le faites pas :

```bash
cargo run -- manpages man
```

## Configuration

`~/.config/voltkit/config.toml`. Chaque clé est facultative ; voir
[`data/config.example.toml`](data/config.example.toml).

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
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/about-dark.png?v=0.7.0">
  <img src="docs/screenshots/about-light.png?v=0.7.0" width="640" alt="La fenêtre À propos de Voltkit montrant l’icône de l’application, la version et l’auteur Alexis Rios-Negron">
</picture>

## Soutenir Voltkit

Si Voltkit vous a été utile, pensez à [faire un don du montant de votre choix](https://paypal.me/AlexisRios) pour aider à faire vivre le projet et à poursuivre son développement. Chaque contribution est sincèrement appréciée.

L’objectif est que Voltkit reste gratuit et accessible à tout le monde. Merci de l’utiliser et pour tout le soutien que vous pouvez apporter !

## Licence

MIT © Alexis Rios-Negron
