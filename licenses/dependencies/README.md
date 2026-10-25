# Licences des dépendances / Dependency licenses

Le mécanisme est adapté des scripts d'Oxyde-GPUI, Copyright (C) 2026
Carlton2001, sous GPL-3.0 (voir le fichier LICENSE à la racine).

```powershell
./scripts/install-cargo-about.ps1
./scripts/update-third-party-licenses.ps1
./scripts/check-third-party-licenses.ps1
```

Le générateur Windows x64 officiel cargo-about 0.9.2 est épinglé et vérifié par
SHA-256. S'il est déjà installé, seule la commande de mise à jour est nécessaire.
La génération utilise Cargo.lock sans le modifier et échoue si une licence ne
peut pas être résolue. `about.toml` définit les licences admises pour cet inventaire.

`THIRD-PARTY-NOTICES.md` regroupe les textes et copyrights avec les noms et
versions des crates, pour tout le workspace, toutes les cibles, les dépendances
de construction et de développement. L'inventaire dépasse donc le contenu du
binaire Windows. Les copies locales de `vendor/` sont incluses. Une section
complémentaire conserve les fichiers NOTICE, COPYRIGHT et AUTHORS amont.

`assets/` conserve les licences Lucide, Inter, JetBrains Mono et GPUI ainsi que
les attributions de RAP. Les chemins du NOTICE copié sont relatifs au dossier
`assets/` du projet source ; la licence Lucide est ici `LUCIDE-LICENSE.txt`.
JetBrains Mono est incluse par prudence même si elle n'est pas embarquée.

`inputs.json` enregistre les empreintes du verrou, des manifestes du workspace
et des crates vendoriées, de la configuration, du générateur, du template, des
attributions et des résultats. La vérification refuse les entrées modifiées,
supprimées et les nouveaux manifestes. Régénérer après une mise à jour des
dépendances ou des attributions. Cette vérification ne refait pas l'analyse des
sources locales : régénérer aussi après toute modification de leurs notices.
Les empreintes portent sur le texte UTF-8 avec fins de ligne LF, afin que la
conversion CRLF de Git sous Windows ne rende pas les notices périmées.

`-Offline` permet une génération avec les sources déjà en cache, mais certaines
crates mal empaquetées nécessitent des licences récupérées sur leurs dépôts
amont ; ce mode peut perdre leurs copyrights spécifiques. Préférer le mode en
ligne pour une livraison. La future distribution devra exécuter la génération
et la vérification, puis inclure ce dossier et le LICENSE de RAP.

The commands above generate and verify dependency notices for the entire RAP
workspace. The report preserves upstream license texts and supplemental notices;
`assets/` contains resource licenses and attributions. Regenerate after dependency
or attribution changes. Prefer online generation for releases. Future packages
must include this directory and RAP's root LICENSE.
