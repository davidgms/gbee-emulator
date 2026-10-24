# GBee - GBA market and language research (2026-10-24)

## Data quality warning (read first)
Hard per-country download data for GBA emulators (My Boy!, Pizza Boy, Lemuroid, RetroArch, John GBA) was not found publicly. Sensor Tower and AppMagic did not surface, and Similarweb hides downloads behind a paywall. Google Trends could not be queried from here. Most of the ranking below is therefore built from Google Play market size plus scattered community signals. Run Google Trends yourself for "gba emulator", "emulador gba", "emulator gba" and "gba エミュレータ" before committing translation money.

Hard data points:
- Google Play downloads by country, Nov 2024 to Nov 2025 (AppTweak, vendor estimate): India 18B (22.1%), Brazil 7.7B (9.4%), US 5.8B, Indonesia 5.7B, Mexico 3.8B, Philippines 2.6B, Turkey 2.4B, Pakistan 2.3B, Egypt 1.8B, Germany 1.7B. https://www.apptweak.com/en/reports/app-downloads-by-country
- My Boy! store rank in Games > Arcade (Similarweb, vendor): Japan #2; US, Germany, Spain, France #3. https://www.similarweb.com/app/google/com.fastemulator.gba/
- Google Play has been blocked in mainland China since 2012. Hong Kong, Macau and Taiwan use the normal Play Store and read Traditional Chinese (zh-HK, zh-TW). https://appinchina.co/blog/google-play-store-in-china-everything-you-need-to-know/ , https://www.apptweak.com/en/aso-blog/how-to-localize-your-app-in-chinese
- Retro handheld market: about US$3.8B in 2025, up about 21% (IconEra, reported by Olhar Digital; vendor estimate). https://olhardigital.com.br/2026/08/05/games-e-consoles/o-boom-dos-handhelds-retro-mercado-deve-movimentar-us-418-bilhoes-em-2026/ Anbernic launched the GBA-style RG SP at US$64.99 in Aug 2026 (blog source). https://tech-insider.org/anbernic-rg-sp-gba-25th-anniversary-launch-2026/
- Physical GBA prices look flat in 2026, with a 30-50% premium for IPS-modded units (sourcing-site estimate, weak). https://electronics.alibaba.com/product/gameboy-price I found no reliable data on the Japanese used market or Mercari.

## Ranked languages (store listing first, UI second)
1. **pt-BR** - Brazil is the #2 Play market by downloads (AppTweak). It is also your home audience and where your LinkedIn launch lands.
2. **Spanish (es-419 first, then es-ES)** - Mexico is #5, plus the rest of LatAm. Spain is a top-3 Arcade market for My Boy!. There is a large Spanish-only Pokemon ROM hack scene (https://discord.com/servers/whack-a-hack-292436944670162955 , https://www.pokecommunity.com/threads/pok%C3%A9mon-terra-firma-march-8.317453/post-8043860).
3. **Indonesian** - #4 Play market by downloads (AppTweak). No GBA-specific data was found, but the market is large for a cheap translation.
4. **Japanese** - Japan is where My Boy! ranks highest (#2 Arcade). It is the GBA's home market and has high revenue per user.
5. **French** - My Boy! is #3 Arcade in France (Similarweb). This also reaches Francophone Africa (inference).
6. **German** - My Boy! is #3 Arcade in Germany, and Germany is #10 on Play (AppTweak).
7. **Turkish** - #8 Play market (AppTweak). No GBA-specific data.
8. **Traditional Chinese (zh-TW, with zh-HK reusing it)** - the only Chinese that reaches buyers on Play. Do Simplified only if you plan to distribute through Chinese stores, which requires a local business licence or partner (appinchina link above).
9. **Korean** - wanted, but I found no data showing GBA or emulator demand in Korea. Low priority.
10. **Vietnamese, Russian, Arabic, Hindi** - no data either way. India, the Philippines and Pakistan are large Play markets, but English listings probably cover them (inference, untested). Arabic adds right-to-left layout cost.

## Top 10 places, events and people to reach
1. **Whack a Hack (WaH)** - calls itself the largest Spanish-language Pokemon fangame and ROM hacking community, active since 2005. https://discord.com/servers/whack-a-hack-292436944670162955
2. **PokeCommunity ROM hacking forums** - the main English hub. Hacks people name include Radical Red and Unbound. It bans ROM links, so pitch the emulator only. https://www.pokecommunity.com/threads/the-future-of-pok%C3%A9mon-romhacks.536951/
3. **gbadev and GBA Jam** - GBA Jam 2024 had 51 entries (https://itch.io/jam/gbajam24). GBA Jam 2026 ran June 13 to Sept 13 (https://itch.io/jam/gbajam26). Offer GBee as a way to play jam entries on phones, or sponsor a prize next year.
4. **Goodboy Galaxy** - a commercial homebrew GBA game. Co-developer Jeremy Clarke moderates the GBAdev Discord. It is a legal showcase ROM for screenshots, if you get permission. https://goodboygalaxy.com/press/pressrelease
5. **Retro Game Corps (Russ)** - a leading YouTube channel for handheld and Android emulation guides. https://rss-parrot.net/web/feeds/bsky.app.profile.retrogamecorps.com
6. **retrohandhelds.gg** - press for Android emulation and handhelds. https://retrohandhelds.gg/best-retro-handhelds-mid-2025/
7. **Games Done Quick / adef** - adef closed AGDQ 2026 with a Pokemon Emerald run (3h13m). https://bulbagarden.net/threads/three-pokemon-games-to-feature-in-this-weeks-awesome-games-done-quick-2026-benefiting-prevent-cancer-foundation.309459/
8. **speedrun.com Pokemon Gen 3 runners** - Emerald and FireRed have active console and Japanese leaderboards. Speedrunners care about accuracy, so pitch them once accuracy is proven. https://www.speedrun.com/users/pokeguy
9. **RetroAchievements** - many GBA sets and community events (https://retroachievements.org/event/126). Integrating rcheevos would be a feature that brings users. I could not get activity by region (site returned 403).
10. **Olhar Digital and other Brazilian tech press** - already covering the retro handheld boom in Portuguese. A natural fit for a Brazilian solo-developer story. https://olhardigital.com.br/2026/08/05/games-e-consoles/o-boom-dos-handhelds-retro-mercado-deve-movimentar-us-418-bilhoes-em-2026/

Not found: Mario Kart Super Circuit or GBA fighting game tournaments, speedrun.com popularity rankings by game, any public Pokemon hack audience split by language, and the Chinese hack scene (Ultimate Emerald has no reliable source). Note also that Showdown ADV OU is a simulator, not GBA hardware, so it is not a target audience.

## Risks
- **Nintendo enforces through civil lawsuits against emulators that bypass encryption.** Yuzu settled for US$2.4M. https://www.androidauthority.com/emulators-for-android-shut-down-pulled-play-store-3424602/ A report of 400+ Switch emulator repos taken down from GitHub in Aug 2026 is secondhand. https://en.kocpc.com.tw/?p=24678 No action against a GBA emulator was found.
- **Play removals have mostly been unexplained.** DraStic left Play in Feb 2025 with no stated cause, and Nintendo's role is speculation. https://www.timeextension.com/news/2025/02/nintendo-ds-emulator-drastic-pulled-from-google-play-store Pizza Boy's developer pulled the GB and GBA apps in Mar 2024, citing family. https://gigazine.net/gsc_news/en/20240310-nintendo-emulator-pizza-emulators-pulls-google-play APKMirror shows new Pizza Boy A builds in 2026, so it may be back (unconfirmed). https://www.apkmirror.com/?p=14610369
- **Apple pulled iGBA as a copied app (guidelines 4.3 and 5.2)**, while Delta, a GBA emulator, stays on the App Store. Original code and branding matter. https://www.mactrast.com/2024/04/apple-explains-why-it-removed-game-boy-emulator-igba-from-app-store/ , https://en.wikipedia.org/wiki/Delta_(emulator)
- **Japan:** Nintendo's patent counsel reportedly said emulators are technically legal there, but distributing games is not. The arrests so far were for selling modded Switch consoles under trademark law. https://www.techspot.com/community/topics/switch-modder-arrested-in-japan-as-nintendo-admits-emulation-is-technically-legal.290320/
- **Wording (my recommendation, not a sourced rule):** do not use "Nintendo", "Game Boy" or "Pokemon" in the app title or icon. Bundle no BIOS or ROMs. Use homebrew in screenshots. Say "play your own legally dumped games" and keep the HLE BIOS front and centre. Keep ROM hack and "free games" language out of the listing and LinkedIn posts, including localized ones.
