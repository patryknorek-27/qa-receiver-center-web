# Instrukcje dla Copilota

Wszystkie zasady są w `AGENTS.md` w katalogu głównym (kolejność czytania, budżet kontekstu, kontrole, dokumentacja). Przeczytaj go najpierw.

Skrót:

- Zacznij od `docs/PLAN-BIEZACY.md` i `docs/MAPA-KODU.md`; wpis w `docs/PULAPKI.md` czytaj tylko dla swojego zadania.
- Nie skanuj całego repozytorium ani katalogów artefaktów (`node_modules`, `dist`, `src-tauri/target`, `bin`, `obj`, `publish`). Duże pliki czytaj wycinkami po `rg -n`.
- Zachowuj granicę React -> Tauri/Rust -> Registry oraz kontrakty komend IPC i endpointów.
- Po zmianie uruchom najwęższą adekwatną kontrolę (`npm run lint`/`build`/`test:logic`, `cargo test --lib`, `dotnet build`). Skrypt `docs:check` nie istnieje.
- Nie nadpisuj niezacommitowanych zmian użytkownika; nie dopisuj haseł ani adresów kont.
