# ESO Addons Manager

[English](README.md) · **Français** · [Deutsch](README.de.md) · [Español](README.es.md) · [Русский](README.ru.md) · [Українська](README.uk.md)

Un gestionnaire de bureau pour installer, mettre à jour et supprimer des addons
de The Elder Scrolls Online — un remplacement de Minion plus pratique et autonome.
Les dépendances des addons sont installées automatiquement (de temps à autre, le
gestionnaire vous demandera d'en choisir une parmi plusieurs), et celles qui ne
servent plus sont supprimées, de sorte que votre dossier AddOns reste propre :
exactement ce qu'il faut pour faire fonctionner vos addons et rien de superflu.

Fonctionne sous Windows, macOS et Linux. L'interface est disponible en six langues —
anglais, français, allemand, espagnol, russe, ukrainien. La langue est choisie
automatiquement selon la langue de votre système et peut être changée dans
l'application.

> Vous utilisez déjà [eso-addons-updater](https://github.com/powerbq/eso-addons-updater) ?
> Rien à changer — il met toujours à jour et lance le gestionnaire, comme avant.
>
> Les nouveaux utilisateurs ont toutefois intérêt à télécharger le gestionnaire
> depuis les fichiers ci-dessous. Au démarrage,
> [eso-addons-updater](https://github.com/powerbq/eso-addons-updater) se met à jour
> lui-même ainsi que le gestionnaire vers la dernière version, puis le lance — et
> pour se mettre à jour, il réécrit son propre exécutable. Pour une application non
> signée, les antivirus peuvent juger ce comportement suspect et bloquer le
> lancement. Le gestionnaire installé depuis l'un des fichiers ci-dessous ne fait
> rien de tel : il se contente de signaler une nouvelle version et de proposer
> d'ouvrir la page de téléchargement, tandis que vous la téléchargez et l'installez
> vous-même. C'est donc le moyen le plus simple d'éviter les ennuis avec l'antivirus
> — il ne restera que l'avertissement indiquant que l'application a un
> [éditeur inconnu](#windows-smartscreen).

## Téléchargement

Dernière version : <https://github.com/powerbq/eso-addons-manager/releases/latest>

Choisissez un **installateur** pour votre système (recommandé), ou une archive
zip **autonome** :

| Système | Installateur | Autonome |
|--------|-----------|----------|
| Windows | `…-windows-amd64-setup.exe` | `eso-addons-manager-windows-amd64.zip` |
| macOS (Apple Silicon) | `…-macos-arm64.dmg` | `eso-addons-manager-macos-arm64.zip` |
| Linux | `.deb` / `.rpm` / `.pkg.tar.zst` / `.AppImage` | `eso-addons-manager-linux-amd64.zip` |

> L'architecture Intel sur macOS n'est pas prise en charge pour le moment.

> Vous pouvez vérifier n'importe quel téléchargement à l'aide du fichier
> `SHA256SUMS` présent dans la même version.

## Installation

- **Windows :** lancez l'installateur `-setup.exe` (il propose une installation
  par utilisateur ou pour tous les utilisateurs). Ou décompressez l'archive
  autonome n'importe où et lancez `app.exe`.
- **macOS :** ouvrez le `.dmg` et faites glisser l'application vers Applications.
  Ou décompressez l'archive autonome.
- **Linux :** installez le paquet —
  `sudo dpkg -i eso-addons-manager_*.deb` / `sudo rpm -U eso-addons-manager-*.rpm` /
  `sudo pacman -U eso-addons-manager-*.pkg.tar.zst` — ou rendez le `.AppImage`
  exécutable et lancez-le, ou décompressez l'archive autonome (accordez les droits
  d'exécution au binaire `app`).

### Windows SmartScreen

L'application n'est pas signée par un certificat et est distribuée telle quelle ;
Windows SmartScreen signalera donc très probablement un éditeur inconnu. C'est
normal. Pour l'exécuter malgré tout, cliquez sur **« Informations complémentaires »**
→ **« Exécuter quand même »**.

## Mise à jour

Le gestionnaire vérifie la présence d'une version plus récente au démarrage et
propose d'ouvrir la page de téléchargement. Les mises à jour ne sont jamais
appliquées en silence — vous téléchargez et installez vous-même la nouvelle
version (relancez l'installateur, mettez à jour le paquet avec votre gestionnaire
de paquets, ou remplacez les fichiers autonomes). La vérification peut être
désactivée, et lancée à la demande, depuis le groupe **☰ menu → Mises à jour**.

## Avant de commencer

> **Important.** Sauvegardez votre dossier `AddOns` avant la première
> utilisation. Le gestionnaire supprime du dossier sélectionné tout ce qu'il ne
> gère pas, choisissez donc le dossier avec soin. Pour prendre en
> charge les addons que vous possédez déjà, utilisez **Analyser les addons
> existants** (voir ci-dessous).

## Utilisation

1. **Choisissez le dossier AddOns du jeu** — **☰ menu → Dossier AddOns**. Une boîte
   de dialogue explique où le jeu range habituellement ses addons et affiche un
   exemple de chemin pour votre système.

2. **Analyser les addons existants** — prend en charge les addons déjà présents dans
   le dossier choisi, pour ne pas avoir à les rajouter. Vous pouvez analyser avant la
   première synchronisation ou plus tard. Attention : si **Synchroniser au
   démarrage** est activé, la synchronisation automatique supprimera au prochain
   démarrage les addons ajoutés par des moyens externes avant que vous ne les
   analysiez — gardez-le à l'esprit.

3. **Catalogue** — l'écran principal de l'application, avec la liste de tous les
   addons disponibles. En haut : recherche par nom, auteur ou dossier, un **Filtre**
   (**Installés**, **Dépendances**, **Favoris**) et **Trier par** (nom, auteur, date
   de mise à jour, nombre de téléchargements, etc.). Sélectionnez un addon pour ouvrir
   sa fiche avec sa **Description**, son **Journal des modifications** et ses captures
   d'écran. De là, vous pouvez installer ou supprimer un addon, l'ajouter aux
   **Favoris** et ouvrir sa page sur esoui.com.

4. **Synchroniser** — met à jour tous les addons installés et leurs bibliothèques,
   et supprime le superflu. Activez **Synchroniser au démarrage** (en bas à droite)
   pour le faire automatiquement au lancement.

5. **Dépendances** — gérées automatiquement : les bibliothèques sont installées,
   mises à jour et supprimées lorsqu'aucun addon n'en a plus besoin. Vous ne gérez
   que la liste des addons que vous voulez. Lorsque plusieurs addons fournissent la
   même bibliothèque, le gestionnaire la résout généralement tout seul (il préfère
   celle vers laquelle pointent les auteurs de l'addon). Ce n'est que lorsqu'il ne
   peut vraiment pas trancher qu'il **vous demande de choisir**. Votre choix est
   mémorisé, et chacune de ces bibliothèques (choisie par vous ou automatiquement) est
   affichée dans l'onglet **Bibliothèques**, où vous pouvez la modifier ou la
   réinitialiser en mode automatique à tout moment.

6. **Tamriel Trade Centre** — si TTC est installé, le gestionnaire met à jour ses
   tables de prix automatiquement pendant une synchronisation, sans le client TTC.
   L'option et la région (**Europe / Amérique**) se trouvent dans le groupe
   **☰ menu → Prix TTC**.

7. **Addons obsolètes** — par défaut, le catalogue n'affiche que les addons
   actuels. Si vous avez besoin d'un ancien addon abandonné et donc masqué, activez
   **☰ menu → Expérimental → Addons obsolètes** et il apparaîtra dans le catalogue.
   C'est expérimental, donc désactivé par défaut.

## Avancé

- **Onglet Exclusions.** Protège des fichiers contre la suppression — par exemple
  les tables de prix mises à jour de Tamriel Trade Centre ou la base téléchargée des
  coordonnées de tous les points de récolte de HarvestMap-Data. Des exclusions
  automatiques pour **Tamriel Trade Centre** et **HarvestMap-Data** sont intégrées et
  s'activent lorsque ces addons sont installés ; vous pouvez ajouter vos propres
  règles à gauche.

- **Onglet Correctifs.** Modifie le *contenu* des fichiers d'un addon pendant une
  synchronisation — un petit rechercher-remplacer à l'intérieur d'un fichier —
  afin qu'un correctif dont vous avez besoin ne soit pas écrasé par les données
  d'origine de l'addon. Les correctifs intégrés s'appliquent automatiquement lorsque
  l'addon concerné est installé et sont affichés en lecture seule, étiquetés par
  addon (par exemple, pour préserver l'intégration Tamriel Trade Centre de
  **DovahMova**). Vous pouvez ajouter vos propres règles à gauche, modifier l'ordre
  de leur application et les activer ou désactiver individuellement. C'est la solution
  en libre-service lorsqu'un correctif intégré
  ne correspond plus après la mise à jour d'un addon, ou lorsqu'un addon dont vous
  avez besoin n'a pas encore de correctif intégré.

> Les règles d'exclusions et de correctifs se définissent avec des expressions
> régulières ; écrivez-les et testez-les sur <https://regex101.com/>.

> Si un addon a besoin d'exclusions ou de correctifs pour fonctionner correctement,
> laissez un retour et leur prise en charge pourra être ajoutée aux listes intégrées.

- **Vos propres addons (personnalisés).** Pour installer des addons qui ne
  figurent pas dans le catalogue ESO UI — les vôtres, ou ceux que vous n'avez que
  sous forme de `.zip` — placez l'archive dans le dossier **custom**
  (**☰ menu → Dossiers de l'application → Addons personnalisés**). À chaque
  synchronisation, le gestionnaire extrait chaque `custom/*.zip` dans votre dossier
  AddOns et l'y conserve aux côtés des addons du catalogue, afin qu'il ne soit pas
  considéré comme un fichier parasite et supprimé. Leurs dépendances sont résolues
  comme celles de n'importe quel autre addon.

- **Où se trouvent vos données.** Le gestionnaire stocke ses données dans les
  emplacements standard du système d'exploitation, propres à chaque utilisateur, et
  non à côté de l'exécutable ; chaque dossier peut s'ouvrir directement depuis le
  menu :
  - **☰ menu → Dossiers de l'application → Configuration** — les paramètres `app.ini`
    (addons installés, favoris, choix de bibliothèques, exclusions, correctifs,
    options) et leurs sauvegardes ;
  - **☰ menu → Dossiers de l'application → Cache de l'application** — téléchargements
    mis en cache : archives d'addons, catalogue et données de prix TTC ;
  - **☰ menu → Dossiers de l'application → Addons personnalisés** — vos propres
    archives `.zip`, extraites dans le dossier AddOns à chaque synchronisation ;
  - **☰ menu → Journal → Ouvrir le dossier** — journaux enregistrés.

  Les dossiers suivent les conventions de chaque plateforme
  (`%LOCALAPPDATA%\EsoAddonsManager` sous Windows,
  `~/Library/…/io.github.powerbq.eso-addons-manager` sous macOS, les répertoires de
  base XDG sous `eso-addons-manager` sous Linux). Au premier lancement, les données
  de l'ancienne disposition (dans le répertoire de travail) sont migrées
  automatiquement vers ces emplacements.

## En cas de problème

Si l'application ne se comporte pas comme prévu, utilisez **☰ menu → Assistance →
Signaler un problème**. Dans cette boîte de dialogue, vous pouvez choisir ce qu'il
faut joindre au rapport, décrire ce qui s'est passé et indiquer comment vous joindre
— puis tout envoyer au développeur.

## Compilation

La compilation depuis les sources, l'empaquetage et l'organisation de la CI
sont décrits dans [BUILD.md](BUILD.md) (en anglais).

## Licence

Distribué sous la [licence MIT](LICENSE).

Copyright (c) 2026 powerbq.

Vous êtes libre de forker, modifier et redistribuer ce logiciel, mais la notice de
copyright originale et le texte de la licence MIT doivent être conservés dans
toutes les copies et œuvres dérivées.
