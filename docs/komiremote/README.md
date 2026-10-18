# Komi Remote — première tranche locale

Fork indépendant de [RustDesk](https://github.com/rustdesk/rustdesk), base
`fada664df7a294d1d1a9ca3e7cd3637069122f17` (1.5.0).
Cette tranche est du **code source expérimental**, pas une distribution Windows validée.
Aucun push, release ou dispatch ; aucun serveur de production ou service persistant modifié.

## Ce qui change

- Ressources originales du pack KomiRemote : huit ressources partagées `res/`,
  ICO du runner Windows, icône PNG/ICO/SVG Flutter et trois logos PNG.
  Correspondance exacte et SHA-256 : [assets-manifest.json](assets-manifest.json).
  Les copies sont binaires identiques aux originaux, sans recoloration ni étirement.
  Le PNG runtime tray/Flutter de 32 px et le SVG utilisent la variante small.
  Les ICO contiennent 16/24/32/48/64/128/256 px.
- Métadonnées visibles Windows : **Komi Remote** ; description explicite de fork.
  `CompanyName=Komi Remote contributors` est une description des contributeurs,
  pas une personne morale vérifiée ni une signature de code.
  Copyright Purslane conservé. À propos identifie le fork et lie ses sources.
- Identité technique Windows : **KomiRemote**, sans espace, via `APP_NAME`.
  Les profils/configurations, IPC et noms d'installation sont ainsi distincts de
  RustDesk. Il n'y a pas de migration automatique du profil RustDesk.
- Les EXE de build restent **rustdesk.exe** ; le workflow manuel renomme seulement
  le runner assemblé **KomiRemote.exe** dans un ZIP `KomiRemote/` non autonome.
  DLL, symboles FFI, crates, drivers et CMake gardent leurs noms upstream.
  Aucun auto-extractible tant que le runtime/CRT n'est pas validé sur Windows.
  Aucun renommage global : nom runtime et nom de fichier ne sont pas la même notion.
  Titres/menus utilisant l'identité technique affichent encore `KomiRemote`.
- Défauts publics Windows, modifiables dans les paramètres existants :

  | Option | Défaut |
  |---|---|
  | `custom-rendezvous-server` | `217.154.20.200` |
  | `relay-server` | `217.154.20.200:21117` |
  | `key` (clé publique, pas un secret) | `52TLfnxb60d5V0VSRwZpUDcDlphr7+qFQqgFumhKpr4=` |
  | `lang` | `fr` |
  | `theme` | `dark` |

  Adresse et clé fournies historiquement par le propriétaire ; **aucun test réseau**
  ou contrôle distant, aucune compatibilité opérationnelle démontrée.

## Initialisation et priorités

`src/komi_config.rs` ne connaît que `std`, les verrous et des maps de défauts.
Un `Once` dans `common::initialize_komi_defaults` l'exécute au plus une fois par
instance du cœur. Appels avant `global_init()` dans `core_main()`, avant le runner
asynchrone dans l'initialisation FFI, et avant la lecture existante de `custom.txt`.
L'adaptateur est **Windows uniquement** ; pas de changement runtime imposé aux
autres plateformes. Les ressources partagées/Flutter restent toutefois partagées.

Sur le runner Windows actuel, `wWinMain` charge `librustdesk.dll`, appelle
`rustdesk_core_main_args` avant Flutter et conserve cette bibliothèque chargée.
Les fenêtres secondaires partagent le processus ou suivent la même entrée.
L'initialisation FFI est une défense supplémentaire, pas la preuve qu'un autre
embedder pourrait appeler n'importe quelle API avant `core_main` en sécurité.
L'ordre complet doit encore être exercé sur Windows, notamment GUI/tray/service.

Les defaults ne sont ni une écriture de préférences, ni un verrouillage : aucune
map `OVERWRITE_*`, aucun `PROD_RENDEZVOUS_SERVER`, aucun mot de passe, aucun appel
réseau ou installation de service. Les valeurs déjà présentes dans les maps sont
préservées. Les options persistées restent prioritaires via le résolveur upstream.
Un second appel ne réinitialise pas une valeur modifiée, même après chargement
d'une personnalisation signée. Le choix de langue système (`default`, stocké vide)
et le thème système restent gérés par les chemins upstream, non remplacés par
une valeur écrite à chaque démarrage. Effacer une option réseau revient à son
défaut ; pour changer de serveur, renseigner explicitement hôte/relay/clé cohérents.
Les configurations encodées dans le nom EXE et les personnalisations signées
conservent leurs priorités upstream. `custom.txt` reste soumis à la même signature
Ed25519 ; ce fichier n'est pas notre mécanisme de configuration.

## Hors tranche / avant distribution

- Pas d'assistant première utilisation, de nouvelle barrière de consentement ou
  de modification des droits, mots de passe et services upstream. L'absence de
  mot de passe permanent embarqué **ne garantit pas** l'absence d'accès permanent.
- Thème sombre existant par défaut ; la palette complète Komi n'est pas appliquée.
  Pas de recoloration globale : blanc sur vert de marque serait insuffisant.
- MSI, raccourcis, titres uniformisés avec espace, éditeur légal, UpgradeCode,
  EULA, signature, SmartScreen et canal de mise à jour : à revoir et tester.
  Le changement de nom active `is_custom_client()` : les branches de mise à jour,
  API et options Pro ne sont pas validées fonctionnellement. Aucun canal Komi
  opérationnel n'est annoncé et aucun binaire ne doit être distribué en l'état.
- Tous les tests Windows d'installation, isolation face à RustDesk existant,
  démarrage/reboot, réseau, consentement, sécurité, fichiers/presse-papiers,
  multi-écrans et pilotes restent à faire.
- Les pages Web upstream (confidentialité/site), notices et copyrights sont
  conservés, sans prétendre qu'ils décrivent déjà le traitement de données Komi.

## Licence et origine des ressources

`LICENCE` reste le texte AGPL-3.0 upstream inchangé ; copyrights et notices tiers
conservés. Le fork n'est ni édité ni approuvé par RustDesk/Purslane.
Le lien vers les sources ne remplace pas l'obligation de publier le **code exact
correspondant aux binaires** et ses instructions lors d'une future distribution.
Ce commit local n'est pas encore publié. Pas d'avis juridique de conformité totale.

Ressources : pack fourni `KomiRemote-logo-noeuds`, concept terminal + deux nœuds,
police Manrope annoncée SIL OFL avec texte converti en contours ; aucune police
ajoutée au binaire. La provenance ne constitue pas un audit des droits du pack.

Voir [BUILD.md](BUILD.md) et [VALIDATION.md](VALIDATION.md).
