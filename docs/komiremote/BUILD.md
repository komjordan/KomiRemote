# BUILD — Komi Remote Windows x64

## Statut

**Aucun build Windows/Flutter exécuté dans cette tranche. Aucun EXE/MSI produit.**
Environnement de travail Linux ; pas de MSVC/Windows SDK, Flutter ni Dart.
Rust 1.75.0 minimal a été installé uniquement dans un répertoire scratch isolé
pour compiler/exécuter le module de défauts sans dépendances natives.
Le sous-module officiel a été initialisé au gitlink
`229b904508364c8997aad0fb5af57effac859f60`, sans changement de source ou de gitlink.

Les références exactes sont les fichiers upstream conservés :
`.github/workflows/flutter-build.yml`, `.github/workflows/bridge.yml`, `build.py`,
`vcpkg.json`, `res/msi/README.md`. Aucun workflow upstream modifié ou lancé.
Un essai **manuel uniquement**, sans release ni secrets, est maintenant préparé dans
`.github/workflows/komiremote-windows-test.yml` : voir
[CI-WINDOWS-TEST.md](CI-WINDOWS-TEST.md) pour les quatre empreintes approuvées
obligatoires, le bundle ZIP **non autonome**, les tests locaux et les risques des CI upstream.
Il n'a pas été exécuté : aucun résultat Windows ne peut en être déduit.
Les commandes suivantes sont une recette de préparation **non exécutée ici**,
à utiliser dans une copie jetable sur un runner Windows explicitement autorisé.

## Dépendances du chemin upstream x64

| Outil | Version/référence |
|---|---|
| Windows | VS 2022 Desktop C++, Windows SDK, MSBuild |
| Rust | 1.75.0, cible `x86_64-pc-windows-msvc` |
| Flutter build | 3.24.5 |
| Flutter génération bridge | 3.22.3 (environnement séparé) |
| LLVM | 15.0.6 |
| vcpkg | `9e593bb18ea69cc5095e012465dcd675a822ed0d` |
| Triplet | `x64-windows-static` |
| NASM CI | 3.01 win64, SHA512 de la recette vcpkg épinglée (voir CI-WINDOWS-TEST.md) |
| flutter_rust_bridge_codegen | 1.80.1, feature `uuid` |
| cargo-expand | 1.0.95 |
| MSI (non adapté Komi) | WiX SDK/extensions/WcaUtil/DUtil 4.0.5, NuGet |

Également Git, Python/pip, NASM, CMake dans PATH. Conserver `Cargo.lock` et
`flutter/pubspec.lock`, ne pas mettre à jour les dépendances arbitrairement.

```powershell
# Dans la copie de build Windows, pas le checkout de revue
# Après installation des outils ci-dessus et clonage de vcpkg dans C:\vcpkg
git submodule update --init --recursive
rustup toolchain install 1.75.0 --component rustfmt --target x86_64-pc-windows-msvc
rustup override set 1.75.0
$env:VCPKG_ROOT = 'C:\vcpkg'
$env:VCPKG_DEFAULT_HOST_TRIPLET = 'x64-windows-static'
git -C C:\vcpkg checkout 9e593bb18ea69cc5095e012465dcd675a822ed0d
& C:\vcpkg\bootstrap-vcpkg.bat
& C:\vcpkg\vcpkg.exe install --triplet x64-windows-static --x-install-root="C:\vcpkg\installed"
```

### Bridge : étape isolée obligatoire

Dans une copie dédiée avec Flutter **3.22.3**, suivre `bridge.yml`, notamment la
substitution temporaire d'extended_text 14.0.0 vers 13.0.0. Ne pas reporter cette
modification dans les sources du build final 3.24.5.

```bash
cargo install cargo-expand --version 1.0.95 --locked
cargo install flutter_rust_bridge_codegen --version 1.80.1 --features uuid --locked
# Après adaptation temporaire du manifeste comme bridge.yml :
(cd flutter && flutter pub get)
flutter_rust_bridge_codegen --rust-input ./src/flutter_ffi.rs --dart-output ./flutter/lib/generated_bridge.dart --c-output ./flutter/macos/Runner/bridge_generated.h
```

Restaurer au build Windows les artefacts exacts énumérés dans `bridge.yml`
(y compris code Rust et headers). Ne pas utiliser un générateur plus récent.

### Build runner + DLL

Le chemin upstream remplace le moteur release Flutter x64 avec
`https://github.com/rustdesk/engine/releases/download/main/windows-x64-release.zip`
et applique au SDK `.github/patches/flutter_3.24.4_dropdown_menu_enableFilter.diff`.
Ces étapes doivent être reproduites ou leur omission explicitement évaluée.
Le tag moteur `main` est mutable : capturer une empreinte approuvée avant de
parler de reproductibilité. Le workflow manuel dédié exige désormais une empreinte
approuvée avant téléchargement/extraction ; aucun lancement n'est autorisé ici.

```powershell
python3 .\build.py --portable --flutter --skip-portable-pack --hwcodec --vram
```

Appels upstream correspondants :

```text
cargo build --locked --features hwcodec,vram,flutter --lib --release
cd flutter
flutter build windows --release
```

Bundle attendu : `flutter/build/windows/x64/runner/Release/`, avec `rustdesk.exe`,
`librustdesk.dll`, les autres DLL et les assets. **L'EXE seul n'est pas un portable.**
Ne pas renommer les DLL ou le binaire CMake pour forcer un nom de distribution.
Capturer les logs, outils réellement utilisés et empreintes du bundle.

### Packaging : restant à adapter, pas un livrable

Le workflow ajoute pilotes/usbmmidd/WindowInjection.dll, contrôle les fichiers
consommés et le manifeste CMake, renomme le runner assemblé `KomiRemote.exe`
sans changer `librustdesk.dll`/FFI, puis ne produit qu'un ZIP **NOT-STANDALONE**.
Il n'appelle plus `libs/portable/generate.py` : CRT/fermeture DLL de WindowInjection
et validation Windows restent bloquants. Voir CI-WINDOWS-TEST.md pour NASM 3.01
vérifié par SHA512 avant vcpkg, rapports PE préparés et critères VM propre.
Un renommage testé en fixture n'est pas une preuve de fonctionnement Windows.

**Ne pas générer/distribuer un MSI Komi avec la recette stock.**
`res/msi/preprocess.py --app-name KomiRemote` attendrait `KomiRemote.exe` et
l'exécuterait ; l'EXE renommé du ZIP ne valide pas ce chemin MSI. Appeler la recette
stock avec RustDesk mélangerait les identités. Le préprocesseur substitue aussi automatiquement des termes de l'EULA :
revue juridique et technique nécessaire. Tester UpgradeCode, isolation, installation,
désinstallation et reboot avant tout installateur.

### Échecs attendus et diagnostic

- `hbb_common` absent : initialiser le sous-module au SHA enregistré, ne pas le patcher.
- Bindings/headers manquants : générer avec le job bridge et versions exactes.
- Bibliothèques natives manquantes : vérifier `VCPKG_ROOT`, triplet statique et MSVC.
- Erreur d'installation liée au nom : garder `KomiRemote`, pas `Komi Remote`, dans
  les chemins techniques. Ne pas assouplir `validate_install_app_name`.
- Aucun certificat Komi : ne jamais utiliser de secrets/signatures upstream ;
  SmartScreen et signatures nécessitent un chantier séparé.

Aucune garantie Windows 7/ARM64 ni reproductibilité binaire bit-à-bit à ce stade.
