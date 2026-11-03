<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="flutter/assets/logo_dark.png">
    <img src="flutter/assets/logo_light.png" alt="KomiRemote" width="420">
  </picture>
</p>

<p align="center">
  <b>KomiRemote</b> — bureau à distance pour Windows, version modifiée de RustDesk.<br>
  <a href="https://komjordan.fr/komi.html">Télécharger</a> ·
  <a href="docs/komiremote/LEGAL-NOTICES-KOMIREMOTE.md">Mentions légales</a> ·
  <a href="LICENCE">AGPL-3.0</a>
</p>

<p align="center">
  <strong>Version actuelle&nbsp;: <a href="https://github.com/komjordan/KomiRemote/tree/v1.5.1-komi.71">v1.5.1-komi.71</a></strong>
  <br>
  <sub>Windows portable · première build publiée</sub>
</p>

> [!IMPORTANT]
> **KomiRemote est une version modifiée de [RustDesk](https://github.com/rustdesk/rustdesk), distribuée sous licence GNU AGPL-3.0.**
> Ce projet n’est **ni affilié à, ni approuvé, ni soutenu** par RustDesk ou Purslane Tech Pte. Ltd.
> « RustDesk » est cité uniquement pour indiquer l’origine du code.

> [!CAUTION]
> **Usage responsable :** n’utilisez KomiRemote que sur des appareils pour lesquels vous avez une autorisation explicite.
> Tout accès non autorisé, prise de contrôle ou atteinte à la vie privée est interdit. Les auteurs ne sont pas responsables d’une utilisation abusive.

## Présentation

KomiRemote reprend le moteur de bureau à distance de RustDesk OSS (base 1.5.0) et y ajoute une interface simplifiée en français pour un usage personnel sous Windows :

- identité visuelle KomiRemote (icônes, logos, nom d’application `KomiRemote` séparé de RustDesk) ;
- barre de session repliable, en mode minimal ou complet ;
- appel vidéo depuis une session déjà connectée ;
- partage de la caméra du contrôleur ;
- approbation manuelle des connexions entrantes par défaut ;
- enregistrement de session avec dialogue « enregistrement sauvegardé » ;
- nom de connexion personnalisé : un appareil peut être joint par son nom en complément de son ID numérique ;
- page À propos avec licence, code source et mentions légales ;
- notification de mise à jour basée sur `https://komjordan.fr/komi-version.json` (aucun téléchargement automatique) ;
- les fonctions présentées s’utilisent sans compte.

## Téléchargement

La version portable Windows (`KomiRemote.exe`) est publiée sur **<https://komjordan.fr/komi.html>**, avec sa taille, sa date et son empreinte SHA-256.

Vérifiez l’empreinte avant de lancer l’exécutable :

```powershell
Get-FileHash .\KomiRemote.exe -Algorithm SHA256
```

L’exécutable n’est pas signé : Windows SmartScreen peut afficher un avertissement (« Informations complémentaires » → « Exécuter quand même »).

## Code source correspondant

La version portable publiée correspond au tag [`v1.5.1-komi.71`](https://github.com/komjordan/KomiRemote/tree/v1.5.1-komi.71). `main` contient les évolutions suivantes, dont les appels vidéo et les pastilles de session.

## État des plateformes

- **Windows :** le portable publié reste le tag ci-dessus. Les captures récentes montrent la barre de session et l’appel vidéo.
- **macOS :** build de test 1.5.1+85, [Apple Silicon et Intel](https://github.com/komjordan/KomiRemote/actions/runs/41680507345). Signature ad hoc, sans Developer ID ni notarisation.
- **Linux, Android et iOS :** développements en cours.

## Captures

### Accueil

![Accueil KomiRemote](docs/komiremote/screenshots/v1.5.1-komi.71/komiremote-v16-accueil.png)

### À propos

![À propos de KomiRemote](docs/komiremote/screenshots/v1.5.1-komi.71/komiremote-v16-a-propos.png)

### Demande de connexion et appel vidéo entrant

<table>
  <tr>
    <th>Demande de connexion</th>
    <th>Appel vidéo entrant — hôte</th>
  </tr>
  <tr>
    <td valign="top"><img src="docs/komiremote/screenshots/v1.5.1-komi.71/komiremote-v16-demande-connexion.png" alt="Demande de connexion au bureau distant"></td>
    <td valign="top"><img src="docs/komiremote/screenshots/2026-11-03/appel-video-entrant.png" alt="Hôte : appel vidéo entrant de Kom Jordan, boutons Accepter et Rejeter"></td>
  </tr>
</table>

Sur petit écran, ouvrir la [demande de connexion](docs/komiremote/screenshots/v1.5.1-komi.71/komiremote-v16-demande-connexion.png) ou l’[appel vidéo entrant](docs/komiremote/screenshots/2026-11-03/appel-video-entrant.png) pour lire la capture à sa taille d’origine.

### Nom de l’appareil

Le nom de l’appareil permet de s’y connecter sans ressaisir son ID. Il se règle dans **Paramètres → Sécurité → Nom de connexion personnalisé**.

![Information sur le nom de l’appareil](docs/komiremote/screenshots/v1.5.1-komi.71/komiremote-v16-nom-appareil.png)

### Nom de connexion personnalisé

Le nom choisi complète l’ID numérique et peut être vérifié, appliqué ou libéré depuis les paramètres de sécurité.

![Nom de connexion personnalisé](docs/komiremote/screenshots/v1.5.1-komi.71/komiremote-v16-nom-connexion-personnalise.png)

### Barres de session — mode minimal

**Hôte — pastilles compactes**

<p align="center">
  <img src="docs/komiremote/screenshots/2026-11-03/pastilles-compactes.png" alt="Hôte : pastilles compactes de contrôle, appel vocal et caméra">
</p>

**Contrôleur — barre minimale**

<p align="center">
  <img src="docs/komiremote/screenshots/v1.5.1-komi.71/komiremote-v16-barre-outils-minimale.png" alt="Barre de session minimale">
</p>

### Barres de session — mode complet

**Hôte — pastilles dépliées**

<p align="center">
  <img src="docs/komiremote/screenshots/2026-11-03/pastilles-depliees.png" alt="Hôte : pastilles dépliées Contrôle, Appel vocal avec durée et Caméra">
</p>

**Contrôleur — barre complète**

<p align="center">
  <img src="docs/komiremote/screenshots/v1.5.1-komi.71/komiremote-v16-barre-outils-complete.png" alt="Barre de session complète">
</p>

## Compilation (Windows)

Prérequis : Visual Studio 2022 (outils C++), Rust, Flutter, vcpkg et Python 3, comme pour RustDesk.
Voir la [documentation de compilation RustDesk](https://rustdesk.com/docs/en/dev/build/) et [docs/komiremote/BUILD.md](docs/komiremote/BUILD.md).

```powershell
python build.py --portable --flutter --hwcodec --vram
```

## Compilation (macOS — branche de test)

Les sources de la build Mac 1.5.1-85 se trouvent sur [`work/macos-video-v2-20261103`](https://github.com/komjordan/KomiRemote/tree/work/macos-video-v2-20261103), au commit [`1e6ef555c89d41d11a4607edae7bc05752b869b9`](https://github.com/komjordan/KomiRemote/commit/1e6ef555c89d41d11a4607edae7bc05752b869b9), et non dans une release 85 ni nécessairement dans `main`.

```sh
git clone --branch work/macos-video-v2-20261103 --recurse-submodules https://github.com/komjordan/KomiRemote.git
cd KomiRemote
git checkout 1e6ef555c89d41d11a4607edae7bc05752b869b9
git submodule update --init --recursive
```

Suivre ensuite la recette du [workflow macOS](https://github.com/komjordan/KomiRemote/blob/1e6ef555c89d41d11a4607edae7bc05752b869b9/.github/workflows/komiremote-macos-test.yml) : bridge, dépendances natives, Flutter 3.24.5, Rust 1.81, puis compilation et signature ad hoc. Apple Silicon et Intel n’utilisent pas les mêmes options.

## Licence et copyright

KomiRemote est distribué selon la **GNU Affero General Public License v3.0** — texte intégral dans [`LICENCE`](LICENCE).

- Copyright © Purslane Tech Pte. Ltd. et contributeurs RustDesk (code d’origine : <https://github.com/rustdesk/rustdesk>).
- Modifications KomiRemote © 2026 Axel KAMGAING KOM.

Ce programme est fourni **sans aucune garantie**. Les licences des dépendances tierces restent applicables.
Détails : [mentions légales KomiRemote](docs/komiremote/LEGAL-NOTICES-KOMIREMOTE.md).
