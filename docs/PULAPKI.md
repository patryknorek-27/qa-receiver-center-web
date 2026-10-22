# Pułapki projektu

## Usunięty odbiornik wraca do grupy firmware

**Objaw:** Po usunięciu odbiornik nadal jest na liście Alpha/Beta lub wraca po ponownym ładowaniu.

**Przyczyna:** Członkostwo grupy pochodzi z `qa-receivers.json`; usunięcie samego modelu z `firmware-configs.json` nie usuwa go z listy.

**Poprawne rozwiązanie:** Zapisz wykluczenie adresu w lokalnej konfiguracji firmware i filtruj listę przy każdym odczycie. Ponowne ręczne dodanie usuwa wykluczenie. Nie zmieniaj źródłowego `qa-receivers.json` przy operacji lokalnego usuwania.

**Powiązany obszar:** `FirmwareConfigMode`, `remove_firmware_config_receiver`, `firmware_receivers_for_group` w `src-tauri/src/lib.rs`

## Security Mode wyłączony

**Objaw:** Aktualizacja kończy się na kontroli logowania, choć odbiornik nie wymaga loginu.

**Przyczyna:** Starszy kod wymagał `UserSecurity=TRUE` i `SecMode=Anon` lub `Full`; zwracał błąd `Receiver security mode does not support a verified password login` dla wyłączonego Security Mode. Odbiornik `10.2.155.40` zwrócił bez logowania `UserSecurity=TRUE`, `SecMode=Off` (2026-10-06). Uruchomiony starszy build nadal będzie zwracał dawny błąd.

**Poprawne rozwiązanie:** Uruchom aktualny build. Rozpoznaj jawne `SecMode=Off` jako brak zabezpieczenia i pomiń próbę logowania oraz zmianę hasła; dla `Anon`/`Full` nadal weryfikuj hasło i zatrzymuj się po błędnej próbie.

**Powiązany obszar:** `src-tauri/src/receiver_password.rs`, `check_initial_firmware_password` w `src-tauri/src/lib.rs`

## Wspólny release Alpha/Beta

**Objaw:** Nowy release jest widoczny na serwerze, ale nie pojawia się na ekranie Alpha ani Beta.

**Przyczyna:** Nazwa jednego release może zawierać jednocześnie `Alpha` i `Beta`. Odrzucanie całego wpisu po wykryciu nazwy drugiego kanału usuwa go z obu ekranów.

**Poprawne rozwiązanie:** Udostępnij taki release w obu ekranach, a kanał wybieraj na poziomie nazw buildów z jego `index.html`. Katalog pobieraj ponownie przy wejściu w ekran oraz po ręcznym odświeżeniu.

**Nie rób:** Nie przypisuj całego release do jednego kanału i nie wybieraj automatycznie firmware tylko na podstawie nazwy release.

**Powiązany obszar:** `src/firmwareReleaseSelection.ts`, `src/App.tsx`, parser katalogu w `src-tauri/src/lib.rs`

## Dokumentacja lokalna i nieistniejąca komenda

**Objaw:** `npm run docs:check` kończy się błędem, a zmiany w części dokumentów nie pojawiają się w `git status`.

**Przyczyna:** W `package.json` nie ma skryptu `docs:check` (stara instrukcja Copilota go wymagała; poprawione 2026-10-22). `AGENTS.md`, `CLAUDE.md`, `.claude/`, `.ignore`, `.github/copilot-instructions.md`, cztery dokumenty z `docs/` i `scripts/Check-RepositoryDocs.ps1` są wykluczone lokalnie przez `.git/info/exclude`, więc nie trafią też na zdalne `backup`, dopóki nie zmieni się to świadomie.

**Poprawne rozwiązanie:** Używaj kontroli istniejących w `package.json`. Dokumenty aktualizuj przy zmianie opisanych faktów i sprawdzaj ich obecność przed użyciem w innym checkoutcie. Decyzję o ich współdzieleniu podejmij osobno.

**Nie rób:** Nie zakładaj, że lokalne zmiany tych plików trafią do commita ani że lokalny skrypt dokumentacyjny jest dostępny w repozytorium.

**Powiązany obszar:** `package.json`, `.git/info/exclude`, `docs/`, `scripts/Check-RepositoryDocs.ps1`

## Monolity UI i Tauri

**Objaw:** Mała zmiana w firmware, statusie odbiornika albo ustawieniach może prowadzić do czytania dużej części `src/App.tsx` lub `src-tauri/src/lib.rs`.

**Przyczyna:** Oba pliki urosły razem z funkcjami produktu i nadal utrzymują współdzielony stan oraz kolejność workflow.

**Poprawne rozwiązanie:** Zacznij od mapy, wyszukaj konkretny symbol i zmieniaj lokalny fragment. Wydzielaj domenę dopiero z testami zachowania i po potwierdzeniu granicy odpowiedzialności.

**Nie rób:** Nie dziel pliku automatycznie po liczbie linii i nie wprowadzaj globalnego store'a tylko dla estetyki.

**Powiązany obszar:** `src/App.tsx`, `src-tauri/src/lib.rs`

## Rust jako granica IPC

**Objaw:** Zmiana nazwy lub argumentów komendy może zepsuć UI bez błędu kompilacji TypeScript.

**Przyczyna:** Nazwy komend Tauri są kontraktem tekstowym między `invoke()` i Rustem.

**Poprawne rozwiązanie:** Przed zmianą wyszukaj nazwę komendy po obu stronach i uruchom `npm run build` oraz `cargo check`.

**Nie rób:** Nie zmieniaj komendy tylko dlatego, że wewnętrzna funkcja została przeniesiona.

**Powiązany obszar:** `src/App.tsx`, `src-tauri/src/lib.rs`

## Aktualizacje firmware są stanowe

**Objaw:** UI może pokazywać status zadania przez polling, gdy Rust nadal posiada rezerwację odbiornika.

**Przyczyna:** Stan orkiestratora żyje po stronie Rust, a UI utrzymuje własne widoki statusu.

**Poprawne rozwiązanie:** Sprawdzaj `firmware_orchestrator.rs`, lifecycle komendy i wszystkie miejsca odpytywania statusu. Nie usuwaj obsługi anulowania ani zwalniania rezerwacji bez testu.

**Nie rób:** Nie traktuj zamknięcia widoku lub wyzerowania stanu React jako zatrzymania zadania.

**Powiązany obszar:** `src-tauri/src/firmware_orchestrator.rs`, workflow firmware w `src/App.tsx`

## Registry jest osobną usługą

**Objaw:** Lokalny build aplikacji przechodzi, ale funkcja historii, IP albo ustawień nie działa na stanowisku.

**Przyczyna:** Te funkcje zależą od dostępności Registry na porcie 8787, jego migracji i konfiguracji serwera.

**Poprawne rozwiązanie:** Dla zmian backendowych uruchom build usługi, a przed publikacją `Run-IntegrationTests.ps1`; sprawdź też fallback lokalny, jeśli dotyczy.

**Nie rób:** Nie otwieraj SQLite bezpośrednio z aplikacji klienckiej i nie traktuj `dotnet build` jako testu sieci, migracji lub uprawnień.

**Powiązany obszar:** `services/receiver-registry/`, `src-tauri/src/lib.rs`

## Pliki wygenerowane i ciężkie artefakty

**Objaw:** Inwentaryzacja pokazuje duże pliki ikon, cache builda lub katalogi `bin`, `obj` i `target`.

**Przyczyna:** Są to zasoby binarne albo artefakty narzędzi, a nie dodatkowa logika aplikacji.

**Poprawne rozwiązanie:** Pomijaj artefakty podczas audytu; analizuj źródła, manifesty i skrypty.

**Nie rób:** Nie przenoś ani nie usuwaj wygenerowanych plików bez sprawdzenia ignorowania i procesu publikacji.

**Powiązany obszar:** `src/assets/`, `src-tauri/target/`, `services/receiver-registry/bin/`, `services/receiver-registry/obj/`

## Kaskada w stylach (src/styles)

**Objaw:** Zmiana reguły CSS nie daje efektu albo psuje inny ekran firmware.

**Przyczyna:** Style powstały jako warstwy poprawek (od 2026-10-22 w 20 plikach `src/styles/NN-*.css`, importowanych kolejno z `src/App.css`). Ta sama klasa (karty odbiorników, workspace firmware) bywa zdefiniowana w kilku plikach, a komentarze typu „Keep this section last” oznaczają bloki nadpisujące wcześniejsze.

**Poprawne rozwiązanie:** `rg -n "\.nazwa-klasy" src/styles`, edytuj ostatnie wystąpienie albo konsoliduj wszystkie razem. Nie zmieniaj kolejności `@import` w `src/App.css`; weryfikuj zmianę wizualnie lub porównaniem zbudowanego CSS.

**Nie rób:** Nie dodawaj kolejnej nadpisującej warstwy na końcu bez sprawdzenia istniejących bloków i nie przenoś reguł między plikami bez sprawdzenia kolejności.

**Powiązany obszar:** `src/App.css`, `src/styles/`, `src/index.css`

## Stałe operacyjne i brama operatora w kodzie

**Objaw:** W `src-tauri/src/lib.rs` (ok. linii 38-40) są jawne stałe: nazwa i hasło „administratora aplikacji” oraz tymczasowe hasło firmware. `LOGIN_ENABLED=false` w `App.tsx` wyłącza ekran logowania; `tauri.conf.json` ma `csp: null`.

**Przyczyna:** Wewnętrzne narzędzie QA; hasło służy jako bramka potwierdzenia operacji (`authenticate_operator`, `validate_application_password`), a hasło tymczasowe jest używane na odbiornikach w trakcie aktualizacji. To nie jest ochrona przed atakującym.

**Poprawne rozwiązanie:** Traktuj te wartości jako poufne: nie kopiuj ich do vaulta, dokumentów, logów, commit message ani publicznych repo. Przed wysłaniem kodu na zewnętrzne konto lub repo zdecyduj świadomie o ich ujawnieniu. Ewentualne przeniesienie do konfiguracji to osobny etap.

**Powiązany obszar:** `src-tauri/src/lib.rs`, `src-tauri/src/receiver_password.rs`, `src/App.tsx`

## Komendy IPC bez wywołań w UI

**Objaw:** Rejestracja w `generate_handler!` ma 5 komend bez `invoke(...)` w `src/`: `load_receivers_config`, `validate_firmware_plan`, `dry_run_single_firmware_update`, `start_single_local_firmware_update` (UI używa wariantu `..._job`) oraz `cancel_single_firmware_update_job` (UI anuluje przez `stop_firmware_update_job`).

**Przyczyna:** Pozostałości po wcześniejszych wersjach workflow lub API przygotowane na przyszłość.

**Poprawne rozwiązanie:** Nie usuwaj bez decyzji; to kontrakt IPC. Przed wycofaniem wyszukaj nazwę po obu stronach i uruchom `cargo check` oraz `npm.cmd run build`. Brak anulowania pojedynczego joba z UI może być luką funkcjonalną, nie tylko martwym kodem.

**Powiązany obszar:** `src-tauri/src/lib.rs` (rejestracja ok. 7150-7216), `src/App.tsx`

## Podglądy HTML w tests/

**Objaw:** `tests/*-preview.html` wyglądają jak testy, ale `npm.cmd run test:logic` ich nie uruchamia (tylko `tests/*.test.ts`).

**Przyczyna:** To ręczne podglądy układu ładujące `/src/App.css` (z jego importami) przez serwer Vite.

**Poprawne rozwiązanie:** Używaj ich tylko do ręcznego podglądu przy `npm.cmd run dev`; nie wliczaj do wyników kontroli.

**Powiązany obszar:** `tests/group-update-preview.html`, `tests/password-recovery-preview.html`

## Rozmiar katalogów roboczych

**Objaw:** `src-tauri/target/` ma ok. 11 GB (debug 9,6 GB, release 1,4 GB); `.dotnet-home/` ok. 370 MB; `node_modules/` ok. 300 MB.

**Przyczyna:** Cache buildów Rust, .NET i npm; wszystko jest ignorowane przez Git.

**Poprawne rozwiązanie:** Nie czytaj tych katalogów. Zwolnienie miejsca: `cargo clean` w `src-tauri` (kolejny build trwa kilka minut), `services/receiver-registry/publish/` zawiera paczkę 0.7.0 do wdrożenia, więc nie usuwaj jej bez potrzeby.

**Powiązany obszar:** `src-tauri/target/`, `.dotnet-home/`, `node_modules/`

## Celowo pozostawione duże pliki

- Style w `src/styles/` nadal zawierają nakładające się warstwy (patrz „Kaskada w stylach”); fizyczny podział już zrobiony, ale konsolidacja reguł wymaga porównania wizualnego i jest poza zwykłą poprawką.
- `src/App.tsx` pozostaje głównym ekranem i koordynatorem wielu powiązanych workflow. Podział na wiele komponentów bez testów UI zwiększyłby liczbę plików potrzebnych do śledzenia współdzielonego stanu.
- `src-tauri/src/lib.rs` pozostaje granicą komend IPC. Wydzielanie osobnych modułów jest sensowne dopiero po dodaniu testów kontraktów i wyraźnych granic dla komunikacji, plików oraz orkiestracji.
- `services/receiver-registry/Data/RegistryDatabase.cs` pozostaje skupiony wokół spójnego dostępu do SQLite, migracji i transakcji; mechaniczny podział utrudniłby śledzenie integralności danych.

Pełny podział tych plików jest **RECOMMENDED BUT NOT APPLIED**: wymaga dodatkowych testów UI/IPC i osobnych etapów migracji.
