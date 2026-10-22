# Zasady pracy nad QA Receiver Center Web

Plik wspólny dla Claude Code, Codex i Copilota (`CLAUDE.md` i `.github/copilot-instructions.md` tylko na niego wskazują). Projekt przenosi aplikację desktopową (Tauri) na wersję webową; cel i etapy są w `docs/PLAN-BIEZACY.md`. Pracuj etapami; stare narzędzia Python i PowerShell oraz desktopowe `../qa-receiver-center` to źródło zachowań, nie wzór architektury.

## Początek zadania (kolejność czytania)

1. `git status --short` – zachowaj istniejące zmiany użytkownika.
2. `docs/PLAN-BIEZACY.md` (aktywny etap). `docs/AI-SESSION-HANDOFF.md` tylko po dłuższej przerwie.
3. `docs/MAPA-KODU.md`: tabela „Zadanie -> gdzie zacząć” i „Przewodnik po dużych plikach”. `docs/PULAPKI.md`: tylko wpis dotyczący zadania.
4. Symbol i jego użycia: `rg -n`, potem odczyt wycinka (`offset`/`limit`). Pełny odczyt dużego pliku tylko gdy nie ma innej drogi.
5. Przy UI wskaż właściciela stanu i przepływ React -> Tauri IPC -> Registry. Build UI nie weryfikuje usługi ani działania na odbiorniku.

## Budżet kontekstu (limity)

- **Nie czytaj i nie skanuj:** `node_modules/`, `dist/`, `src-tauri/target/`, `src-tauri/gen/`, `src-tauri/icons/`, `services/**/{bin,obj,publish}/`, `.dotnet-home/`, `.design-reference/`, `package-lock.json`, `Cargo.lock`, `src/assets/`, `docs/*.png`, `public/*.mp4`.
- **Duże pliki:** `src-tauri/src/lib.rs` (~8,1 tys.), `src/App.tsx` (~6,8 tys.), `RegistryDatabase.cs` (~2,1 tys.). Tylko `rg -n` + wycinek wg mapy. Style są w `src/styles/NN-*.css` (20 plików po ok. 200-1000 linii, importowanych przez `src/App.css`); późniejsze pliki nadpisują wcześniejsze (patrz `PULAPKI.md`).
- Wyniki komend ograniczaj: `| tail -n 20`, `cargo test --lib -q`, `npm.cmd run build --silent`. Nie wklejaj całych logów do odpowiedzi.
- Szerokie przeszukiwanie (wiele katalogów) deleguj do subagenta/Explore; do głównej rozmowy wraca tylko wniosek.
- Nowe, niepowiązane zadanie = nowa sesja (`/clear`); długą sesję kompaktuj przy ~60% kontekstu.
- Nie powtarzaj odczytu mapy ani kontroli bez nowego ryzyka.

## Zmiany i kontrole

- Zachowuj kontrakty komend Tauri, endpointów Registry i migracji danych, chyba że zadanie wymaga zmiany. Przed zmianą nazwy komendy wyszukaj ją po obu stronach (`invoke('nazwa'` i `fn nazwa`).
- Nie dziel `src/App.tsx`, `src-tauri/src/lib.rs` ani `RegistryDatabase.cs` tylko ze względu na rozmiar.
- Kontrole dobieraj do obszaru: `npm.cmd run lint`, `npm.cmd run build`, `npm.cmd run test:logic`, `cargo test --lib` / `cargo check` w `src-tauri`, `dotnet build services/receiver-registry/QAReceiverRegistry.csproj --no-restore`. Integracja Registry (`Run-IntegrationTests.ps1`) tylko przy zmianach usługi, zwłaszcza przed publikacją.
- Nie uruchamiaj rutynowo aplikacji przez sterowanie komputerem ani nie buduj instalatora. Jeśli wygląd lub zachowanie na żywo nie było sprawdzone, napisz to jednym zdaniem.
- Nie wpisuj do kodu, dokumentów ani vaulta haseł, tokenów i adresów kont. W `lib.rs` istnieją stałe operacyjne (patrz `PULAPKI.md`); nie kopiuj ich dalej.

## Dokumentacja po zmianach

- `docs/PLAN-BIEZACY.md` – po zmianie zakresu, decyzji, statusu lub następnego kroku.
- `docs/MAPA-KODU.md` – po zmianie entry pointu, domeny, przepływu, komendy kontroli lub dużej zmianie rozkładu monolitów.
- `docs/PULAPKI.md` – gdy odkryjesz trwałą pułapkę albo usuniesz opisaną.
- `docs/AI-SESSION-HANDOFF.md` – gdy zmieni się szybki start.
- Bez rutynowej historii każdej edycji; dokumenty krótkie i bez powielania. Po zmianie dokumentów: `scripts/Check-RepositoryDocs.ps1` (lokalny).

## Drugi mózg (Obsidian)

Vault: `C:\Users\pnorek\Documents\PATRYK_QA` (zasady: `AGENTS.md` w vaulcie). Pamięć operacyjna między sesjami i projektami: `wiki/hot.md` (stan i następny krok), `wiki/projects/qa-receiver-center-web/`, `wiki/log.md`. Kod i pułapki techniczne zostają w `docs/` repo; vault trzyma skrót, decyzje i linki, bez kopiowania. Na koniec etapu: jedna linia w `wiki/log.md` i aktualizacja `Next Action` w `wiki/hot.md`.

## Repozytoria

`origin` to prywatne GitHub `qa-receiver-center-web`; push tylko na prośbę użytkownika. `desktop` to lokalne repo desktopowe: poprawki z niego przenoś przez `git fetch desktop` + `git cherry-pick`, nigdy nie pushuj do `desktop`. Nie wpisuj do kodu ani dokumentów sekretów; konfiguracja serwera (konta, certyfikat) żyje w `config\` na serwerze, poza repo.

## Komunikacja

Pracuj na jednym etapie. Aktualizacje krótkie: ustalenie, zmiana, wynik kontroli, otwarta kwestia.
