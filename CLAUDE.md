# Claude Code

@AGENTS.md
@docs/PLAN-BIEZACY.md

- Odpowiadaj po polsku, krótko.
- Do szerokich przeszukań używaj subagenta Explore; nie czytaj plików z listy „Nie czytaj” w `AGENTS.md`.
- Ustawienia lokalne ograniczające kontekst: `.claude/settings.local.json` (deny dla artefaktów, wyłączone wtyczki niezwiązane z projektem). Wtyczki LSP (TypeScript, Rust, C#) są włączone: do nawigacji po symbolach w dużych plikach używaj narzędzia LSP zamiast czytania pliku.
- Drugi przegląd przed wydaniem: `/codex:review --base main --background` (nie do rutynowych zmian).
