---
title: "Converter lógica do app web (data.js, strings.js) para Rust/WASM"
labels: [fase-2-logica, conversao-rust]
status: revertido
---

**Origem:** web/data.js (Place, Prefs, formatação, parsing de clima/cotações/TMDB), web/strings.js
**Destino:** crate `freemytv-logic` compilado para WebAssembly

Atenção: WASM exige Chromium 57+. webOS 3/4 e Tizen 3/4 não rodam; manter o JS como fallback nessas TVs. Os testes de `tests/web/` servem de especificação.

## Critérios de aceite
- [ ] Mesma saída do Python (teste de paridade em `tests/python/test_rust_parity.py`, quando houver arquivo gerado)
- [ ] Testes unitários no crate
- [ ] Documentado em `documentacao/rust.md` e `documentacao/alteracoes.md`
- [ ] Benchmark em `tests/bench/bench.py`, quando aplicável
