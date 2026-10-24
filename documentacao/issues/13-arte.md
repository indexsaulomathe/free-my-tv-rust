---
title: "Converter geradores de arte e símbolos"
labels: [fase-1-ferramentas, conversao-rust]
status: a-fazer
---

**Origem:** tools/make_art.py, tools/make_symbols.py (Pillow)
**Destino:** `freemytv make-art`, `freemytv make-symbols`

Baixa prioridade: só rodam quando a arte muda. Usaria os crates `image`/`ab_glyph`.

## Critérios de aceite
- [ ] Mesma saída do Python (teste de paridade em `tests/python/test_rust_parity.py`, quando houver arquivo gerado)
- [ ] Testes unitários no crate
- [ ] Documentado em `documentacao/rust.md` e `documentacao/alteracoes.md`
- [ ] Benchmark em `tests/bench/bench.py`, quando aplicável
