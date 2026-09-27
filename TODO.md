# TODO

Les choses auxquelles je pense au fur et à mesure qu'il faudrait faire :

- [ ] Déployer l'app (updater tauri) et la rendre open source
- [ ] Ajouter des 'presets' qui permettent de choisir une combinaison d'absolument tout dans l'app (items, stickers, palettes, activation ou non de certaines choses), ça permettrait d'avoir une sauvegarde de presets facile et de faire des swaps rapides.
- [ ] Essayer de désactiver le warning qui apparait pour le swap de la palette
- [ ] Reprendre plein de features de RL-Designer (Previews (mais ici il faudrait des preview bien plus complètes si possible), etc...)
- [ ] Essayer de voir si on peut pas faire à la place d'utiliser l'api rlpeak directement utiliser les items du jeu pour faire les swaps (et ça serait bien plus rapide et plus facile que de devoir tapper dans l'api, vu qu'on a déjà tous les fichiers du jeu, surtout que apparament les .upk des items sont unitaires)
- [ ] Voir si c'est possible aussi du coup de pouvoir choisir sur quel item on fait le swap (plutot que le defaut vortex et standard (et pour le sticker je sais pas.))
- [ ] Option pour enlever le bruit des crossbar
- [ ] Plugin pour la vitesse de la balle (mais ça je pense impossible à faire sans injection)
- [ ] Travailler l'overlay et le display des stats (aussi display mmr? -> déjà implémenté normalement mais il faut juste verrifier et améliorer)
- [x] decal balle custom (page Balle, à tester en jeu)
- [x] Decals custom « full color » — fait le 2026-09-27 : stickers hybrides (zones peintes + vraies couleurs), validés en jeu sur l'Octane. Les anciennes notes de `docs_rl/custom_decals*.md` sont dépassées.
- [ ] Tests en jeu à faire (2026-09-27), un par un, jeu fermé pour appliquer :
  1. Import depuis BakkesMod (carte BakkesMod ou bandeau de la page Stickers), conversion cochée : packs et balles arrivent dans la bibliothèque de l'app.
  2. Fennec : Born To Be Itzy (Fennec) → Flammes, équiper « Flammes » sur la Fennec.
  3. Dominus : pas de pack Dominus ; pour tester le mécanisme, dupliquer un pack avec `"BodyID": 403`.
  4. Universel : HeatFire → un sticker universel esports 2025 possédé (Team BDS…), l'équiper.
  5. Balle : page Balle → Itzy (Default) → Appliquer, puis un match (entraînement libre).
  6. Régression Octane : réappliquer un pack Octane.
  - En cas de crash : Réglages › Logs › Jeu.
- [ ] Venom / Road Hog / Esper : pas de donneur hybride ; piste = donneur Octane (ClassyLady) avec les textures de carrosserie re-pointées vers celles de la voiture.
